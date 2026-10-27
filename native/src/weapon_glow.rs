// 传说武器流光：握在手里的传说品质词条武器，刀身轮流挂上原版的金色附魔光效（黄金淬炼、神圣涂油、雷电涂油），
// 刀身特效、刀身染色与挥砍轨迹颜色一起变化。效果由 scene-builder weapon-glow-patch 写入 9830000 段，只有视觉、不加数值。
// 同一只手已有原版附魔（涂油、战技、祷告）时让位；双手持握时收在背上的那把不发光；换下武器后撤下。
// 调试菜单（物品页）可以切换配色，或让任意近战武器预览效果。
use std::sync::{OnceLock,atomic::{AtomicBool,AtomicU8,Ordering}};
use std::time::Instant;
use eldenring::cs::{ChrAsmArmStyle,ChrIns,ChrInsExt,EquipParamWeapon,PlayerGameData,SoloParamRepository,SpEffectParam,SpEffectVfxParam};
use fromsoftware_shared::FromStatic;

pub const FIRST:i32=9830000;
/// 光效编号是右手，+1 是左手
const TEMPERING:i32=FIRST;
const HOLY:i32=FIRST+10;
const LIGHTNING:i32=FIRST+20;
/// 每种光效停留的秒数；换到下一种时上一种多留一小段，减少断档
const STEP:f64=1.4;
const OVERLAP:f64=0.3;

#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub enum Style {Flow,FlowLightning,Tempering,Holy,Off}
impl Style {
    pub const ALL:[Style;5]=[Style::Flow,Style::FlowLightning,Style::Tempering,Style::Holy,Style::Off];
    pub fn label(self)->&'static str {
        match self {
            Style::Flow=>"金色流动（黄金淬炼 ↔ 神圣光）",
            Style::FlowLightning=>"金色流动 + 雷光",
            Style::Tempering=>"只用黄金淬炼",
            Style::Holy=>"只用神圣光",
            Style::Off=>"关闭",
        }
    }
    fn palette(self)->&'static [i32] {
        match self {
            Style::Flow=>&[TEMPERING,HOLY],
            Style::FlowLightning=>&[TEMPERING,HOLY,TEMPERING,LIGHTNING],
            Style::Tempering=>&[TEMPERING],
            Style::Holy=>&[HOLY],
            Style::Off=>&[],
        }
    }
}
static STYLE:AtomicU8=AtomicU8::new(0);
static PREVIEW:AtomicBool=AtomicBool::new(false);
pub fn style()->Style {Style::ALL[STYLE.load(Ordering::Relaxed) as usize%Style::ALL.len()]}
pub fn set_style(style:Style) {STYLE.store(Style::ALL.iter().position(|s|*s==style).unwrap_or(0) as u8,Ordering::Relaxed);}
/// 调试：任意近战武器都发光，方便不拿传说武器时看效果
pub fn preview()->bool {PREVIEW.load(Ordering::Relaxed)}
pub fn set_preview(on:bool) {PREVIEW.store(on,Ordering::Relaxed);}
pub fn clock()->f64 {static START:OnceLock<Instant>=OnceLock::new();START.get_or_init(Instant::now).elapsed().as_secs_f64()}

fn ours(id:i32)->bool {(FIRST..FIRST+100).contains(&id)}

/// 此刻应当挂着的光效；hands 为 [右手, 左手] 是否发光
fn desired(style:Style,time:f64,hands:[bool;2])->Vec<i32> {
    let palette=style.palette();
    if palette.is_empty() {return vec![];}
    let (step,count)=((time/STEP).floor() as i64,palette.len() as i64);
    let current=palette[step.rem_euclid(count) as usize];
    let previous=Some(palette[(step-1).rem_euclid(count) as usize]).filter(|p|*p!=current&&time.rem_euclid(STEP)<OVERLAP);
    let mut want=vec![];
    for (hand,on) in hands.into_iter().enumerate() {
        if !on {continue;}
        want.push(current+hand as i32);
        if let Some(previous)=previous {want.push(previous+hand as i32);}
    }
    want
}

/// 刀身能挂附魔光效的武器类别（原版编号的百万位）：排除火把、盾、法杖、圣印记、弓弩与香水瓶。
/// 不用“可附魔”标志判断：剑骸大剑这类特殊武器原版不能涂油，但刀身一样能挂光效
fn has_blade(base:u32)->bool {!matches!(erdtree_tower::affixes::base(base)/1_000_000,24|30..=34|40..=44|61)}

