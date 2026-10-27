//! 职业传承：局外养成，不加数值。每个职业单独累计熟练度经验，升级后解锁新的开局护符，
//! 可以用来替换默认的祖灵角；开局时把选用的护符写进 Run，所以更换从下一局开始生效。
use serde::{Deserialize,Serialize};
use crate::{Class,Phase,Run,RuleResult,journal::{Outcome,Record}};

pub const MAX_LEVEL:u8=10;
/// 默认的开局护符：祖灵角
pub const DEFAULT_TALISMAN:u32=6110;
/// 新开局护符的解锁等级（与 talismans() 的第 2～5 项对应）
pub const UNLOCKS:[u8;4]=[1,3,6,10];
/// 每个职业可选的开局护符（护符编号），第一项是默认的祖灵角，其余按 UNLOCKS 的等级解锁。
/// 都是原版的基础护符，按职业的打法挑选，强度与祖灵角同档
pub fn talismans(class:Class)->[u32;5] {
    let unlocks=match class {
        Class::Vagabond=>[4000,1150,4100,2200],   // 龙徽盾护符、绿龟护符、大盾护符、曲剑护符
        Class::Warrior=>[2080,1150,2090,2050],    // 带翼剑徽章、绿龟护符、短剑护符、献斗剑护符
        Class::Hero=>[2130,2070,2190,8040],       // 斧护符、大槌护符、咆哮链坠、双手剑护符
        Class::Bandit=>[2090,2150,6000,5050],     // 短剑护符、硬箭护符、克雷普的小瓶子、染红凶刀
        Class::Astrologer=>[3000,6020,2110,5060], // 魔法师球护符、卡利亚徽章、蓝色舞娘、染蓝凶刀
        Class::Prophet=>[3040,3060,6020,2210],    // 信徒的立誓布、古王护符、卡利亚徽章、友好壶
        Class::Samurai=>[2050,2100,2080,1150],    // 献斗剑护符、远箭护符、带翼剑徽章、绿龟护符
        Class::Prisoner=>[2060,3000,4090,6020],   // 矛护符、魔法师球护符、献斗盾护符、卡利亚徽章
        Class::Confessor=>[3040,4000,6010,2200],  // 信徒的立誓布、龙徽盾护符、隐身面纱、曲剑护符
        Class::Wretch=>[2070,1030,2040,6050],     // 大槌护符、众武护符、红羽七刃剑、夏玻利利之祸
    };
    [DEFAULT_TALISMAN,unlocks[0],unlocks[1],unlocks[2],unlocks[3]]
}
/// 第 index 个开局护符的解锁等级（祖灵角为 0）
pub fn unlock_level(index:usize)->u8 {if index==0 {0}else{UNLOCKS[index-1]}}
/// 一个职业的传承：熟练度经验与选用的开局护符（0 表示默认的祖灵角）
#[derive(Clone,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
#[serde(default)]
pub struct ClassLegacy {pub xp:u32,pub talisman:u32}
/// 从 level 升到 level+1 需要的经验
pub fn next_xp(level:u8)->u32 {100+25*u32::from(level)}
impl ClassLegacy {
    /// (等级, 本级已有经验, 本级所需经验)；满级时后两项为 0
    pub fn progress(&self)->(u8,u32,u32) {
        let (mut level,mut left)=(0u8,self.xp);
        while level<MAX_LEVEL&&left>=next_xp(level) {left-=next_xp(level);level+=1;}
        if level==MAX_LEVEL {(level,0,0)}else{(level,left,next_xp(level))}
    }
    pub fn level(&self)->u8 {self.progress().0}
    pub fn unlocked(&self,class:Class,talisman:u32)->bool {
        talismans(class).iter().position(|id|*id==talisman).is_some_and(|i|self.level()>=unlock_level(i))
    }
    /// 开局实际使用的护符：选用的护符已解锁时用它，否则用祖灵角
    pub fn chosen(&self,class:Class)->u32 {if self.talisman!=0&&self.unlocked(class,self.talisman) {self.talisman}else{DEFAULT_TALISMAN}}
    pub fn choose(&mut self,class:Class,talisman:u32)->Result<(),&'static str> {
        if !talismans(class).contains(&talisman) {return Err("这个职业不能选用这枚护符");}
        if !self.unlocked(class,talisman) {return Err("熟练度不足，尚未解锁");}
        self.talisman=if talisman==DEFAULT_TALISMAN {0}else{talisman};Ok(())
    }
}
/// 一个角色存档下全部职业的传承
#[derive(Clone,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
#[serde(default)]
pub struct Legacy {pub classes:Vec<(Class,ClassLegacy)>}
impl Legacy {
    pub fn get(&self,class:Class)->ClassLegacy {self.classes.iter().find(|(c,_)|*c==class).map(|(_,l)|l.clone()).unwrap_or_default()}
    pub fn get_mut(&mut self,class:Class)->&mut ClassLegacy {
        if let Some(i)=self.classes.iter().position(|(c,_)|*c==class) {return &mut self.classes[i].1;}
        self.classes.push((class,ClassLegacy::default()));&mut self.classes.last_mut().unwrap().1
    }
    pub fn encode(&self)->String {serde_json::to_string(self).unwrap_or_default()}
    pub fn decode(text:&str)->Self {serde_json::from_str(text).unwrap_or_default()}
    /// 记一局结算：给这一局的职业加经验，返回变化（调试旅程不计）
    pub fn award(&mut self,record:&Record)->Option<Gain> {
        let xp=earned(record);
        if xp==0 {return None;}
        let entry=self.get_mut(record.class);
        let before=entry.level();entry.xp=entry.xp.saturating_add(xp);
        Some(Gain{class:record.class,xp,before,after:entry.level()})
    }
}
/// 一局结算带来的熟练度变化
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Gain {pub class:Class,pub xp:u32,pub before:u8,pub after:u8}
impl Gain {
    /// 这次升级新解锁的开局护符
    pub fn unlocked(&self)->Vec<u32> {
        talismans(self.class).into_iter().enumerate().filter(|(i,_)|*i>0&&(self.before+1..=self.after).contains(&unlock_level(*i))).map(|(_,id)|id).collect()
    }
}
/// 一局的熟练度经验：打赢的普通战 8、精英 20、Boss 按幕 50 / 100 / 150，抵达第二、三幕 +40 / +120（累计），登顶 +150；
/// 苦难律法每 2 分再加 1%（最多 +150%）。调试旅程为 0
pub fn earned(record:&Record)->u32 {
    if record.debug {return 0;}
    let fights:u32=if record.fights.is_empty() {record.battles*8} else {
        record.fights.iter().filter(|f|f.won).map(|f|match f.kind.as_str() {"精英"=>20,"Boss"=>50*u32::from(f.act.clamp(1,3)),_=>8}).sum()
    };
    let acts=match record.act {0|1=>0,2=>40,_=>120};
    let won=if record.result==Outcome::Won {150}else{0};
    let base=fights+acts+won;
    base+base*record.law.score().min(300)/200
}
impl Run {
    /// 开局护符：只能在第一幕起点、尚未行动时设置，必须是这个职业的可选护符
    pub fn set_start_talisman(&mut self,talisman:u32)->RuleResult<()> {
        if !talismans(self.class).contains(&talisman) {return Err("开局护符无效".into());}
        if self.act!=1||!self.visited.is_empty()||self.phase!=Phase::Map {return Err("开局护符只能在开局时选择".into());}
        self.start_talisman=talisman;Ok(())
    }
    /// 已经带在身上的开局护符（旧存档为 0，表示随身的祖灵角，不影响奖励）
    pub fn valid_start_talisman(&self)->bool {self.start_talisman==0||talismans(self.class).contains(&self.start_talisman)}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::Fight;
    fn fight(act:u8,kind:&str,won:bool)->Fight {Fight{act,layer:1,kind:kind.into(),won,enemies:vec![]}}
    #[test]
    fn levels_follow_the_curve_and_cap() {
        let mut l=ClassLegacy::default();
        assert_eq!(l.progress(),(0,0,100));
        l.xp=100;assert_eq!(l.progress(),(1,0,125));
        l.xp=100+125+30;assert_eq!(l.progress(),(2,30,150));
        let total:u32=(0..MAX_LEVEL).map(next_xp).sum();
        assert_eq!(total,2125);
        l.xp=total+5000;assert_eq!(l.progress(),(MAX_LEVEL,0,0));
        assert_eq!(UNLOCKS.last(),Some(&MAX_LEVEL));
    }
    #[test]
    fn talismans_unlock_by_level_per_class() {
        for class in Class::ALL {
            let list=talismans(class);
            assert_eq!(list[0],DEFAULT_TALISMAN);
            assert!(list.iter().enumerate().all(|(i,id)|!list[..i].contains(id)),"{class:?} has duplicates");
            assert!(list.iter().all(|id|crate::rewards::lookup(0x20000000|id).is_some()),"{class:?} lists a talisman outside the catalog");
        }
        let mut l=ClassLegacy::default();
        assert_eq!(l.chosen(Class::Samurai),DEFAULT_TALISMAN);
        assert_eq!(l.choose(Class::Samurai,2050),Err("熟练度不足，尚未解锁"));
        l.xp=100;l.choose(Class::Samurai,2050).unwrap();
        assert_eq!((l.chosen(Class::Samurai),l.chosen(Class::Hero)),(2050,DEFAULT_TALISMAN),"another class does not inherit the choice");
        assert_eq!(l.choose(Class::Samurai,2100),Err("熟练度不足，尚未解锁"));
        assert_eq!(l.choose(Class::Samurai,4000),Err("这个职业不能选用这枚护符"));
        l.choose(Class::Samurai,DEFAULT_TALISMAN).unwrap();assert_eq!(l.talisman,0);
    }
    #[test]
    fn classes_level_independently_and_debug_runs_earn_nothing() {
        let mut legacy=Legacy::default();
        let record=Record{class:Class::Samurai,act:2,result:Outcome::Dead,fights:vec![fight(1,"怪物",true),fight(1,"精英",true),fight(1,"Boss",true),fight(2,"怪物",false)],..Default::default()};
        assert_eq!(earned(&record),8+20+50+40);
        let gain=legacy.award(&record).unwrap();
        assert_eq!((gain.xp,gain.before,gain.after,gain.unlocked()),(118,0,1,vec![2050]));
        assert_eq!((legacy.get(Class::Samurai).level(),legacy.get(Class::Hero).level()),(1,0));
        assert_eq!(legacy.award(&Record{debug:true,..record.clone()}),None);
        assert_eq!(Legacy::decode(&legacy.encode()),legacy);
        assert_eq!(Legacy::decode("garbage"),Legacy::default());
        let law=(0..27).fold(crate::law::Law::default(),|l,i|l.toggle(i));
        let won=Record{result:Outcome::Won,act:3,law,fights:vec![],battles:10,..Default::default()};
        assert_eq!(earned(&won),(80+120+150)*5/2);
        assert_eq!(Gain{class:Class::Hero,xp:0,before:2,after:6}.unlocked(),vec![2070,2190]);
    }
    #[test]
    fn start_talisman_is_set_once_at_the_start_and_leaves_the_reward_pool() {
        let mut run=Run::new(42,Class::Samurai);
        assert!(run.set_start_talisman(4000).is_err(),"not a samurai talisman");
        run.set_start_talisman(2050).unwrap();
        assert_eq!(Run::decode(&run.encode().unwrap()).unwrap().start_talisman,2050);
        for node in 0..run.nodes.len() {assert!(!crate::rewards::offers(&run,node).contains(&crate::Reward::Item(0x20000000|2050)));}
        run.phase=Phase::Combat{id:0};
        assert!(run.set_start_talisman(2100).is_err());
    }
}
