//! [HansFix] 游龙模式·向下：沿连线向上以外的方向进入关卡（原版不允许的方向）记为一次“向下”。
//! - 向下进入的关卡在地图上标红，整局累计向下次数；
//! - 第 k 次向下的战斗关卡：敌人血量 ×2×1.1^(k-1)，攻击伤害 ×1.35×1.04^(k-1)（第一次正好 2 倍 / 1.35 倍）；
//! - 打完红色战斗关卡进入与精英相同的结算界面；其中带随机词条的武器与防具会被“扭曲”：
//!   随机一个词条数值 ×2、另一个词条数值 ×(-1)；只有一个词条时随机 ×2 或 ×(-1)。
//! 词条效果是 regulation 里的 SpEffect 行（9850000 + 种类×10 + 品质），×2 / ×(-1) 的版本由
//! HansFix 的 regulation 补丁追加在 9851000 / 9852000 起的同一位置。
use std::{collections::HashMap,sync::LazyLock};
use serde::{Deserialize,Serialize};

pub const HP_START:f64=2.0;
pub const HP_STEP:f64=1.1;
pub const ATTACK_START:f64=1.35;
pub const ATTACK_STEP:f64=1.04;
/// 敌人强化效果：ENEMY_EFFECT_FIRST + k（k = 第几次向下，超过 MAX_LEVEL 按 MAX_LEVEL）
pub const ENEMY_EFFECT_FIRST:u32=9641000;
pub const MAX_LEVEL:u32=60;
pub const AFFIX_FIRST:u32=9850000;
pub const DOUBLE_FIRST:u32=9851000;
pub const NEGATE_FIRST:u32=9852000;
pub const AFFIX_KINDS:u32=21;
pub const FIXED_KIND:u32=18;

pub fn hp_rate(level:u32)->f64 {HP_START*HP_STEP.powi(level.clamp(1,MAX_LEVEL) as i32-1)}
pub fn attack_rate(level:u32)->f64 {ATTACK_START*ATTACK_STEP.powi(level.clamp(1,MAX_LEVEL) as i32-1)}
pub fn enemy_effect(level:u32)->u32 {ENEMY_EFFECT_FIRST+level.clamp(1,MAX_LEVEL)}

/// 一件被扭曲的装备：物品编号（防具带 0x10000000 标记）与“原词条效果 → 新效果”
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct Cursed {pub item:u32,pub swaps:Vec<(u32,u32)>}

/// 词条装备（v2）带的词条效果，顺序同游戏参数里的常驻效果栏位
static EFFECTS:LazyLock<HashMap<u32,Vec<u32>>>=LazyLock::new(|| {
    include_str!("../scene-assets/affix-v2-audit.tsv").lines().skip(1).filter_map(|line| {
        let p:Vec<_>=line.split('\t').collect();
        let effects=p.get(8)?.split(',').filter_map(|e|e.trim().parse().ok()).filter(|e|affix(*e).is_some()).collect();
        Some((p[0].parse().ok()?,effects))
    }).collect()
});
pub fn effects(item:u32)->&'static [u32] {EFFECTS.get(&item).map_or(&[],Vec::as_slice)}
/// 词条效果编号 → (种类, 品质)
pub fn affix(effect:u32)->Option<(u32,u32)> {
    let offset=effect.checked_sub(AFFIX_FIRST)?;
    // 种类 18（防御时精力消耗）在游戏里是护符的固定行为（stateInfo），没有可翻倍的数值，不参与扭曲
    (offset<AFFIX_KINDS*10&&offset%10<4&&offset/10!=FIXED_KIND).then_some((offset/10,offset%10))
}
pub fn doubled(effect:u32)->u32 {effect-AFFIX_FIRST+DOUBLE_FIRST}
pub fn negated(effect:u32)->u32 {effect-AFFIX_FIRST+NEGATE_FIRST}

/// 红色关卡奖励里的一件词条装备会怎样被扭曲（同一局、同一关卡、同一件装备结果固定）
pub fn plan(seed:u64,node:usize,item:u32)->Option<Cursed> {
    let effects=effects(item);
    let roll=crate::mix(seed^(node as u64).wrapping_mul(7919)^u64::from(item)^0xde5c_e27d);
    let swaps=match effects.len() {
        0=>return None,
        1=>vec![(effects[0],if roll%2==0 {doubled(effects[0])} else {negated(effects[0])})],
        n=>{
            let up=(roll%n as u64) as usize;
            let down=(up+1+((roll>>16)%(n as u64-1)) as usize)%n;
            vec![(effects[up],doubled(effects[up])),(effects[down],negated(effects[down]))]
        },
    };
    Some(Cursed{item,swaps})
}

