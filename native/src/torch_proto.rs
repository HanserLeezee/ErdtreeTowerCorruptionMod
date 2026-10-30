//! [HansFix] 火把战技原型：火把装上专用战灰（图标为法术图标、战技名为法术名、战技本身无动作），
//! 左手举起火把（防御 / L1）累计 10 秒后直接把法术效果放在玩家身上，不播施法动作。
//! 原型阶段：每次领取关卡奖励都会额外得到一把实验火把（黄金誓言）。
use std::sync::Mutex;
use std::time::{Duration, Instant};
use eldenring::cs::{CSGaitemImp, CSWepGaitemIns, ChrAsmArmStyle, ChrIns, ChrInsExt, PlayerGameData, SoloParamRepository, SpEffectParam};
use fromsoftware_shared::{FromStatic, Superclass};

/// 原版火把
pub const TORCH: u32 = 24000000;
/// 实验战灰：黄金誓言（regulation 里由 torch_proto_patch.py 生成）
pub const GEM: i32 = 990000;
/// 黄金誓言的增益效果（原版祷告的范围判定挂的就是它）
const GOLDEN_VOW: i32 = 1660000;
/// 施法确认：0.5 秒的标记效果，只在玩家身上播放这个法术原版的释放特效与音效（不做动作）
const CAST_FLASH: i32 = 9600300;
const HOLD: Duration = Duration::from_secs(10);

struct State {
    held_since: Option<Instant>,
    last: Option<(i32, i32, bool, bool)>,
}
static STATE: Mutex<State> = Mutex::new(State { held_since: None, last: None });


fn torches(data: &PlayerGameData) -> u64 {
    super::loot::held(data, |e| e.category() == eldenring::cs::ItemCategory::Weapon && e.param_id() / 100 * 100 == TORCH)
}
/// 发一把带实验战灰的火把（法术战灰只能装在火把上，火把之间可以互换）
pub fn grant(data: &PlayerGameData) -> Result<(), String> {
    let before = torches(data);
    super::random_ashes::give(&[(TORCH as i32, 1, GEM)])?;
    let after = torches(data);
    super::log(format!("HANSFIX_TORCH granted torch={TORCH} gem={GEM} torches {before}->{after}"));
    if after <= before { return Err("游戏没有收下实验火把（背包数量未变化）".into()); }
    Ok(())
}

/// 原型：领取关卡奖励后额外发放（概率 100%）
pub fn on_reward_claimed(data: &PlayerGameData) {
    if !super::hansfix::wander() { return; }
    if let Err(e) = grant(data) { super::log(format!("HANSFIX_TORCH grant failed: {e}")); }
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

/// 每帧
pub fn sync(chr: &mut ChrIns, data: &PlayerGameData) {
    let Ok(mut state) = STATE.lock() else { return };
    let (weapon, gem) = left_weapon(data).unwrap_or((-1, -1));
    let ours = weapon >= 0 && weapon as u32 / 100 * 100 == TORCH && gem == GEM;
    let requests = &chr.modules.action_request.action_requests;
    let (guard, l1) = (requests.guard(), requests.l1());
    let raised = ours && (guard || l1);
    if state.last != Some((weapon, gem, guard, l1)) {
        // 只在左手是火把时记录，便于确认“举起”到底触发了哪个输入
        if weapon >= 0 && weapon as u32 / 100 * 100 == TORCH {
            super::log(format!("HANSFIX_TORCH left={weapon} gem={gem} ours={ours} guard={guard} l1={l1}"));
        }
        state.last = Some((weapon, gem, guard, l1));
    }
    if !raised { state.held_since = None; return; }
    let since = *state.held_since.get_or_insert_with(Instant::now);
    if since.elapsed() < HOLD { return; }
    state.held_since = Some(Instant::now());
    let present = unsafe { SoloParamRepository::instance() }.ok().is_some_and(|p| p.get::<SpEffectParam>(GOLDEN_VOW as u32).is_some());
    if present {
        chr.apply_speffect(CAST_FLASH, false);
        chr.apply_speffect(GOLDEN_VOW, false);
        super::log("HANSFIX_TORCH cast golden vow after 10s raised");
    }
}
