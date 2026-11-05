// 内置「死亡不掉卢恩」：本机角色活着时常驻祭品树枝的效果（SpEffect 360700，stateInfo 159：死亡时不掉卢恩）。
// 原版这个效果只在戴着祭品树枝护符时生效（持续时间 0），这里改成常驻、不显示图标，再挂到本机角色身上；
// 没有戴护符，死亡时也就没有护符可碎。只改本机内存里的参数，不改 regulation。
use eldenring::cs::{ChrIns,ChrInsExt,SoloParamRepository,SpEffectParam};
use fromsoftware_shared::FromStatic;

const EFFECT:i32=360700;
const KEEP_RUNES:u16=159;

pub fn tick(chr:&mut ChrIns) {
    if chr.modules.data.hp<=0||chr.chr_flags1c5.death_flag()||chr.special_effect.entries().any(|e|e.param_id==EFFECT) {return;}
    let Ok(params)=(unsafe {SoloParamRepository::instance_mut()}) else {return;};
    let Some(row)=params.get_mut::<SpEffectParam>(EFFECT as u32) else {return;};
    if row.state_info()!=KEEP_RUNES {super::log_repeating(format!("Keep runes: SpEffect {EFFECT} stateInfo={} is not {KEEP_RUNES}; skipped",row.state_info()));return;}
    if row.effect_endurance()!=-1.0 {row.set_effect_endurance(-1.0);row.set_icon_id(-1);}
    chr.apply_speffect(EFFECT,true);
}
