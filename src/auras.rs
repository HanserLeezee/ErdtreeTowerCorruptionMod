//! 角色光环：局外的外观奖励，不加数值。身上常驻一个原版的增益光效（只有视觉），按旅程档案（tower-runs.jsonl）的成绩解锁，
//! 每个角色存档自选一个。特效参数为 9870000 + 编号，由 scene-builder aura-patch 复制中性的 90400 后只改光效；
//! 原生 DLL 挂在本机角色身上并同步给联机队友（队友看得到）。
use crate::journal::{Outcome,Record};

pub const EFFECT_FIRST:i32=9870000;
pub struct Aura {pub id:u8,pub name:&'static str,pub vfx:i32,pub source:&'static str,pub unlock:&'static str,pub color:u32}
/// 编号写进存档，只能追加；vfx 是原版 SpEffectVfxParam 的行（都是挂在身上的常驻光效），与 scene-builder/AuraAssets.cs 一致
pub const AURAS:[Aura;9]=[
    Aura{id:1,name:"余烬之焰",vfx:1605000,source:"火焰赐我力量",unlock:"默认拥有",color:0xE8602C},
    Aura{id:2,name:"赐福微光",vfx:1643000,source:"赐福之恩",unlock:"完成 5 局旅程",color:0x9CD86A},
    Aura{id:3,name:"疾风之息",vfx:5846,source:"风之结晶露滴",unlock:"抵达第二幕",color:0x8CE0D0},
    Aura{id:4,name:"刺客暗影",vfx:1651000,source:"刺客的手法",unlock:"抵达第三幕",color:0x8C7CB8},
    Aura{id:5,name:"鲜血欢愉",vfx:321600,source:"鲜血君王的欢愉",unlock:"累计击败 30 个 Boss",color:0xC0303C},
    Aura{id:6,name:"神圣加护",vfx:1648000,source:"神圣加护",unlock:"登顶 1 次",color:0xF4EED8},
    Aura{id:7,name:"黑焰庇佑",vfx:1627000,source:"黑焰庇佑",unlock:"苦难律法 140 分以上登顶",color:0x6A3CA0},
    Aura{id:8,name:"黄金誓言",vfx:1660000,source:"黄金誓言",unlock:"苦难律法 210 分以上登顶",color:0xF2C85C},
    Aura{id:9,name:"黄金雷光",vfx:1790300,source:"黄金雷电",unlock:"苦难律法 280 分以上登顶",color:0xFFE27A},
];
pub fn lookup(id:u8)->Option<&'static Aura> {AURAS.iter().find(|a|a.id==id)}
pub fn effect(id:u8)->i32 {EFFECT_FIRST+i32::from(id)}
/// 按旅程档案判断是否解锁（调试旅程不算）
pub fn unlocked(id:u8,records:&[Record])->bool {
    let real=||records.iter().filter(|r|!r.debug);
    let won=||real().filter(|r|r.result==Outcome::Won);
    match id {
        1=>true,
        2=>real().count()>=5,
        3=>real().any(|r|r.act>=2),
        4=>real().any(|r|r.act>=3),
        5=>real().map(|r|r.fights.iter().filter(|f|f.kind=="Boss"&&f.won).count()).sum::<usize>()>=30,
        6=>won().next().is_some(),
        7=>won().any(|r|r.law.score()>=140),
        8=>won().any(|r|r.law.score()>=210),
        9=>won().any(|r|r.law.score()>=280),
        _=>false,
    }
}
/// 选用的光环：没有解锁或编号无效时不挂（0 表示不挂）
pub fn worn(id:u8,records:&[Record])->Option<&'static Aura> {lookup(id).filter(|a|unlocked(a.id,records))}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Class,Run,journal::{Fight,Record}};
    #[test]
    fn ids_effects_and_visuals_are_unique() {
        for (i,a) in AURAS.iter().enumerate() {
            assert!(a.id>0&&AURAS[..i].iter().all(|o|o.id!=a.id&&o.vfx!=a.vfx&&o.name!=a.name),"{}",a.name);
            assert!((EFFECT_FIRST..EFFECT_FIRST+100).contains(&effect(a.id)));
        }
    }
    #[test]
    fn unlocks_follow_the_archive() {
        let run=Run::new(1,Class::Hero);
        let lost=Record::new(&run,vec![],vec![],0);
        let mut won=lost.clone();won.result=Outcome::Won;
        assert!(unlocked(1,&[])&&!unlocked(2,&vec![lost.clone();4])&&unlocked(2,&vec![lost.clone();5]));
        let mut deep=lost.clone();deep.act=3;assert!(unlocked(3,std::slice::from_ref(&deep))&&unlocked(4,std::slice::from_ref(&deep)));
        let boss=|won:bool|Fight{act:1,layer:15,kind:"Boss".into(),won,enemies:vec![]};
        let mut hunter=lost.clone();hunter.fights=vec![boss(true);15];hunter.fights.push(boss(false));
        assert!(!unlocked(5,std::slice::from_ref(&hunter))&&unlocked(5,&[hunter.clone(),hunter]));
        assert!(unlocked(6,std::slice::from_ref(&won))&&!unlocked(7,std::slice::from_ref(&won)));
        let mut hard=won.clone();hard.law=(0..crate::law::NODES.len()).fold(Default::default(),|l:crate::law::Law,id|l.toggle(id));
        assert!(unlocked(9,std::slice::from_ref(&hard)));
        let mut debug=hard;debug.debug=true;assert!(!unlocked(9,&[debug]));
        assert!(worn(8,&[won.clone()]).is_none()&&worn(6,&[won]).is_some()&&worn(0,&[]).is_none());
    }
}
