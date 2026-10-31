use serde::{Deserialize,Serialize};

/// 余烬：誓约三选一里的机制增益，没有代价，不能深化；选过的整局生效
#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash,Serialize,Deserialize)]
pub enum Ember {Execution,Bloodlord,Frost,Deathflame,Miquella,Rise,DragonHeart,Revenge,Frenzy,Echo,Feast,Thorns,SpiritLink,SpiritBurst,SpiritFeast,SpiritCommand,SpiritTwin,SoulEater,Chain,Godslayer,SpiritKeeper}

// 运行时数值（参数类的数值见 scene-builder/EmberAssets.cs，说明文字同步引用这些常量）
pub const EXECUTION_SECONDS:u32=5;
pub const EXECUTION_STAMINA_CUT:u32=50;
pub const BUILDUP:u32=12;
pub const DEATHFLAME_RADIUS:f32=4.0;
pub const DEATHFLAME_PERCENT:u32=8;
pub const LIFESTEAL_PERCENT:u32=8;
pub const RISE_BELOW_PERCENT:u32=10;
pub const RISE_TO_PERCENT:u32=30;
pub const DRAGON_RESTORE_PERCENT:u32=50;
pub const DRAGON_COOLDOWN_SECONDS:u64=30;
pub const REVENGE_SECONDS:u64=3;
pub const REVENGE_PERCENT:u32=30;
pub const FRENZY_STEP_PERCENT:u32=2;
pub const FRENZY_STACKS:u8=8;
pub const ECHO_RADIUS:f32=4.0;
pub const ECHO_PERCENT:u32=3;
pub const FEAST_PER_FIGHT:u8=1;
pub const THORNS_PERCENT:u32=25;
// 召唤流余烬（只在身上带着骨灰时出现）
pub const SPIRIT_SHARE_PERCENT:u32=30;
pub const SPIRIT_BURST_RADIUS:f32=6.0;
pub const SPIRIT_BURST_PERCENT:u32=8;
pub const SPIRIT_FEAST_HEAL_PERCENT:u32=15;
pub const SPIRIT_FEAST_FP_PERCENT:u32=5;
pub const SPIRIT_COMMAND_PERCENT:u32=20;
// 叠层余烬：层数由战斗中的事件累积，整局保留（Run::ember_stacks），不设上限
/// 噬魂者：击倒普通敌人、精英、Boss 分别得魂点，每满 SOUL_POINTS 点一层
pub const SOUL_POINTS:u32=10;
pub const SOUL_KILL:[u32;3]=[1,3,10];
pub const SOUL_STEP_PERCENT:u32=2;
pub const CHAIN_STEP_PERCENT:u32=6;
pub const SLAYER_HP_PERCENT:u32=4;
pub const SLAYER_ATTACK_PERCENT:u32=2;
pub const KEEPER_ATTACK_PERCENT:u32=3;
/// 灵魂收容的减伤逐层乘算（每层受到的伤害 ×0.98），层数再多也不会到 100%
pub const KEEPER_CUT_PERCENT:u32=2;
/// 直接扣敌人生命的余烬对 Boss 每次最多扣最大生命的 1%
pub const BOSS_CAP_PERCENT:u32=1;

