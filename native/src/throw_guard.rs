// 处决时只有对面不无敌：原版的背刺、弹反处决等投技动画里，攻击方挂着“投技中攻击方无敌”（TAE 动作 68），
// 被处决的一方挂着“除投技外无敌”（TAE 动作 67），投技过程中双方都不吃其他伤害。
// 现在只清敌人身上的这两个标志：你（以及队友、召唤物）处决时照旧无敌，被处决的敌人会吃到召唤物、队友和其他攻击的伤害；
// 敌人抓投你时，抓你的敌人不再无敌，你照旧无敌。
// 每帧在 ChrIns_PostPhysics（动画事件在 HavokBehavior 里已经设好、伤害在之后的 DmgMan 里结算）清除。
// 只清投技的这两个，翻滚无敌、完全无敌（TAE 94）等其他无敌不动。
use eldenring::cs::{ChrIns,WorldChrMan};

/// 清掉一个角色身上的投技无敌，返回清之前是否挂着
fn clear(chr:&mut ChrIns)->bool {
    let flags=&mut chr.modules.action_flag.action_modifiers_flags;
    let had=flags.invincible_during_throw_attacker()||flags.invincible_excluding_throw_attacks_defender();
    flags.set_invincible_during_throw_attacker(false);
    flags.set_invincible_excluding_throw_attacks_defender(false);
    had
}

/// active：旅程进行中（大赐福、标题等其他时候保持原版）
pub fn sync(world:&mut WorldChrMan,active:bool) {
    if !active {return;}
    // 我方：本机玩家、所有玩家角色（联机队友）、召唤物（骨灰、灵马）——保留无敌
    let mut ours:Vec<usize>=world.player_chr_set.characters().map(|p|std::ptr::from_ref(&p.chr_ins) as usize).collect();
    ours.extend(world.main_player.as_ref().map(|p|std::ptr::from_ref(&p.chr_ins) as usize));
    ours.extend(world.summon_buddy_chr_set.characters().map(|c|std::ptr::from_ref(&*c) as usize));
    let mut cleared=0usize;
    for chr in world.chr_sets.iter().flatten().flat_map(|set|set.characters()) {
        if ours.contains(&(std::ptr::from_ref(&*chr) as usize)) {continue;}
        cleared+=usize::from(clear(chr));
    }
    if cleared>0 {log_once();}
}
fn log_once() {
    use std::sync::atomic::{AtomicBool,Ordering};
    static LOGGED:AtomicBool=AtomicBool::new(false);
    if !LOGGED.swap(true,Ordering::Relaxed) {super::log("TOWER_THROW enemy throw invincibility cleared");}
}
