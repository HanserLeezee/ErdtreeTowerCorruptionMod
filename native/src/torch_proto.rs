//! [HansFix] 火把（游戏内部分）：
//! - 火把位：把本局每把火把写进它占用的预留武器行（外观复制自原版火把，补正与 3 个词条按这把火把改写）；
//! - 文字：火把位、法术战技（战技名/说明）与法术战灰的名字装进游戏原本为空的 DLC2 文本表；
//! - 释放：左手举起（防御 / L1）装有法术战灰的火把满 10 秒，直接放出这个法术（范围判定或增益效果），
//!   同时播放法术原版的释放光效与音效，不做动作。
use std::sync::Mutex;
use std::time::{Duration, Instant};
use eldenring::cs::{CSGaitemImp, CSWepGaitemIns, ChrAsmArmStyle, ChrIns, ChrInsExt, EquipParamWeapon, PlayerGameData, SoloParamRepository, SpEffectParam};
use erdtree_tower::torches::{self, Torch};
use fromsoftware_shared::{FromStatic, Superclass};

const HOLD: Duration = Duration::from_secs(10);
/// 调试：举起 1 秒就释放
static FAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn fast() -> bool { FAST.load(std::sync::atomic::Ordering::Relaxed) }
pub fn set_fast(on: bool) { FAST.store(on, std::sync::atomic::Ordering::Relaxed); }
// DLC2 的空文本表：武器名/武器说明，战灰名/战灰说明，战技名/战技说明
const WEAPON_NAME: u32 = 410; const WEAPON_CAPTION: u32 = 412;
const GEM_NAME: u32 = 422; const GEM_CAPTION: u32 = 424;
const ARTS_NAME: u32 = 431; const ARTS_CAPTION: u32 = 432;

struct State {
    held_since: Option<Instant>,
    last: Option<(i32, i32, bool, bool)>,
    applied: Option<Vec<Torch>>,
    spells_text: bool,
    fired: bool,
}
static STATE: Mutex<State> = Mutex::new(State { held_since: None, last: None, applied: None, spells_text: false, fired: false });

/// 发一把火把（武器 = 它的火把位，战灰 = 它的法术）
/// all：本局全部火把（含这一把）。先把名字装好再发放，否则拿到时的提示框会显示 ?WeaponName?
pub fn give(torch: &Torch, all: &[Torch]) -> Result<(), String> {
    apply_torches(all, true);
    if let Ok(mut state) = STATE.lock() { state.applied = Some(all.to_vec()); }
    super::random_ashes::give(&[(torch.weapon_id() as i32, 1, torch.gem() as i32)])?;
    super::log(format!("HANSFIX_TORCH granted slot={} weapon={} base={} spell={} scaling={:?} affixes={:?}", torch.slot, torch.weapon_id(), torch.base, torch.spell, torch.scaling, torch.affixes));
    Ok(())
}

fn base_name(base: u32) -> String {
    super::hansfix_text::lookup(11, base).or_else(|| super::hansfix_text::lookup(310, base)).unwrap_or_else(|| "火把".into())
}
/// 把火把写进它的火把位（参数与文字）。all=true 时按整局的火把重写文字表
fn apply_torches(list: &[Torch], all: bool) {
    let Ok(params) = (unsafe { SoloParamRepository::instance_mut() }) else { return };
    for torch in list {
        let Some(base) = params.get::<EquipParamWeapon>(torch.base).map(|r| r as *const _) else { continue };
        let Some(slot) = params.get_mut::<EquipParamWeapon>(torch.weapon_id()) else { continue };
        unsafe { std::ptr::copy_nonoverlapping(base, slot as *mut _, 1) };
        let v = torch.scaling_values();
        slot.set_correct_strength(v[0]); slot.set_correct_agility(v[1]); slot.set_correct_magic(v[2]); slot.set_correct_faith(v[3]); slot.set_correct_luck(v[4]);
        let a = |i: usize| torch.affixes.get(i).map_or(-1, |e| *e as i32);
        slot.set_resident_sp_effect_id(a(0)); slot.set_resident_sp_effect_id1(a(1)); slot.set_resident_sp_effect_id2(a(2));
        slot.set_gem_mount_type(2);
    }
    if all {
        let names = list.iter().map(|t| (t.weapon_id(), format!("{}{}", t.prefix(), base_name(t.base)))).collect();
        let captions = list.iter().map(|t| {
            // 说明只放模组的词条；补正看补正栏，战技看战技栏
            (t.weapon_id(), format!("{}\n\n可以用来在铁匠处与其他武器合并以提升补正。", t.affix_lines().join("\n")))
        }).collect();
        let ok = super::hansfix_text::install_table(WEAPON_NAME, names) & super::hansfix_text::install_table(WEAPON_CAPTION, captions);
        super::log(format!("HANSFIX_TORCH slots applied torches={} texts={ok}", list.len()));
    }
}
fn install_spell_texts() -> bool {
    let spells = &*torches::SPELLS;
    let caption = |s: &torches::Spell| format!("举起10秒后，可以释放战技：{}", s.name);
    let ok = [
        super::hansfix_text::install_table(ARTS_NAME, spells.iter().map(|s| (torches::ART_FIRST + u32::from(s.index), s.name.to_owned())).collect()),
        super::hansfix_text::install_table(ARTS_CAPTION, spells.iter().map(|s| (torches::ART_FIRST + u32::from(s.index), caption(s))).collect()),
        super::hansfix_text::install_table(GEM_NAME, spells.iter().map(|s| (torches::GEM_FIRST + u32::from(s.index), format!("火把战技：{}", s.name))).collect()),
        super::hansfix_text::install_table(GEM_CAPTION, spells.iter().map(|s| (torches::GEM_FIRST + u32::from(s.index), caption(s))).collect()),
    ];
    super::log(format!("HANSFIX_TORCH spell texts installed={ok:?}"));
    ok.iter().all(|b| *b)
}

