//! [HansFix] Optional community patches, switched on by marker files next to this DLL.
//! Every HansFix patch ships the same DLL plus only its own marker, so patches never depend on
//! each other and can be combined. Without any marker the DLL behaves exactly like the official build.
//!   hansfix-wander.enabled               游龙模式（改变联机协议：队伍里所有人都要装同一补丁）
//!   hansfix-better-map-merchant.enabled  N 键只读地图、联机画线不随换层清空、每人独立商店（与官方版联机兼容）
//! The map key can be rebound with `key.map=<键>` in tower-options.ini.
use std::{path::{Path,PathBuf},sync::atomic::{AtomicBool,Ordering}};

pub const WANDER_MARKER:&str="hansfix-wander.enabled";
pub const MERCHANT_MARKER:&str="hansfix-better-map-merchant.enabled";
/// Protocol offset used when the wander patch is on, so mixed parties fail the handshake loudly.
pub const WANDER_PROTOCOL_OFFSET:u32=1000;

static WANDER:AtomicBool=AtomicBool::new(false);
static MERCHANT:AtomicBool=AtomicBool::new(false);
static STALE:std::sync::OnceLock<String>=std::sync::OnceLock::new();
/// Size of scene-assets/regulation.bin of the official release this DLL was built against (v98).
/// A different size means the launcher updated the mod after this patch was applied: the patched
/// DLL would no longer match the game data, so every HansFix feature stays off and a warning is shown.
pub const BASE_RELEASE:&str="v98";
pub const BASE_REGULATION_LEN:u64=3438688;
static MAP_KEY:std::sync::OnceLock<&'static str>=std::sync::OnceLock::new();

pub fn wander()->bool {WANDER.load(Ordering::Relaxed)}
pub fn merchant()->bool {MERCHANT.load(Ordering::Relaxed)}

fn map_key_from(text:&str)->Option<&'static str> {
    text.lines().filter_map(|l|l.trim().trim_start_matches('\u{feff}').split_once('='))
        .filter(|(k,_)|k.trim().eq_ignore_ascii_case("key.map"))
        .filter_map(|(_,v)|erdtree_tower::options::KEYS.iter().copied().find(|k|k.eq_ignore_ascii_case(v.trim()))).last()
}
pub fn map_label()->&'static str {MAP_KEY.get().copied().unwrap_or("N")}
pub fn map_key()->hudhook::imgui::Key {crate::options::key_named(map_label()).unwrap_or(hudhook::imgui::Key::N)}

pub fn set(wander_on:bool,merchant_on:bool) {
    WANDER.store(wander_on,Ordering::Relaxed);MERCHANT.store(merchant_on,Ordering::Relaxed);
    erdtree_tower::party_handshake::set_extension(if wander_on {WANDER_PROTOCOL_OFFSET} else {0});
}

/// Warning text when this patched DLL sits on top of a different official release.
pub fn stale()->Option<&'static str> {STALE.get().map(String::as_str)}
pub fn stale_in(dir:&Path)->Option<u64> {
    std::fs::metadata(dir.join("scene-assets").join("regulation.bin")).ok().map(|m|m.len()).filter(|&len|len!=BASE_REGULATION_LEN)
}
pub fn detect(dir:&Path)->(bool,bool) {(dir.join(WANDER_MARKER).is_file(),dir.join(MERCHANT_MARKER).is_file())}

/// DllMain: read markers next to the DLL. Must run before anything talks to the party.
pub fn init(dll:Option<PathBuf>) {
    let dir=dll.as_deref().and_then(Path::parent).map(PathBuf::from);
    let (mut wander_on,mut merchant_on)=dir.as_deref().map(detect).unwrap_or_default();
    if let Some(len)=dir.as_deref().and_then(stale_in) {
        crate::log(format!("HANSFIX_STALE base={BASE_RELEASE} regulation_len={len} expected={BASE_REGULATION_LEN}"));
        let _=STALE.set(format!("HansFix 补丁是为爬塔 {BASE_RELEASE} 制作的，与当前模组版本不匹配：请在启动器「设置 → 更新 → 校验并修复文件」恢复官方文件，再安装对应新版本的补丁。"));
        wander_on=false;merchant_on=false;
    }
    set(wander_on,merchant_on);
    if let Some(key)=dir.as_deref().and_then(|d|std::fs::read_to_string(d.join(erdtree_tower::options::FILE)).ok()).and_then(|t|map_key_from(&t)) {let _=MAP_KEY.set(key);}
    crate::log(format!("HANSFIX wander={wander_on} better_map_merchant={merchant_on} map_key={} protocol={} link={}",map_label(),
        erdtree_tower::party_handshake::protocol(),erdtree_tower::party_link::wire_version()));
}

#[cfg(test)]
mod tests {
    #[test]
    fn markers_switch_features_independently_and_only_wander_changes_the_protocol() {
        let dir=std::env::temp_dir().join(format!("hansfix-test-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&dir);std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(super::detect(&dir),(false,false));
        std::fs::write(dir.join(super::MERCHANT_MARKER),"").unwrap();
        assert_eq!(super::detect(&dir),(false,true));
        super::set(false,true);assert_eq!(erdtree_tower::party_handshake::protocol(),erdtree_tower::party_handshake::VERSION);
        std::fs::write(dir.join(super::WANDER_MARKER),"").unwrap();
        assert_eq!(super::detect(&dir),(true,true));
        super::set(true,false);assert_eq!(erdtree_tower::party_handshake::protocol(),erdtree_tower::party_handshake::VERSION+1000);
        assert_eq!(erdtree_tower::party_link::wire_version(),erdtree_tower::party_link::VERSION+1000);
        super::set(false,false);assert_eq!(erdtree_tower::party_handshake::protocol(),erdtree_tower::party_handshake::VERSION);
        assert_eq!(super::stale_in(&dir),None,"no scene-assets: nothing to compare");
        std::fs::create_dir_all(dir.join("scene-assets")).unwrap();
        std::fs::write(dir.join("scene-assets/regulation.bin"),[0u8;16]).unwrap();
        assert_eq!(super::stale_in(&dir),Some(16));
        let _=std::fs::remove_dir_all(&dir);
        assert_eq!(super::map_key_from("key.map = m\nkey.build=C"),Some("M"));
        assert_eq!(super::map_key_from("key.map=bogus"),None);
    }
}
