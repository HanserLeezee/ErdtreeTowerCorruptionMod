//! TOWER_PATCH (IME): Chinese and other IME text in imgui inputs.
//!
//! The game window normally has no IME context, so keys reach imgui as plain
//! ASCII. While imgui wants text, the window gets the default IME context and
//! the original association is restored afterwards (no IME during gameplay).
//! Committed IME text is read from `WM_IME_COMPOSITION` on the window thread and
//! fed to imgui as characters; the result flags are stripped before the message
//! continues so the default procedure does not produce a second copy through
//! `WM_IME_CHAR`/`WM_CHAR` (the game's own procedure may not forward IME
//! messages at all, which is why relying on `WM_CHAR` alone dropped the text).
//! The composition and candidate windows follow the active text field.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::UI::Input::Ime::{
    ImmAssociateContext, ImmAssociateContextEx, ImmGetCompositionStringW, ImmGetContext,
    ImmReleaseContext, ImmSetCandidateWindow, ImmSetCompositionWindow, CANDIDATEFORM,
    CFS_CANDIDATEPOS, CFS_POINT, COMPOSITIONFORM, GCS_RESULTSTR, HIMC, IACE_DEFAULT,
};
use windows::Win32::UI::WindowsAndMessaging::{WM_IME_CHAR, WM_IME_COMPOSITION, WM_IME_STARTCOMPOSITION};

/// The IME is currently switched on for imgui text input
static ACTIVE: AtomicBool = AtomicBool::new(false);
/// The window had no context and we associated the default one (restore to none afterwards)
static OWNED: AtomicBool = AtomicBool::new(false);
/// Text field position in client coordinates (x and y as f32 bits), `u64::MAX` when unknown
static POSITION: AtomicU64 = AtomicU64::new(u64::MAX);
/// High surrogate waiting for its pair in `WM_IME_CHAR`
static PENDING_HIGH: AtomicU32 = AtomicU32::new(0);

/// Result-string flags of `WM_IME_COMPOSITION` (read, read clause, result, result clause)
const RESULT_FLAGS: isize = 0x1e00;

/// Where the active text field is (bottom-left corner, client coordinates); the IME windows open there
pub fn set_position(x: f32, y: f32) {
    POSITION.store((u64::from(x.to_bits()) << 32) | u64::from(y.to_bits()), Ordering::Relaxed);
}

fn position() -> Option<POINT> {
    let packed = POSITION.load(Ordering::Relaxed);
    (packed != u64::MAX).then(|| POINT {
        x: f32::from_bits((packed >> 32) as u32) as i32,
        y: f32::from_bits(packed as u32) as i32,
    })
}

/// What the window procedure should do with the message after `before`
pub(crate) enum Ime {
    /// Pass the message on (possibly with a changed `lparam`)
    Forward(LPARAM),
    /// Fully handled here; return 0 without calling the game's procedure
    Handled,
}

/// Runs on the window thread for every message, before it is queued for imgui or passed to the game.
pub(crate) unsafe fn before(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, want_text: bool, push: &dyn Fn(char)) -> Ime {
    unsafe { sync(hwnd, want_text) };
    if !want_text {
        return Ime::Forward(lparam);
    }
    match msg {
        WM_IME_STARTCOMPOSITION => {
            unsafe { place(hwnd) };
            Ime::Forward(lparam)
        },
        WM_IME_COMPOSITION => {
            unsafe { place(hwnd) };
            if lparam.0 & GCS_RESULTSTR.0 as isize == 0 {
                return Ime::Forward(lparam);
            }
            for c in unsafe { result(hwnd) } {
                push(c);
            }
            Ime::Forward(LPARAM(lparam.0 & !RESULT_FLAGS))
        },
        // Some IMEs deliver committed text only as WM_IME_CHAR: take it here so the default procedure does not repeat it as WM_CHAR
        WM_IME_CHAR => {
            unsafe { ime_char(wparam.0 as u32, push) };
            Ime::Handled
        },
        _ => Ime::Forward(lparam),
    }
}