fn left_weapon(data: &PlayerGameData) -> Option<(i32, i32)> {
    let asm = &data.equipment.chr_asm;
    if asm.equipment.arm_style == ChrAsmArmStyle::RightBothHands { return None; }
    let slot = asm.equipment.active_left_weapon_slot() as usize;
    let id = asm.equipment_param_ids.get(slot).copied().filter(|id| *id > 0)?;
    let manager = unsafe { CSGaitemImp::instance() }.ok()?;
    let handle = asm.gaitem_handles[slot];
    if handle.index() as usize >= manager.gaitems.len() { return Some((id, -1)); }
    let weapon = manager.gaitem_ins_by_handle(&handle).and_then(|w| w.as_subclass::<CSWepGaitemIns>());
    let gem = weapon.map_or(-1, |weapon| {
        let gem = weapon.gem_slot_table.gem_slots[0].gaitem_handle;
        if gem.is_indexed() && (gem.index() as usize) < manager.gaitems.len() {
            manager.gaitem_ins_by_handle(&gem).and_then(|g| g.item_id.param_id()).map_or(-1, |id| id as i32)
        } else { -1 }
    });
    Some((id, gem))
}
fn is_torch(weapon: i32) -> bool {
    let id = weapon as u32 / 100 * 100;
    weapon >= 0 && (torches::slot_of(weapon as u32).is_some() || torches::BASES.contains(&id))
}

/// 每帧
pub fn sync(chr: &mut ChrIns, data: &PlayerGameData, run: Option<&erdtree_tower::Run>) {
    let Ok(mut state) = STATE.lock() else { return };
    if !state.spells_text { state.spells_text = install_spell_texts(); }
    let list = run.map(|r| r.torches.clone()).unwrap_or_default();
    if state.applied.as_ref() != Some(&list) { apply_torches(&list, true); state.applied = Some(list); }
    let (weapon, gem) = left_weapon(data).unwrap_or((-1, -1));
    let spell = (gem >= 0).then(|| torches::spell_of_gem(gem as u32)).flatten();
    let ours = is_torch(weapon) && spell.is_some();
    let requests = &chr.modules.action_request.action_requests;
    let (guard, l1) = (requests.guard(), requests.l1());
    if state.last != Some((weapon, gem, guard, l1)) {
        if is_torch(weapon) { super::log(format!("HANSFIX_TORCH left={weapon} gem={gem} ours={ours} guard={guard} l1={l1}")); }
        state.last = Some((weapon, gem, guard, l1));
    }
    let Some(spell) = spell.filter(|_| ours && (guard || l1)) else { state.held_since = None; state.fired = false; return };
    // 一次举起只释放一次：放下后再举起才重新计时（否则一直举着会每 10 秒叠一次特效和音效）
    if state.fired { return; }
    let since = *state.held_since.get_or_insert_with(Instant::now);
    if since.elapsed() < if fast() { Duration::from_secs(1) } else { HOLD } { return; }
    state.fired = true;
    cast(chr, spell);
}

