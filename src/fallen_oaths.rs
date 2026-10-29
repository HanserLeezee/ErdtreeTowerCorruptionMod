//! [HansFix] 堕落誓约：新誓约（灵辉、锻铁、磐石、贪婪、疫血、吊命）只在堕落首领的第三页出现。
//! 每份新誓约没有原来的固定代价，改为两个随机负面词条，再加一个随机的“反转”词条作为正面；
//! 词条档位随誓约等级（I/II/III），深化时一起变强。
use serde::{Deserialize,Serialize};
use crate::contracts::Kind;

/// 只在堕落首领处出现的新誓约
pub const NEW:[Kind;6]=[Kind::Radiant,Kind::Iron,Kind::Bulwark,Kind::Greed,Kind::Plague,Kind::Lifeline];
/// 词条种类数：最大生命、受到属性伤害、最大精力、精力消耗、专注消耗、战技伤害
pub const CURSES:u8=6;
/// 词条效果行：9600200 + 种类×10 + 等级-1（负面），+5（正面）
pub const CURSE_FIRST:i32=9600200;
pub fn percent(curse:u8,level:u8)->u32 {
    let l=usize::from(level.clamp(1,3)-1);
    match curse {1|5=>[10,12,15][l],_=>[8,10,12][l]}
}
/// 词条说明；positive 为反转后的正面版本
pub fn text(curse:u8,level:u8,positive:bool)->String {
    let p=percent(curse,level);
    let (name,worse_up)=match curse {
        0=>("最大生命",false),1=>("受到魔力／火／雷／圣伤害",true),2=>("最大精力",false),
        3=>("精力消耗",true),4=>("专注消耗",true),_=>("战技伤害",false),
    };
    let up=worse_up!=positive;
    format!("{name} {}{p}%",if up {"+"} else {"-"})
}
pub fn effect(curse:u8,level:u8,positive:bool)->i32 {CURSE_FIRST+i32::from(curse)*10+i32::from(level.clamp(1,3)-1)+if positive {5} else {0}}

#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct FallenOath {pub kind:Kind,pub level:u8,pub minus:[u8;2],pub plus:u8}
impl FallenOath {
    pub fn roll(kind:Kind,seed:u64)->Self {
        let mut order:Vec<u8>=(0..CURSES).collect();
        order.sort_by_key(|c|crate::mix(seed^u64::from(*c)^0xc0_75e5));
        Self{kind,level:1,minus:[order[0],order[1]],plus:order[2]}
    }
    pub fn valid(&self)->bool {
        NEW.contains(&self.kind)&&(1..=3).contains(&self.level)&&self.minus[0]!=self.minus[1]
            &&self.minus.iter().chain(Some(&self.plus)).all(|c|*c<CURSES)&&!self.minus.contains(&self.plus)
    }
    /// 词条效果行（不含誓约本身的增益）
    pub fn curse_effects(&self)->Vec<i32> {
        vec![effect(self.minus[0],self.level,false),effect(self.minus[1],self.level,false),effect(self.plus,self.level,true)]
    }
    /// （说明, 是否正面）：增益在前，然后正面词条，最后两个负面
    pub fn lines(&self)->Vec<(String,bool)> {
        vec![(self.kind.boon_at(self.level),true),(text(self.plus,self.level,true),true),
            (text(self.minus[0],self.level,false),false),(text(self.minus[1],self.level,false),false)]
    }
}
/// 第三页上的一张卡
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Card {New(FallenOath),Deepen(usize)}