/// `WM_IME_CHAR` carries one UTF-16 unit; characters outside the BMP arrive as two messages
unsafe fn ime_char(unit: u32, push: &dyn Fn(char)) {
    let unit = unit & 0xffff;
    match unit {
        0xd800..=0xdbff => PENDING_HIGH.store(unit, Ordering::Relaxed),
        0xdc00..=0xdfff => {
            let high = PENDING_HIGH.swap(0, Ordering::Relaxed);
            if high != 0 {
                if let Some(c) = char::from_u32(0x10000 + ((high - 0xd800) << 10) + (unit - 0xdc00)) {
                    push(c);
                }
            }
        },
        _ => {
            PENDING_HIGH.store(0, Ordering::Relaxed);
            if let Some(c) = char::from_u32(unit) {
                push(c);
            }
        },
    }
}

unsafe fn sync(hwnd: HWND, want_text: bool) {
    if want_text == ACTIVE.load(Ordering::Relaxed) {
        return;
    }
    ACTIVE.store(want_text, Ordering::Relaxed);
    if want_text {
        let himc = unsafe { ImmGetContext(hwnd) };
        if himc.is_invalid() {
            let _ = unsafe { ImmAssociateContextEx(hwnd, HIMC::default(), IACE_DEFAULT) };
            OWNED.store(true, Ordering::Relaxed);
        } else {
            let _ = unsafe { ImmReleaseContext(hwnd, himc) };
            OWNED.store(false, Ordering::Relaxed);
        }
    } else if OWNED.swap(false, Ordering::Relaxed) {
        // Back to no IME so gameplay keys are not swallowed by an open composition
        let _ = unsafe { ImmAssociateContext(hwnd, HIMC::default()) };
    }
}

unsafe fn place(hwnd: HWND) {
    let Some(point) = position() else { return };
    let himc = unsafe { ImmGetContext(hwnd) };
    if himc.is_invalid() {
        return;
    }
    let composition = COMPOSITIONFORM { dwStyle: CFS_POINT, ptCurrentPos: point, rcArea: RECT::default() };
    let candidate = CANDIDATEFORM { dwIndex: 0, dwStyle: CFS_CANDIDATEPOS, ptCurrentPos: point, rcArea: RECT::default() };
    unsafe {
        let _ = ImmSetCompositionWindow(himc, &composition);
        let _ = ImmSetCandidateWindow(himc, &candidate);
        let _ = ImmReleaseContext(hwnd, himc);
    }
}

unsafe fn result(hwnd: HWND) -> Vec<char> {
    let himc = unsafe { ImmGetContext(hwnd) };
    if himc.is_invalid() {
        return Vec::new();
    }
    let bytes = unsafe { ImmGetCompositionStringW(himc, GCS_RESULTSTR, None, 0) };
    let mut text = Vec::new();
    if bytes > 0 {
        let mut units = vec![0u16; bytes as usize / 2];
        let read = unsafe {
            ImmGetCompositionStringW(himc, GCS_RESULTSTR, Some(units.as_mut_ptr().cast()), bytes as u32)
        };
        if read > 0 {
            units.truncate(read as usize / 2);
            text = char::decode_utf16(units).filter_map(|c| c.ok()).collect();
        }
    }
    let _ = unsafe { ImmReleaseContext(hwnd, himc) };
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surrogate_pairs_from_ime_char_become_one_character() {
        let seen = std::cell::RefCell::new(Vec::new());
        let push = |c: char| seen.borrow_mut().push(c);
        unsafe {
            ime_char(0x4e2d, &push); // 中
            ime_char(0xd83d, &push); // 😀 high
            ime_char(0xde00, &push); // 😀 low
            ime_char(0xde00, &push); // stray low surrogate is dropped
        }
        assert_eq!(*seen.borrow(), vec!['中', '😀']);
    }
}
