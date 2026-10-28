// 角色光环：本机角色身上常驻选用的光环特效（src/auras.rs，9870000 + 编号，只有原版增益的身体光效）。
// 挂特效时同步给联机队友，队友也看得到；换光环或摘下时撤下其他光环。选择存在角色存档旁的 .aura 文件。
use eldenring::cs::{ChrIns,ChrInsExt,SoloParamRepository,SpEffectParam};
use fromsoftware_shared::FromStatic;

static WORN:std::sync::atomic::AtomicU8=std::sync::atomic::AtomicU8::new(0);
pub fn worn()->u8 {WORN.load(std::sync::atomic::Ordering::Relaxed)}

fn path(owner:(u32,i32))->std::path::PathBuf {super::hub::path(owner,"aura")}
/// 读取并校验本机的光环：没解锁的光环不挂
pub fn load(owner:(u32,i32)) {
    let chosen=std::fs::read_to_string(path(owner)).ok().and_then(|s|s.trim().parse::<u8>().ok()).unwrap_or(0);
    let worn=erdtree_tower::auras::worn(chosen,&super::run_log::records()).map_or(0,|a|a.id);
    WORN.store(worn,std::sync::atomic::Ordering::Relaxed);
}
/// 选用光环（0 为摘下），立即写盘
pub fn choose(owner:(u32,i32),id:u8)->Result<(),String> {
    if id!=0&&!erdtree_tower::auras::unlocked(id,&super::run_log::records()) {return Err("这个光环还没有解锁".into());}
    std::fs::write(path(owner),id.to_string()).map_err(|e|format!("光环保存失败：{e}"))?;
    WORN.store(id,std::sync::atomic::Ordering::Relaxed);
    super::log(format!("Aura chosen: {id}"));Ok(())
}
/// 每帧：本机角色挂上选用的光环、撤下其他光环（参数缺失时不挂）
pub fn sync(chr:&mut ChrIns) {
    if chr.modules.data.hp<=0||chr.chr_flags1c5.death_flag() {return;}
    let Ok(params)=(unsafe {SoloParamRepository::instance()}) else{return;};
    let wanted=Some(worn()).filter(|id|*id!=0).map(erdtree_tower::auras::effect);
    let present:Vec<i32>=chr.special_effect.entries().map(|e|e.param_id).collect();
    for aura in erdtree_tower::auras::AURAS.iter() {
        let id=erdtree_tower::auras::effect(aura.id);
        if present.contains(&id)&&wanted!=Some(id) {chr.remove_speffect(id);}
    }
    if let Some(id)=wanted.filter(|id|!present.contains(id)&&params.get::<SpEffectParam>(*id as u32).is_some()) {chr.apply_speffect(id,false);}
}
