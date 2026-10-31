//! [HansFix] 火把：每把火把是独立的一件装备（占 regulation 里预留的一个火把位 24800000 + 位号×100），
//! 各自带：原版火把外观、补正（力气/灵巧/智力/信仰/感应里随机 1~5 项）、3 个词条、一个法术战技、合成次数。
//! 法术战技是只能装在火把上的专用战灰（990000 + 法术序号），火把之间可以互换；
//! 左手举起火把（防御）满 10 秒自动释放这个法术，不做动作。
//! 合成次数 0/1/2 对应 火把 / 灼烧的火把 / 黑焰的火把。
use std::sync::LazyLock;
use serde::{Deserialize,Serialize};

pub const SLOT_FIRST:u32=24800000;
pub const SLOTS:u16=512;
pub const GEM_FIRST:u32=990000;
pub const ART_FIRST:u32=9600;
pub const FLASH_FIRST:u32=9600300;
/// 原版火把（本体 6 把 + DLC 3 把）
pub const BASES:[u32;9]=[24000000,24020000,24040000,24050000,24060000,24070000,24500000,24510000,24590000];
pub const STATS:[&str;5]=["力气","灵巧","智力","信仰","感应"];
pub const LETTERS:[&str;7]=["","E","D","C","B","A","S"];
/// 字母档位换成 +0 时的补正数值（取各档中间值）
pub const VALUES:[f32;7]=[0.0,15.0,40.0,75.0,115.0,155.0,190.0];
/// 火把的词条池（与装备词条同一套效果行：9850000 + 种类×10 + 品质）
pub const AFFIX_KINDS:[u32;10]=[4,7,16,17,5,1,19,11,8,10];

pub struct Spell {pub index:u8,pub magic:u32,pub name:&'static str,pub icon:u32,pub reference:u32,pub bullet:bool,pub sfx:i32}
pub static SPELLS:LazyLock<Vec<Spell>>=LazyLock::new(||include_str!("../scene-assets/hansfix-torch-spells.tsv").lines()
    .filter(|l|!l.starts_with('#')&&!l.trim().is_empty()).map(|l|{
        let p:Vec<&str>=l.split('\t').collect();
        Spell{index:p[0].parse().unwrap(),magic:p[1].parse().unwrap(),name:p[2],icon:p[3].parse().unwrap(),reference:p[4].parse().unwrap(),bullet:p[5]=="bullet",sfx:p[6].parse().unwrap()}
    }).collect());

#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct Torch {
    pub slot:u16,
    /// 外观来自哪把原版火把
    pub base:u32,
    /// 品质 0 普通 1 稀有 2 史诗 3 传说（决定词条档位）
    pub quality:u8,
    /// 合成次数：0 火把、1 灼烧的火把、2 黑焰的火把
    #[serde(default)]
    pub merges:u8,
    /// 五项补正的档位：0 无，1..=6 为 E D C B A S
    pub scaling:[u8;5],
    /// 词条效果行
    pub affixes:Vec<u32>,
    /// 法术序号（SPELLS）
    pub spell:u8,
}
impl Torch {
    pub fn weapon_id(&self)->u32 {SLOT_FIRST+u32::from(self.slot)*100}
    pub fn gem(&self)->u32 {GEM_FIRST+u32::from(self.spell)}
    pub fn prefix(&self)->&'static str {match self.merges {0=>"",1=>"灼烧的",_=>"黑焰的"}}
    pub fn scaling_values(&self)->[f32;5] {self.scaling.map(|l|VALUES[usize::from(l.min(6))])}
    /// 说明里的补正行，例如“补正：力气 C　信仰 A”
    pub fn scaling_line(&self)->String {
        let parts:Vec<String>=(0..5).filter(|&i|self.scaling[i]>0).map(|i|format!("{} {}",STATS[i],LETTERS[usize::from(self.scaling[i])])).collect();
        format!("补正：{}",parts.join("　"))
    }
    pub fn affix_lines(&self)->Vec<String> {
        self.affixes.iter().filter_map(|&e|crate::descent::affix(e).map(|(kind,tier)|crate::descent::describe(kind,tier,1))).collect()
    }
}
pub fn slot_of(weapon:u32)->Option<u16> {
    let id=weapon/100*100;
    (SLOT_FIRST..SLOT_FIRST+u32::from(SLOTS)*100).contains(&id).then(||((id-SLOT_FIRST)/100) as u16)
}
pub fn spell_of_gem(gem:u32)->Option<&'static Spell> {gem.checked_sub(GEM_FIRST).and_then(|i|SPELLS.get(i as usize))}