/// 握在手里的传说近战武器：[右手, 左手]
fn glowing_hands(data:&PlayerGameData,params:&SoloParamRepository,preview:bool)->[bool;2] {
    let asm=&data.equipment.chr_asm;
    let slots=[asm.equipment.active_right_weapon_slot() as usize,asm.equipment.active_left_weapon_slot() as usize];
    // 双手持握时另一只手的武器收在背上
    let held=[asm.equipment.arm_style!=ChrAsmArmStyle::LeftBothHands,asm.equipment.arm_style!=ChrAsmArmStyle::RightBothHands];
    std::array::from_fn(|hand| {
        let Some(id)=asm.equipment_param_ids.get(slots[hand]).copied().filter(|id|*id>0) else {return false};
        let base=id as u32/100*100;
        held[hand]&&has_blade(base)&&params.get::<EquipParamWeapon>(base).is_some()
            &&(preview||erdtree_tower::affixes::tier(base)==Some(3))
    })
}

/// 已经挂着原版刀身附魔光效的手：[右手, 左手]
fn enchanted_hands(chr:&ChrIns,params:&SoloParamRepository)->[bool;2] {
    let mut hands=[false;2];
    for entry in chr.special_effect.entries().filter(|e|!ours(e.param_id)) {
        let Some(effect)=params.get::<SpEffectParam>(entry.param_id as u32) else {continue};
        for vfx in [effect.vfx_id(),effect.vfx_id1(),effect.vfx_id2(),effect.vfx_id3(),effect.vfx_id4(),effect.vfx_id5(),effect.vfx_id6(),effect.vfx_id7()] {
            let Some(look)=(vfx>0).then(||params.get::<SpEffectVfxParam>(vfx as u32)).flatten() else {continue};
            if look.enchant_start_dmy_id_0()<0 {continue;}
            match look.effect_type() {1=>hands[0]=true,2=>hands[1]=true,_=>{}}
        }
    }
    hands
}

pub fn sync(chr:&mut ChrIns,data:&PlayerGameData,time:f64) {
    let Ok(params)=(unsafe {SoloParamRepository::instance()}) else {return};
    let glowing=glowing_hands(data,params,preview());
    let busy=enchanted_hands(chr,params);
    let hands=[glowing[0]&&!busy[0],glowing[1]&&!busy[1]];
    let want:Vec<_>=desired(style(),time,hands).into_iter().filter(|id|params.get::<SpEffectParam>(*id as u32).is_some()).collect();
    let present:Vec<_>=chr.special_effect.entries().map(|e|e.param_id).filter(|id|ours(*id)).collect();
    for id in &present {if !want.contains(id){chr.remove_speffect(*id);}}
    for id in want {if !present.contains(&id){chr.apply_speffect(id,false);}}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flow_alternates_with_a_short_overlap_per_hand() {
        assert_eq!(desired(Style::Flow,0.5,[true,false]),vec![TEMPERING]);
        assert_eq!(desired(Style::Flow,STEP+0.1,[true,true]),vec![HOLY,TEMPERING,HOLY+1,TEMPERING+1]);
        assert_eq!(desired(Style::Flow,STEP+0.5,[false,true]),vec![HOLY+1]);
        assert_eq!(desired(Style::Flow,2.0*STEP+0.5,[true,false]),vec![TEMPERING]);
        assert!(desired(Style::Flow,1.0,[false,false]).is_empty());
        assert!(desired(Style::Off,1.0,[true,true]).is_empty());
    }
    #[test]
    fn single_colour_styles_never_restart_the_effect() {
        for t in [0.0,0.1,STEP,STEP+0.1,10.0*STEP+0.05] {assert_eq!(desired(Style::Tempering,t,[true,false]),vec![TEMPERING]);}
    }
    #[test]
    fn blades_glow_but_catalysts_shields_and_bows_do_not() {
        assert!(has_blade(3000000)&&has_blade(19020000)&&has_blade(1000000));
        assert!(!has_blade(33000000)&&!has_blade(34000000)&&!has_blade(31000000)&&!has_blade(40000000)&&!has_blade(24000000));
        let legendary=erdtree_tower::affixes::CATALOG.iter().find(|i|erdtree_tower::affixes::tier(i.id)==Some(3)&&erdtree_tower::affixes::base(i.id)/1_000_000==3).unwrap();
        assert!(has_blade(legendary.id));
    }
    #[test]
    fn every_effect_stays_in_the_glow_range() {
        for style in Style::ALL {for t in (0..80).map(|i|i as f64*0.17) {assert!(desired(style,t,[true,true]).into_iter().all(ours));}}
    }
}
