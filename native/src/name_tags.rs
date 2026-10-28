// 原版的友方名牌（队友头顶的名字与血条；队友不在画面内或被挡住时改画在屏幕左侧）：名字前加上队友选用的称号。
// 名牌数据在 CSFeManImp::friendly_chr_tag_displays，游戏每帧从这里复制给 HUD；这里只改显示用的名字，不动存档里的角色名。
// HUD 补丁（scene-builder/native_name_tag_glow.py）认出“称号·”前缀后给称号上特效；没有补丁时称号照样以原色文字显示。
use eldenring::cs::{CSFeManImp,WorldChrMan};
use fromsoftware_shared::FromStatic;

/// 本机选用的称号（已按旅程档案校验解锁），联机时随汇报发给房主
static WORN:std::sync::atomic::AtomicU8=std::sync::atomic::AtomicU8::new(0);
pub fn worn()->u8 {WORN.load(std::sync::atomic::Ordering::Relaxed)}

/// 称号选择存在角色存档旁的 .title 文件（与职业传承一样每个角色一份）
fn path(owner:(u32,i32))->std::path::PathBuf {super::hub::path(owner,"title")}
/// 读取并校验本机的称号：没解锁的称号不戴
pub fn load(owner:(u32,i32)) {
    let chosen=std::fs::read_to_string(path(owner)).ok().and_then(|s|s.trim().parse::<u8>().ok()).unwrap_or(0);
    let records=super::run_log::records();
    let worn=erdtree_tower::titles::worn(chosen,&records).map_or(0,|t|t.id);
    WORN.store(worn,std::sync::atomic::Ordering::Relaxed);
}
/// 选用称号（0 为摘下），立即写盘
pub fn choose(owner:(u32,i32),id:u8)->Result<(),String> {
    if id!=0&&!erdtree_tower::titles::unlocked(id,&super::run_log::records()) {return Err("这个称号还没有解锁".into());}
    std::fs::write(path(owner),id.to_string()).map_err(|e|format!("称号保存失败：{e}"))?;
    WORN.store(id,std::sync::atomic::Ordering::Relaxed);
    super::log(format!("Title chosen: {id}"));Ok(())
}

/// 每帧：按全队称号（Steam ID → 编号）改写友方名牌的名字；单人或离队时传空表，名字恢复原样
pub fn sync(world:&WorldChrMan,titles:&[(u64,u8)]) {
    let Ok(frontend)=(unsafe {CSFeManImp::instance_mut()}) else{return;};
    for entry in frontend.friendly_chr_tag_displays.iter_mut() {
        if entry.field_ins_handle.is_empty() {continue;}
        let Ok(current)=entry.name_string.to_string() else{continue};
        if current.is_empty() {continue;}
        let steam=world.player_chr_set.characters().find(|p|p.chr_ins.field_ins_handle==entry.field_ins_handle)
            .map(|p|p.player_session_holder.player_network_session.remote_identity);
        let title=steam.and_then(|id|titles.iter().find(|(p,_)|*p==id)).and_then(|(_,t)|erdtree_tower::titles::lookup(*t));
        let desired=erdtree_tower::titles::decorate(&current,title);
        if desired!=current {
            if let Err(error)=entry.name_string.assign_str(&desired) {super::log_repeating(format!("Name tag title failed: {error:?}"));}
        }
    }
}