/// 补正项数为 k 时单项档位的分布（E D C B A S 的百分比）
pub fn letter_odds(k:usize)->[u32;6] {
    // 众数 50%，其余按 E D C B A S 的顺序依次递减
    let mode=match k {5=>0,4=>1,3=>2,2=>3,_=>4};
    if k<=1 {return [0,0,25,15,50,10];}
    let rest=[20,13,9,5,3];
    let mut odds=[0u32;6];odds[mode]=50;
    let mut r=rest.iter();
    for (i,slot) in odds.iter_mut().enumerate() {if i!=mode {*slot=*r.next().unwrap();}}
    odds
}
fn pick(roll:u64,odds:&[u32])->usize {
    let total:u32=odds.iter().sum();let mut v=(roll%u64::from(total)) as u32;
    for (i,o) in odds.iter().enumerate() {if v<*o {return i;} v-=o;}
    odds.len()-1
}
/// 随机一把新火把
pub fn roll(seed:u64,slot:u16,quality:u8,dlc:bool)->Torch {
    let r=|salt:u64|crate::mix(seed^salt);
    let bases:Vec<u32>=BASES.iter().copied().filter(|b|dlc||*b<24500000).collect();
    let base=bases[(r(0x70c4)%bases.len() as u64) as usize];
    let k=1+(r(0x5ca1)%5) as usize;
    let mut stats:Vec<usize>=(0..5).collect();stats.sort_by_key(|i|r(0x57a7^*i as u64));
    let mut scaling=[0u8;5];
    for (n,&stat) in stats.iter().take(k).enumerate() {scaling[stat]=1+pick(r(0x1e77^n as u64),&letter_odds(k)) as u8;}
    let mut kinds=AFFIX_KINDS.to_vec();kinds.sort_by_key(|kind|r(0xaff1^u64::from(*kind)));
    let tier=u32::from(quality.min(3));
    let affixes=kinds.iter().take(3).map(|kind|crate::descent::AFFIX_FIRST+kind*10+tier).collect();
    let spell=(r(0x5be1)%SPELLS.len() as u64) as u8;
    Torch{slot,base,quality:quality.min(3),merges:0,scaling,affixes,spell}
}

impl crate::Run {
    /// 新拿到一把火把：占用最小的空闲火把位，返回它
    pub fn new_torch(&mut self,quality:u8,salt:u64)->crate::RuleResult<Torch> {
        let slot=(0..SLOTS).find(|s|!self.torches.iter().any(|t|t.slot==*s)).ok_or("火把位已用完")?;
        self.torch_rolls=self.torch_rolls.wrapping_add(1);
        let seed=crate::mix(self.reward_seed()^salt^u64::from(self.torch_rolls).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let torch=roll(seed,slot,quality,crate::dlc::mode()==crate::dlc::Mode::Full);
        self.torches.push(torch.clone());
        Ok(torch)
    }
    pub fn torch(&self,weapon:u32)->Option<&Torch> {let slot=slot_of(weapon)?;self.torches.iter().find(|t|t.slot==slot)}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spells_table_is_complete() {
        assert_eq!(SPELLS.len(),37);
        assert!(SPELLS.iter().enumerate().all(|(i,s)|usize::from(s.index)==i&&!s.name.is_empty()&&s.icon>0));
        assert!(SPELLS.iter().any(|s|s.magic==6600&&s.bullet),"Golden Vow casts through its area bullet");
    }
    #[test]
    fn odds_follow_the_scaling_count() {
        for k in 1..=5 {assert_eq!(letter_odds(k).iter().sum::<u32>(),100);}
        assert_eq!(letter_odds(5)[0],50);assert_eq!(letter_odds(4)[1],50);assert_eq!(letter_odds(3)[2],50);assert_eq!(letter_odds(2)[3],50);
        assert_eq!(&letter_odds(1)[..2],&[0,0],"a single scaling is never below C");
    }
    #[test]
    fn rolled_torches_are_valid_and_slots_are_unique() {
        let mut run=crate::Run::new(9,crate::Class::Hero);
        for n in 0..40 {
            let t=run.new_torch((n%4) as u8,n).unwrap();
            let k=t.scaling.iter().filter(|l|**l>0).count();
            assert!((1..=5).contains(&k));assert_eq!(t.affixes.len(),3);
            if k==1 {assert!(t.scaling.iter().all(|l|*l==0||*l>=3));}
            assert!(usize::from(t.spell)<SPELLS.len());assert!(BASES.contains(&t.base));
            assert_eq!(slot_of(t.weapon_id()+7),Some(t.slot));assert_eq!(run.torch(t.weapon_id()).unwrap(),&t);
        }
        let slots:std::collections::HashSet<_>=run.torches.iter().map(|t|t.slot).collect();assert_eq!(slots.len(),40);
        assert_eq!(slot_of(24000000),None);
        assert_eq!(crate::Run::decode(&run.encode().unwrap()).unwrap(),run);
    }
}