fn cast(chr: &mut ChrIns, spell: &torches::Spell) {
    let Ok(params) = (unsafe { SoloParamRepository::instance() }) else { return };
    let flash = (torches::FLASH_FIRST + u32::from(spell.index)) as i32;
    if params.get::<SpEffectParam>(flash as u32).is_some() { chr.apply_speffect(flash, false); }
    let result = if spell.bullet { spawn_bullet(chr, spell.reference as i32, spell.magic as i32) }
        else if params.get::<SpEffectParam>(spell.reference).is_some() { chr.apply_speffect(spell.reference as i32, false); Ok(()) }
        else { Err(format!("effect {} missing", spell.reference)) };
    match result {
        Ok(()) => super::log(format!("HANSFIX_TORCH cast {} ({}) after 10s raised", spell.name, spell.magic)),
        Err(e) => super::log(format!("HANSFIX_TORCH cast {} failed: {e}", spell.name)),
    }
}

#[repr(C, align(16))]
struct SpawnData([u8; 0x110]);
type Spawn = unsafe extern "C" fn(*mut std::ffi::c_void, *mut i32, *const SpawnData, *mut i32);
/// 以玩家为原点、面朝方向发射一个法术判定（不播动作）。布局与 star_stream 的 SpawnData / fromsoftware-rs 的 BulletSpawnData 一致
fn spawn_bullet(chr: &ChrIns, bullet: i32, magic: i32) -> Result<(), String> {
    use eldenring::cs::{Bullet, CSBulletManager};
    let params = unsafe { SoloParamRepository::instance() }.map_err(|_| "参数未就绪")?;
    if params.get::<Bullet>(bullet as u32).is_none() { return Err(format!("bullet {bullet} missing")); }
    if size_of::<eldenring::cs::BulletSpawnData>() != size_of::<SpawnData>() { return Err("spawn layout mismatch".into()); }
    // 星流钩子已经把这个函数开头改成了跳转：有跳板就走跳板，没有再按特征码校验原函数
    let function = match super::star_stream::original_spawn() {Some(f) => f, None => super::items::checked_function(0x3a2cb0, &[0x40, 0x53, 0x55, 0x56, 0x57, 0x48, 0x81, 0xec, 0x98, 0x07, 0x00, 0x00])?};
    let manager = unsafe { CSBulletManager::instance_mut() }.map_err(|_| "bullet manager not ready")?;
    let mut data = SpawnData([0; 0x110]);
    let owner = unsafe { std::ptr::read_unaligned((&chr.field_ins_handle as *const eldenring::cs::FieldInsHandle).cast::<u64>()) };
    let put_i32 = |data: &mut SpawnData, at: usize, v: i32| data.0[at..at + 4].copy_from_slice(&v.to_le_bytes());
    data.0[0..8].copy_from_slice(&owner.to_le_bytes());
    // 带上法术编号：游戏按“这个玩家放出了这个法术”来处理（法术补正、效果、联机同步与原版施法释放时一致）
    put_i32(&mut data, 0x08, -1); put_i32(&mut data, 0x0c, magic);
    put_i32(&mut data, 0x14, bullet);
    put_i32(&mut data, 0x18, -1); put_i32(&mut data, 0x1c, -1);
    data.0[0x20..0x28].fill(0xff);
    // 面朝方向：physics.orientation 是绕 Y 轴的四元数，朝向为 (-sin yaw, 0, -cos yaw)
    let q = chr.modules.physics.orientation;
    let yaw = 2.0 * q.1.atan2(q.3);
    let forward = [-yaw.sin(), 0.0, -yaw.cos()];
    let right = [forward[2], 0.0, -forward[0]];
    // 从人物前方 1.5 米处放出（像从手上放出那样），不贴在身体里：贴身生成的雾类判定会一直残留在身上
    let p = chr.modules.physics.position;
    for (offset, row) in [(0x50, [right[0], right[1], right[2], 0.0]), (0x60, [0.0, 1.0, 0.0, 0.0]), (0x70, [forward[0], forward[1], forward[2], 0.0]), (0x80, [p.0 + forward[0] * 1.5, p.1 + 1.2, p.2 + forward[2] * 1.5, 1.0])] {
        for (i, v) in row.into_iter().enumerate() { data.0[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&f32::to_le_bytes(v)); }
    }
    let spawn: Spawn = unsafe { std::mem::transmute(function) };
    let (mut out, mut error) = (0i32, 0i32);
    unsafe { spawn((manager as *mut CSBulletManager).cast(), &mut out, &data, &mut error) };
    if out == -1 { return Err(format!("spawn error {error}")); }
    Ok(())
}