impl Ember {
    pub const ALL:[Self;21]=[Self::Execution,Self::Bloodlord,Self::Frost,Self::Deathflame,Self::Miquella,Self::Rise,Self::DragonHeart,Self::Revenge,Self::Frenzy,Self::Echo,Self::Feast,Self::Thorns,
        Self::SpiritLink,Self::SpiritBurst,Self::SpiritFeast,Self::SpiritCommand,Self::SpiritTwin,Self::SoulEater,Self::Chain,Self::Godslayer,Self::SpiritKeeper];
    pub fn index(self)->usize {Self::ALL.iter().position(|e|*e==self).unwrap()}
    pub fn name(self)->&'static str {match self {
        Self::Execution=>"无声处刑",Self::Bloodlord=>"鲜血君王的赐福",Self::Frost=>"永冻吐息",Self::Deathflame=>"死诞之焰",
        Self::Miquella=>"米凯拉之刃",Self::Rise=>"荷莱·露的再起",Self::DragonHeart=>"古龙之心",Self::Revenge=>"以血还血",
        Self::Frenzy=>"癫火狂热",Self::Echo=>"圣杯余响",Self::Feast=>"噬神之蛇的饱食",Self::Thorns=>"荆棘之躯",
        Self::SpiritLink=>"仿身泪滴的替身",Self::SpiritBurst=>"灵骸迸裂",Self::SpiritFeast=>"众灵的盛宴",Self::SpiritCommand=>"灵唤铃的号令",
        Self::SpiritTwin=>"双生的灵唤铃",Self::SoulEater=>"噬魂者",Self::Chain=>"处刑之链",Self::Godslayer=>"屠神者",Self::SpiritKeeper=>"灵魂收容",
    }}
    pub fn effect(self)->String {match self {
        Self::Execution=>format!("致命一击后 {EXECUTION_SECONDS} 秒内\n精力消耗 -{EXECUTION_STAMINA_CUT}%"),
        Self::Bloodlord=>format!("每次命中积累\n{BUILDUP} 点出血"),
        Self::Frost=>format!("每次命中积累\n{BUILDUP} 点冻伤"),
        Self::Deathflame=>format!("敌人倒下时，对 {DEATHFLAME_RADIUS} 米内其他敌人\n造成其最大生命 {DEATHFLAME_PERCENT}% 的伤害\n对 Boss 最多 {BOSS_CAP_PERCENT}%"),
        Self::Miquella=>format!("造成伤害的 {LIFESTEAL_PERCENT}%\n转为生命"),
        Self::Rise=>format!("每场战斗一次：生命低于 {RISE_BELOW_PERCENT}% 时\n回复到 {RISE_TO_PERCENT}%"),
        Self::DragonHeart=>format!("精力耗尽时回复 {DRAGON_RESTORE_PERCENT}%\n冷却 {DRAGON_COOLDOWN_SECONDS} 秒"),
        Self::Revenge=>format!("受伤后 {REVENGE_SECONDS} 秒内\n下一次命中伤害 +{REVENGE_PERCENT}%"),
        Self::Frenzy=>format!("每次命中攻击 +{FRENZY_STEP_PERCENT}%，最多 +{}%\n受伤时清零",FRENZY_STEP_PERCENT*u32::from(FRENZY_STACKS)),
        Self::Echo=>format!("喝圣杯瓶时，对 {ECHO_RADIUS} 米内敌人\n造成其最大生命 {ECHO_PERCENT}% 的伤害\n对 Boss 最多 {BOSS_CAP_PERCENT}%"),
        Self::Feast=>format!("敌人倒下时补回 1 次红圣杯瓶\n每场战斗最多 {FEAST_PER_FIGHT} 次"),
        Self::Thorns=>format!("受到伤害的 {THORNS_PERCENT}%\n反弹给最近的敌人\n对 Boss 每次最多 {BOSS_CAP_PERCENT}%"),
        Self::SpiritLink=>format!("召唤物在场时\n你受到伤害的 {SPIRIT_SHARE_PERCENT}%\n改由召唤物承受"),
        Self::SpiritBurst=>format!("召唤物倒下或消失时，对 {SPIRIT_BURST_RADIUS} 米内敌人\n造成其最大生命 {SPIRIT_BURST_PERCENT}% 的伤害\n对 Boss 最多 {BOSS_CAP_PERCENT}%"),
        Self::SpiritFeast=>format!("敌人倒下时\n召唤物回复 {SPIRIT_FEAST_HEAL_PERCENT}% 生命\n你回复 {SPIRIT_FEAST_FP_PERCENT}% 专注"),
        Self::SpiritCommand=>format!("召唤物造成的伤害 +{SPIRIT_COMMAND_PERCENT}%\n受到的伤害 -{SPIRIT_COMMAND_PERCENT}%"),
        Self::SpiritTwin=>"每场战斗可以再召唤一次骨灰\n两只召唤物同时在场\n（第二个骨灰照常消耗）".into(),
        Self::SoulEater=>format!("击倒得魂：小怪{} 精英{} Boss{}\n每 {SOUL_POINTS} 魂一层，每层攻击 +{SOUL_STEP_PERCENT}%\n层数不设上限，整局保留",SOUL_KILL[0],SOUL_KILL[1],SOUL_KILL[2]),
        Self::Chain=>format!("每次致命一击叠一层\n每层致命一击伤害 +{CHAIN_STEP_PERCENT}%\n层数不设上限，整局保留"),
        Self::Godslayer=>format!("每赢一场精英战或 Boss 战叠一层\n每层最大生命 +{SLAYER_HP_PERCENT}%、攻击 +{SLAYER_ATTACK_PERCENT}%\n层数不设上限，整局保留"),
        Self::SpiritKeeper=>format!("召唤物在场时击倒敌人叠一层\n每层召唤物伤害 +{KEEPER_ATTACK_PERCENT}%\n受到的伤害 -{KEEPER_CUT_PERCENT}%（逐层乘算）\n层数不设上限，整局保留"),
    }}
    /// 品质：余烬只有史诗（2）与传说（3），按强度定，决定抽到的概率与奖励卡的颜色
    pub fn quality(self)->u8 {match self {
        Self::Rise|Self::SpiritTwin|Self::Miquella|Self::Feast|Self::Godslayer=>3,
        _=>2,
    }}
    /// 只影响敌人的余烬：联机客机上不生效（敌人由房主计算）
    /// 叠层余烬（层数由战斗中的事件累积，不设上限）
    pub fn stacking(self)->bool {matches!(self,Self::SoulEater|Self::Chain|Self::Godslayer|Self::SpiritKeeper)}
    /// 当前层数的效果，显示在构筑面板里
    pub fn stack_bonus(self,stacks:u32)->Option<String> {
        let r=stack_rates(self,stacks)?;
        let percent=|rate:f32|((rate-1.0)*100.0).round() as i64;
        Some(match self {
            Self::SoulEater=>format!("攻击 +{}%",percent(r.attack)),
            Self::Chain=>format!("致命一击伤害 +{}%",percent(r.attack)),
            Self::Godslayer=>format!("最大生命 +{}%、攻击 +{}%",percent(r.hp),percent(r.attack)),
            Self::SpiritKeeper=>format!("召唤物伤害 +{}%、受到的伤害 -{}%",percent(r.attack),-percent(r.cut)),
            _=>return None,
        })
    }
    pub fn affects_enemies(self)->bool {matches!(self,Self::Bloodlord|Self::Frost|Self::Deathflame|Self::Echo|Self::Thorns|Self::SpiritBurst)}
    /// 召唤流余烬：只有身上带着骨灰时才会出现在奖励里
    pub fn needs_spirit(self)->bool {matches!(self,Self::SpiritLink|Self::SpiritBurst|Self::SpiritFeast|Self::SpiritCommand|Self::SpiritTwin|Self::SpiritKeeper)}
}
/// 作用在敌人身上的余烬效果。联机成员在本机算出（只算自己与自己召唤物的命中、击倒），随汇报发给房主，由房主执行。
/// slot：这场战斗敌人名单（enemies::node_roster）里的位置，房主与成员一致
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub enum Strike {
    /// 命中：鲜血君王的赐福、永冻吐息的异常积累
    Buildup{slot:u8,ember:Ember},
    /// 按目标最大生命的百分比扣血（Boss 有上限）：死诞之焰、圣杯余响、灵骸迸裂
    Wound{slot:u8,ember:Ember},
    /// 荆棘之躯：按成员受到的伤害反弹
    Thorns{slot:u8,taken:i32},
}
/// 荆棘之躯一次反弹按受到的伤害计，超过这个数视为无效请求
pub const THORNS_TAKEN_LIMIT:i32=100_000;
/// 汇报里最多带的敌人侧效果条数（最近的若干条；房主按序号执行没执行过的）
pub const STRIKE_WINDOW:usize=32;
impl Strike {
    pub fn slot(self)->u8 {match self {Self::Buildup{slot,..}|Self::Wound{slot,..}|Self::Thorns{slot,..}=>slot}}
    pub fn ember(self)->Ember {match self {Self::Buildup{ember,..}|Self::Wound{ember,..}=>ember,Self::Thorns{..}=>Ember::Thorns}}
    /// 成员确实带着这个余烬，且效果类型与余烬对得上
    pub fn valid(self,held:&[Ember])->bool {
        held.contains(&self.ember())&&match self {
            Self::Buildup{ember,..}=>matches!(ember,Ember::Bloodlord|Ember::Frost),
            Self::Wound{ember,..}=>wound_percent(ember).is_some(),
            Self::Thorns{taken,..}=>(1..=THORNS_TAKEN_LIMIT).contains(&taken),
        }
    }
}
/// 按目标最大生命百分比扣血的余烬
pub fn wound_percent(ember:Ember)->Option<u32> {match ember {
    Ember::Deathflame=>Some(DEATHFLAME_PERCENT),Ember::Echo=>Some(ECHO_PERCENT),Ember::SpiritBurst=>Some(SPIRIT_BURST_PERCENT),_=>None,
}}
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct StrikeRequest {pub seq:u32,pub strike:Strike}
/// 战斗中让叠层余烬加层的事件
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Gain {
    /// 击倒敌人：rank 0 普通、1 精英、2 Boss；spirit 为召唤物当时在场
    Kill{rank:u8,spirit:bool},
    Critical,
    /// 赢下一场精英战或 Boss 战
    Slain,
}
/// 叠层余烬在某个层数下的倍率（攻击倍率、最大生命倍率、受到伤害的倍率），运行时写进效果参数
#[derive(Clone,Copy,Debug,PartialEq)]
pub struct Rates {pub attack:f32,pub hp:f32,pub cut:f32}
pub fn stack_rates(ember:Ember,stacks:u32)->Option<Rates> {
    let step=|percent:u32|1.0+(percent as f32)*(stacks as f32)/100.0;
    let plain=Rates{attack:1.0,hp:1.0,cut:1.0};
    Some(match ember {
        Ember::SoulEater=>Rates{attack:step(SOUL_STEP_PERCENT),..plain},
        Ember::Chain=>Rates{attack:step(CHAIN_STEP_PERCENT),..plain},
        Ember::Godslayer=>Rates{attack:step(SLAYER_ATTACK_PERCENT),hp:step(SLAYER_HP_PERCENT),..plain},
        Ember::SpiritKeeper=>Rates{attack:step(KEEPER_ATTACK_PERCENT),cut:(1.0-KEEPER_CUT_PERCENT as f32/100.0).powi(stacks.min(10_000) as i32),..plain},
        _=>return None,
    })
}
/// 叠层余烬的进度，整局保留；只记拿到余烬之后的事件
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
pub struct Stacks {
    /// 噬魂者的魂点（层数 = 魂点 / SOUL_POINTS）
    #[serde(default)] pub souls:u32,
    #[serde(default)] pub chain:u32,
    #[serde(default)] pub slayer:u32,
    #[serde(default)] pub keeper:u32,
}
impl Stacks {
    pub fn is_empty(&self)->bool {*self==Self::default()}
    pub fn level(&self,ember:Ember)->u32 {match ember {
        Ember::SoulEater=>self.souls/SOUL_POINTS,Ember::Chain=>self.chain,Ember::Godslayer=>self.slayer,Ember::SpiritKeeper=>self.keeper,_=>0,
    }}
    /// 记一次事件；返回因此升层的余烬（用于提示）
    pub fn gain(&mut self,held:&[Ember],event:Gain)->Vec<Ember> {
        let before=*self;
        let has=|e|held.contains(&e);
        match event {
            Gain::Kill{rank,spirit}=>{
                if has(Ember::SoulEater) {self.souls=self.souls.saturating_add(SOUL_KILL[usize::from(rank.min(2))]);}
                if has(Ember::SpiritKeeper)&&spirit {self.keeper=self.keeper.saturating_add(1);}
            },
            Gain::Critical=>if has(Ember::Chain) {self.chain=self.chain.saturating_add(1);},
            Gain::Slain=>if has(Ember::Godslayer) {self.slayer=self.slayer.saturating_add(1);},
        }
        held.iter().copied().filter(|e|self.level(*e)>before.level(*e)).collect()
    }
}
/// 按种子抽一张没拿过的余烬；spirits 为 false 时不出召唤流余烬
pub fn offer(seed:u64,held:&[Ember],spirits:bool)->Option<Ember> {
    let pool:Vec<_>=Ember::ALL.into_iter().filter(|e|!held.contains(e)&&(spirits||!e.needs_spirit())).collect();
    (!pool.is_empty()).then(||pool[(crate::mix(seed^0xe3be_12a5) % pool.len() as u64) as usize])
}
/// 余烬规则 1 起按品质抽：先按 odds（普通、稀有、史诗、传说，合计 100）掷品质，再在这个品质里挑没拿过的；
/// 这个品质已经没有可抽的，就依次往低一档、再往高一档找
pub fn offer_by_quality(seed:u64,held:&[Ember],spirits:bool,odds:[u32;4])->Option<Ember> {
    let pool:Vec<_>=Ember::ALL.into_iter().filter(|e|!held.contains(e)&&(spirits||!e.needs_spirit())).collect();
    if pool.is_empty() {return None;}
    let roll=(crate::mix(seed^0x51a1_17e5)%100) as u32;
    let mut below=0;let rolled=odds.iter().position(|chance|{below+=chance;roll<below}).unwrap_or(3) as u8;
    let order=(0..=rolled).rev().chain(rolled+1..4);
    for tier in order {
        let candidates:Vec<_>=pool.iter().copied().filter(|e|e.quality()==tier).collect();
        if !candidates.is_empty() {return Some(candidates[(crate::mix(seed^0xe3be_12a5) % candidates.len() as u64) as usize]);}
    }
    None
}
/// 余烬的品质概率：只有史诗与传说，传说的机会按装备品质概率里“传说 : 史诗”的比例换算（第一幕开头约 15%，第三幕末约 28%）
pub fn odds(items:[u32;4])->[u32;4] {
    let legendary=(items[3]*100+(items[2]+items[3])/2)/(items[2]+items[3]).max(1);
    [0,0,100-legendary,legendary]
}
/// 普通战奖励有 1/4 的概率把最后一个奖励换成余烬，精英战必定有一张；Boss 与宝箱不出余烬
pub const MONSTER_CHANCE:u64=4;
pub fn reward(run:&crate::Run,node:usize)->Option<Ember> {
    if !run.ember_rewards {return None;}
    let roll=crate::mix(run.reward_seed()^(node as u64).wrapping_mul(7919)^0xe3be_12a5);
    match run.nodes.get(node)?.kind {
        crate::NodeKind::Elite=>{},
        crate::NodeKind::Monster if roll%MONSTER_CHANCE==0=>{},
        _=>return None,
    }
    // 规则 0（旧存档）不分品质，等概率
    if run.ember_rules>=1 {offer_by_quality(roll>>8,&run.embers,carries_spirit(run),odds(crate::affixes::quality_odds(run)))}
    else {offer(roll>>8,&run.embers,carries_spirit(run))}
}
/// 身上带着骨灰：召唤师（开局就有骨灰），或拿到过任何骨灰奖励
pub fn carries_spirit(run:&crate::Run)->bool {
    run.summoner()||run.claimed_items.iter().any(|id|crate::rewards::lookup(*id).is_some_and(|i|i.kind==crate::rewards::ItemKind::Spirit))
}
/// 借用原版护符的图标
pub fn icon_talisman(ember:Ember)->u32 {match ember {
    Ember::Execution=>5050,Ember::Bloodlord=>2160,Ember::Frost=>1140,Ember::Deathflame=>1190,Ember::Miquella=>1250,Ember::Rise=>3090,
    Ember::DragonHeart=>4020,Ember::Revenge=>2060,Ember::Frenzy=>2080,Ember::Echo=>5000,Ember::Feast=>5030,Ember::Thorns=>1210,
    Ember::SpiritLink=>6090,Ember::SpiritBurst=>6020,Ember::SpiritFeast=>2210,Ember::SpiritCommand=>3060,Ember::SpiritTwin=>6090,
    Ember::SoulEater=>2050,Ember::Chain=>2090,Ember::Godslayer=>1001,Ember::SpiritKeeper=>2210,
}}
/// 扣敌人生命的量：按目标最大生命的百分比，Boss 另有上限
pub fn enemy_damage(max_hp:i32,percent:u32,boss:bool)->i32 {
    let percent=if boss {percent.min(BOSS_CAP_PERCENT)}else{percent};
    (i64::from(max_hp.max(0))*i64::from(percent)/100) as i32
}
/// 荆棘：按受到的伤害反弹，对 Boss 不超过其最大生命的 1%
pub fn thorns_damage(taken:i32,target_max:i32,boss:bool)->i32 {
    let damage=(i64::from(taken.max(0))*i64::from(THORNS_PERCENT)/100) as i32;
    if boss {damage.min(enemy_damage(target_max,BOSS_CAP_PERCENT,true))}else{damage}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_quality_has_embers_and_rolls_follow_the_odds() {
        assert!(Ember::ALL.iter().all(|e|matches!(e.quality(),2|3)),"embers are only epic or legendary");
        assert_eq!(Ember::ALL.iter().filter(|e|e.quality()==3).count(),5);
        assert_eq!(odds(crate::affixes::QUALITY_START),[0,0,85,15]);assert_eq!(odds(crate::affixes::QUALITY_END),[0,0,72,28]);
        let mut counts=[0u32;4];
        for seed in 0..20000u64 {counts[usize::from(offer_by_quality(seed,&[],true,[0,0,50,50]).unwrap().quality())]+=1;}
        assert!(counts[0]==0&&counts[1]==0&&(9000..11000).contains(&counts[2])&&(9000..11000).contains(&counts[3]),"{counts:?}");
        let mut late=[0u32;4];
        for seed in 0..20000u64 {late[usize::from(offer_by_quality(seed,&[],true,[0,0,0,100]).unwrap().quality())]+=1;}
        assert_eq!(late[3],20000,"legendary odds give legendary embers while any are left");
        // 这一档拿完了：先往低一档找
        let held:Vec<_>=Ember::ALL.into_iter().filter(|e|e.quality()==3).collect();
        assert!((0..500).all(|seed|offer_by_quality(seed,&held,true,[0,0,0,100]).unwrap().quality()==2));
        let mut all=vec![];while let Some(e)=offer_by_quality(all.len() as u64*13,&all,true,[0,0,85,15]) {all.push(e);}
        assert_eq!(all.len(),Ember::ALL.len());
    }
    #[test]
    fn offers_skip_held_embers_and_run_out_cleanly() {
        let mut held=vec![];
        for _ in 0..Ember::ALL.len() {let e=offer(held.len() as u64*31,&held,true).unwrap();assert!(!held.contains(&e));held.push(e);}
        assert_eq!(offer(1,&held,true),None);
        assert_eq!(offer(5,&[],true),offer(5,&[],true));
        // 没有骨灰时只出 15 枚通用余烬，抽完就没有了
        let mut plain=vec![];
        while let Some(e)=offer(plain.len() as u64*17,&plain,false) {assert!(!e.needs_spirit());plain.push(e);}
        assert_eq!(plain.len(),15);
    }
    #[test]
    fn enemy_damage_is_capped_for_bosses() {
        assert_eq!(enemy_damage(10000,DEATHFLAME_PERCENT,false),800);
        assert_eq!(enemy_damage(10000,DEATHFLAME_PERCENT,true),100);
        assert_eq!(thorns_damage(400,10000,false),100);assert_eq!(thorns_damage(4000,10000,true),100);
        assert_eq!(enemy_damage(-5,ECHO_PERCENT,false),0);
    }
    #[test]
    fn member_strikes_need_the_matching_ember() {
        let held=[Ember::Bloodlord,Ember::Deathflame,Ember::Thorns];
        assert!(Strike::Buildup{slot:0,ember:Ember::Bloodlord}.valid(&held));
        assert!(!Strike::Buildup{slot:0,ember:Ember::Frost}.valid(&held),"frost not held");
        assert!(!Strike::Buildup{slot:0,ember:Ember::Deathflame}.valid(&held),"not a buildup ember");
        assert!(Strike::Wound{slot:1,ember:Ember::Deathflame}.valid(&held));
        assert!(!Strike::Wound{slot:1,ember:Ember::Bloodlord}.valid(&held),"bloodlord does not wound");
        assert!(!Strike::Wound{slot:1,ember:Ember::Echo}.valid(&held));
        assert!(Strike::Thorns{slot:2,taken:300}.valid(&held));
        assert!(!Strike::Thorns{slot:2,taken:0}.valid(&held)&&!Strike::Thorns{slot:2,taken:THORNS_TAKEN_LIMIT+1}.valid(&held));
        assert!(Ember::ALL.into_iter().filter(|e|e.affects_enemies()).all(|e|wound_percent(e).is_some()||matches!(e,Ember::Bloodlord|Ember::Frost|Ember::Thorns)),"every enemy-side ember can be requested");
    }
    #[test]
    fn embers_show_up_in_combat_rewards_and_persist() {
        use crate::{Class,NodeKind,Phase,Reward,Run};
        let (mut elites,mut monsters,mut monster_embers)=(0,0,0);
        for seed in 0..400 {
            let mut run=Run::new(seed,Class::Samurai);
            for (id,node) in run.nodes.clone().iter().enumerate() {
                let ember=reward(&run,id);
                match node.kind {
                    NodeKind::Elite=>{elites+=1;assert!(ember.is_some());},
                    NodeKind::Monster=>{monsters+=1;if ember.is_some(){monster_embers+=1;}},
                    _=>assert!(ember.is_none()),
                }
            }
            let id=run.available()[0];run.enter(id).unwrap();run.win_combat().unwrap();
            let Phase::Reward{offers,..}=run.phase.clone() else{panic!()};
            assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
            if let Some(index)=offers.iter().position(|r|matches!(r,Reward::Ember(_))) {
                let Reward::Ember(ember)=offers[index] else{unreachable!()};
                assert_eq!(index,offers.len()-1);
                run.claim_reward(index).unwrap();assert_eq!(run.embers,vec![ember]);
                assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
                for later in 0..run.nodes.len() {assert_ne!(reward(&run,later),Some(ember),"held embers never repeat");}
                let mut twice=run.clone();twice.embers.push(ember);assert!(twice.validate().is_err());
            }
        }
        assert!(elites>0);
        let rate=monster_embers as f64/monsters as f64;assert!((0.18..0.32).contains(&rate),"monster ember rate {rate}");
        let mut old=Run::new(3,Class::Samurai);old.ember_rewards=false;
        assert!((0..old.nodes.len()).all(|id|reward(&old,id).is_none()));
        let mut json:serde_json::Value=serde_json::from_str(&Run::new(3,Class::Samurai).encode().unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("ember_rewards");
        assert!(!Run::decode(&json.to_string()).unwrap().ember_rewards,"saves from before embers keep their reward pages");
    }
    #[test]
    fn summon_embers_need_a_spirit_ash() {
        use crate::{Class,Run};
        let (mut plain,mut summoner)=(0,0);
        for seed in 0..300 {
            let mut run=Run::new(seed,Class::Samurai);
            for id in 0..run.nodes.len() {if reward(&run,id).is_some_and(Ember::needs_spirit) {plain+=1;}}
            run.claimed_items.push(0x40000000|232000);// 离群野狼的骨灰
            assert!(carries_spirit(&run));
            for id in 0..run.nodes.len() {if reward(&run,id).is_some_and(Ember::needs_spirit) {summoner+=1;}}
        }
        assert_eq!(plain,0,"no summon embers without a spirit ash");
        assert!(summoner>0);
    }
    #[test]
    fn stacks_grow_only_from_their_own_events_without_a_cap() {
        let mut stacks=Stacks::default();
        assert!(stacks.gain(&[Ember::Frenzy],Gain::Critical).is_empty());assert!(stacks.is_empty(),"no stacking ember held");
        let held=[Ember::SoulEater,Ember::Chain,Ember::Godslayer,Ember::SpiritKeeper];
        for _ in 0..9 {assert!(stacks.gain(&held,Gain::Kill{rank:0,spirit:false}).is_empty());}
        assert_eq!(stacks.gain(&held,Gain::Kill{rank:0,spirit:false}),vec![Ember::SoulEater]);
        assert_eq!(stacks.gain(&held,Gain::Kill{rank:2,spirit:true}),vec![Ember::SoulEater,Ember::SpiritKeeper]);
        assert_eq!((stacks.level(Ember::SoulEater),stacks.level(Ember::SpiritKeeper)),(2,1));
        assert_eq!(stacks.gain(&held,Gain::Kill{rank:1,spirit:false}),vec![],"3 souls: 23 in total");
        assert_eq!(stacks.gain(&held,Gain::Critical),vec![Ember::Chain]);
        assert_eq!(stacks.gain(&held,Gain::Slain),vec![Ember::Godslayer]);
        for _ in 0..100 {for event in [Gain::Kill{rank:2,spirit:true},Gain::Critical,Gain::Slain] {stacks.gain(&held,event);}}
        assert_eq!(held.map(|e|stacks.level(e)),[102,101,101,101],"no cap");
        assert_eq!(stacks.gain(&held,Gain::Critical),vec![Ember::Chain]);
        assert!(Ember::ALL.iter().all(|e|e.stacking()==e.stack_bonus(1).is_some()));
        assert_eq!(Ember::Godslayer.stack_bonus(8).unwrap(),"最大生命 +32%、攻击 +16%");
        assert_eq!(Ember::Chain.stack_bonus(50).unwrap(),"致命一击伤害 +300%");
        assert_eq!(Ember::SpiritKeeper.stack_bonus(10).unwrap(),"召唤物伤害 +30%、受到的伤害 -18%");
        let deep=stack_rates(Ember::SpiritKeeper,500).unwrap();assert!(deep.cut>0.0&&deep.cut<0.001,"damage cut never reaches zero");
        assert_eq!(stack_rates(Ember::SoulEater,0).unwrap(),Rates{attack:1.0,hp:1.0,cut:1.0});
        let mut full=Stacks{chain:u32::MAX,..Default::default()};assert!(full.gain(&held,Gain::Critical).is_empty(),"saturates instead of overflowing");
    }
    #[test]
    fn card_text_uses_the_tuned_numbers() {
        assert_eq!(Ember::Execution.effect(),"致命一击后 5 秒内\n精力消耗 -50%");
        assert_eq!(Ember::Frenzy.effect(),"每次命中攻击 +2%，最多 +16%\n受伤时清零");
        assert!(Ember::ALL.iter().all(|e|e.effect().lines().all(|l|l.chars().count()<=20)),"card lines must fit");
    }
}