/// 词条说明（与 scene-builder AffixAssets.EffectText 一致），factor 为 1、2 或 -1
pub fn describe(kind:u32,tier:u32,factor:i32)->String {
    let t=tier as f64;
    // (名称, 原版数值, 正数是否写成“-”：减少类词条显示为负号)
    let (name,value,reduce,unit)=match kind {
        0=>("最大生命",2.0+t,false,"%"),1=>("最大专注",3.0+t,false,"%"),2=>("最大耐力",2.0+t,false,"%"),3=>("耐力恢复速度",1.0+t,false,""),
        4=>("各类攻击伤害",2.0+t,false,"%"),5=>("魔法与祷告专注消耗",2.0+t,true,"%"),6=>("物理承伤",1.0+t*0.5,true,"%"),
        7=>("战技伤害",6.0+2.0*t,false,"%"),8=>("致命一击伤害",8.0+3.0*t,false,"%"),9=>("受到的削韧",5.0+2.0*t,true,"%"),10=>("卢恩获取",5.0+3.0*t,false,"%"),
        11=>("元素承伤",2.0+t,true,"%"),12=>("蓄力重攻击伤害",6.0+2.0*t,false,"%"),13=>("跳跃攻击伤害",6.0+2.0*t,false,"%"),14=>("防御反击伤害",8.0+2.0*t,false,"%"),
        15=>("弓弩伤害",5.0+2.0*t,false,"%"),16=>("魔法伤害",4.0+2.0*t,false,"%"),17=>("祷告伤害",4.0+2.0*t,false,"%"),18=>("防御时精力消耗",8.0+3.0*t,true,"%"),
        19=>("动作精力消耗",3.0+t,true,"%"),_=>("圣杯瓶恢复量",4.0+2.0*t,false,"%"),
    };
    let signed=value*f64::from(factor)*if reduce {-1.0} else {1.0};
    let number=format!("{:.1}",signed.abs());let number=number.trim_end_matches(".0");
    format!("{name} {}{number}{unit}",if signed<0.0 {"-"} else {"+"})
}

/// 把装备说明里被扭曲的词条行换成新数值，并标注 ×2 / ×-1
pub fn rewrite(text:&str,cursed:&Cursed)->String {
    let mut text=text.to_owned();
    for &(from,to) in &cursed.swaps {
        let Some((kind,tier))=affix(from) else{continue};
        let factor=if to>=NEGATE_FIRST {-1} else {2};
        let original=describe(kind,tier,1);
        let replaced=format!("{}（{}）",describe(kind,tier,factor),if factor==2 {"×2"} else {"×-1"});
        text=text.replace(&original,&replaced);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scaling_starts_at_double_and_grows_per_descent() {
        assert!((hp_rate(1)-2.0).abs()<1e-9&&(attack_rate(1)-1.35).abs()<1e-9);
        assert!((hp_rate(3)-2.0*1.21).abs()<1e-9&&(attack_rate(3)-1.35*1.04*1.04).abs()<1e-9);
        assert_eq!(enemy_effect(0),ENEMY_EFFECT_FIRST+1);assert_eq!(enemy_effect(999),ENEMY_EFFECT_FIRST+MAX_LEVEL);
    }
    #[test]
    fn descriptions_match_the_generated_item_text_and_curses_change_one_or_two_lines() {
        assert_eq!(describe(6,1,1),"物理承伤 -1.5%");assert_eq!(describe(18,0,1),"防御时精力消耗 -8%");assert_eq!(describe(3,2,1),"耐力恢复速度 +3");
        assert_eq!(describe(18,0,-1),"防御时精力消耗 +8%");assert_eq!(describe(0,3,2),"最大生命 +10%");
        let mut seen=(0,0);
        for line in include_str!("../scene-assets/affix-v2-details.tsv").lines().take(4000) {
            let p:Vec<_>=line.split('\t').collect();let id:u32=p[0].parse().unwrap();
            let info=p[3].replace("\\n","\n");
            for &e in effects(id) {let (k,t)=affix(e).unwrap();assert!(info.contains(&describe(k,t,1)),"{id} {info}");}
            for node in 0..3 {
                let Some(c)=plan(7,node,id) else{assert!(effects(id).is_empty());continue};
                assert_eq!(plan(7,node,id).as_ref(),Some(&c),"deterministic");
                let n=effects(id).len();assert_eq!(c.swaps.len(),n.min(2));
                if n>=2 {assert!(c.swaps[0].1>=DOUBLE_FIRST&&c.swaps[0].1<NEGATE_FIRST&&c.swaps[1].1>=NEGATE_FIRST&&c.swaps[0].0!=c.swaps[1].0);seen.1+=1;} else {seen.0+=1;}
                let text=rewrite(&info,&c);assert_ne!(text,info);
                for (from,to) in &c.swaps {assert_eq!(to-from,if *to>=NEGATE_FIRST {NEGATE_FIRST-AFFIX_FIRST} else {DOUBLE_FIRST-AFFIX_FIRST});}
            }
        }
        assert!(seen.0>0&&seen.1>0);
    }
}
