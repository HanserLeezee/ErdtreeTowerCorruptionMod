use serde::{Deserialize,Serialize};
use crate::{Class,RuleResult};

#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub enum Kind {Steel,Breaker,Combo,Guard,Mystic,Hunter,Execution,Feather,Blood,Ascetic,
    // [HansFix6] 新誓约：灵辉（属性↑物理↓）、锻铁（物理↑属性↓）、磐石（减伤）、贪婪（卢恩）、疫血（异常触发）
    Radiant,Iron,Bulwark,Greed,Plague,
    // [HansFix] 吊命之誓：每局首次致命伤害保留 1 点生命，短时无敌并减少精力消耗；喝下圣杯灵药后重置
    Lifeline}
impl Kind {
    pub const ALL:[Self;16]=[Self::Steel,Self::Breaker,Self::Combo,Self::Guard,Self::Mystic,Self::Hunter,Self::Execution,Self::Feather,Self::Blood,Self::Ascetic,
        Self::Radiant,Self::Iron,Self::Bulwark,Self::Greed,Self::Plague,Self::Lifeline];
    /// 旧规则（每幕一份）只用最初的十种，旧存档与旧参数不受影响
    pub const LEGACY:[Self;10]=[Self::Steel,Self::Breaker,Self::Combo,Self::Guard,Self::Mystic,Self::Hunter,Self::Execution,Self::Feather,Self::Blood,Self::Ascetic];
    /// 卡面插画：0–5 为原图集，6–11 为新誓约封面图集（磐石、吊命、贪婪、灵辉、锻铁、疫血）
    pub fn art(self)->usize {match self {Self::Bulwark=>6,Self::Lifeline=>7,Self::Greed=>8,Self::Radiant=>9,Self::Iron=>10,Self::Plague=>11,other=>other.index()%6}}
    pub fn name(self)->&'static str {match self {
        Self::Steel=>"蓄力之誓",Self::Breaker=>"凌空之誓",Self::Combo=>"连击之誓",
        Self::Guard=>"坚盾之誓",Self::Mystic=>"秘法之誓",Self::Hunter=>"猎手之誓",
        Self::Execution=>"处刑之誓",Self::Feather=>"轻羽之誓",Self::Blood=>"血契之誓",Self::Ascetic=>"苦修之誓",
        Self::Radiant=>"灵辉之誓",Self::Iron=>"锻铁之誓",Self::Bulwark=>"磐石之誓",Self::Greed=>"贪婪之誓",Self::Plague=>"疫血之誓",Self::Lifeline=>"吊命之誓",
    }}
    pub fn boon(self)->&'static str {match self {
        Self::Steel=>"蓄力重攻击伤害 +20%",Self::Breaker=>"跳跃攻击伤害 +25%",
        Self::Combo=>"连段末击伤害 +30%",Self::Guard=>"防御反击伤害 +30%",
        Self::Mystic=>"魔法／祷告伤害 +20%",Self::Hunter=>"弓弩伤害 +30%",
        Self::Execution=>"致命一击伤害 +30%",
        Self::Feather=>"轻负重时伤害 +15%\n精力恢复 +5 点/秒",
        Self::Blood=>"生命低于 50%\n伤害 +25%",
        Self::Ascetic=>"胜利恢复最大生命 10%\n最大专注 15%",
        Self::Radiant|Self::Iron|Self::Bulwark|Self::Greed|Self::Plague|Self::Lifeline=>"",
    }}
    pub fn bane(self)->&'static str {match self {
        Self::Steel|Self::Hunter=>"战技伤害 -20%",Self::Breaker=>"战技专注消耗 +25%",
        Self::Combo=>"受到伤害 +10%",Self::Guard=>"动作精力消耗 +20%",
        Self::Mystic=>"法术专注消耗 +20%",
        Self::Execution=>"非致命攻击伤害 -10%",
        Self::Feather=>"受到物理伤害 +10%",
        Self::Blood=>"红圣杯瓶恢复量 -20%",
        Self::Ascetic=>"红、蓝圣杯瓶恢复量 -35%",
        Self::Radiant=>"物理伤害 -15%",Self::Iron=>"魔力／火／雷／圣伤害 -25%",
        Self::Bulwark=>"最大专注 -20%",Self::Greed=>"受到伤害 +10%",
        Self::Plague=>"出血、毒、腐败抗性 -30%",
        Self::Lifeline=>"最大生命 -10%",
    }}
    pub fn hint(self)->&'static str {match self {
        Self::Steel=>"抓住敌人硬直，以蓄力重击换取爆发。",
        Self::Breaker=>"以跳跃攻击接近敌人，减少使用战技。",
        Self::Combo=>"完整打出连段才获得增益；普通连击中途没有加成。",
        Self::Guard=>"格挡后重击反击，注意为后续闪避保留精力。",
        Self::Mystic=>"适配装备附送基础攻击法术；自行装备并记忆。",
        Self::Hunter=>"适配装备附送 30 支对应箭矢或弩箭。",
        Self::Execution=>"以背刺和弹反处决敌人。",
        Self::Feather=>"装备重量低于最大负重的 30% 时获得增益。",
        Self::Blood=>"以低血量换取更高伤害。",
        Self::Ascetic=>"胜利恢复按最大生命和专注计算，不超过上限。",
        Self::Radiant=>"魔力、火、雷、圣属性伤害提高，物理伤害降低；适合法术与属性武器。",
        Self::Iron=>"物理伤害提高，所有属性伤害降低；适合纯物理武器。",
        Self::Bulwark=>"受到的所有伤害降低，代价是最大专注降低。",
        Self::Greed=>"击败敌人获得的卢恩提高；首领额外卢恩不受影响。",
        Self::Plague=>"附近发生出血、中毒或腐败时，20 秒内伤害提高。",
        Self::Lifeline=>"每局第一次受到致命伤害时生效；喝下圣杯灵药后可再次生效。",
    }}
    pub fn index(self)->usize {Self::ALL.iter().position(|k|*k==self).unwrap()}
    /// 新规则下各等级的增益（1–3 级）
    pub fn boon_at(self,level:u8)->String {
        let l=usize::from(level.clamp(1,3)-1);
        let narrow=[35,55,80][l];
        match self {
            Self::Steel=>format!("蓄力重攻击伤害 +{narrow}%"),Self::Breaker=>format!("跳跃攻击伤害 +{narrow}%"),
            Self::Combo=>format!("连段末击伤害 +{narrow}%"),Self::Guard=>format!("防御反击伤害 +{narrow}%"),
            Self::Hunter=>format!("弓弩伤害 +{narrow}%"),Self::Execution=>format!("致命一击伤害 +{narrow}%"),
            Self::Mystic=>format!("魔法／祷告伤害 +{}%",[25,40,55][l]),
            Self::Feather=>format!("轻负重时伤害 +{}%\n精力恢复 +{} 点/秒",[20,30,40][l],[5,7,10][l]),
            Self::Blood=>format!("生命低于 50%\n伤害 +{}%",[35,55,75][l]),
            Self::Ascetic=>{let (hp,fp)=ASCETIC[l];format!("胜利恢复最大生命 {hp}%\n最大专注 {fp}%")},
            Self::Radiant=>format!("魔力／火／雷／圣伤害 +{}%",[25,40,55][l]),
            Self::Iron=>format!("物理伤害 +{}%",[15,25,35][l]),
            Self::Bulwark=>format!("受到伤害 -{}%",[10,15,20][l]),
            Self::Greed=>format!("卢恩获取 +{}%",[25,40,60][l]),
            Self::Plague=>format!("附近出血／中毒／腐败时\n20 秒内伤害 +{}%",[15,22,30][l]),
            Self::Lifeline=>format!("首次致命伤害时保留 1 点生命\n{0} 秒无敌，精力消耗 -90%",LIFELINE_SECONDS[l]),
        }
    }
    /// 新规则下的代价：不随等级加重
    pub fn bane_at(self)->&'static str {match self {
        Self::Combo=>"受到伤害 +5%",Self::Guard=>"动作精力消耗 +10%",_=>self.bane(),
    }}
}
/// 苦修之誓各等级的胜利恢复（最大生命 %、最大专注 %）
pub const ASCETIC:[(u8,u8);3]=[(15,20),(20,25),(25,30)];
pub const MAX_LEVEL:u8=3;
/// 吊命之誓各等级的无敌与减耗时长（秒）
pub const LIFELINE_SECONDS:[u8;3]=[10,15,20];
/// 新规则每幕的新誓约数量（新誓约补丁开启时）
pub const OFFERS:usize=3;
/// [HansFix] 新誓约的效果行是否在 regulation 里（堕落模式补丁带的 regulation 才有）。核心库默认开启；DLL 按补丁标记设置。
static EXTENDED:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(true);
pub fn set_extended(on:bool) {EXTENDED.store(on,std::sync::atomic::Ordering::Relaxed);}
pub fn extended()->bool {EXTENDED.load(std::sync::atomic::Ordering::Relaxed)}
/// 当前可出现的誓约
/// 每幕誓约界面的誓约池：始终与官方相同（新誓约只在堕落首领处出现，见 fallen_oaths）
pub fn pool()->&'static [Kind] {&Kind::LEGACY}
fn offer_count()->usize {2}
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct Oath {pub kind:Kind,pub level:u8}
/// 誓约界面上的一张卡
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Card {Oath(Kind),Deepen(Kind)}

#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct Gift {pub index:usize,pub delivered:[bool;2]}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct State {
    /// 旧规则与旧存档两张，新规则三张（存档为数组，长度兼容）
    pub act:u8,pub offers:Vec<Kind>,pub chosen:Option<Kind>,pub gear:[u32;3],
    pub gear_done:bool,pub peak:u8,pub gift:Option<Gift>,
    /// 0：旧规则（每幕一份、下一幕作废）；1：誓约整局保留，可以深化
    #[serde(default)] pub rules:u8,
    /// 新规则下本局保留的誓约，含本幕刚立下或深化的
    #[serde(default,skip_serializing_if="Vec::is_empty")] pub held:Vec<Oath>,
    /// 第三张卡：深化已有的誓约
    #[serde(default,skip_serializing_if="Option::is_none")] pub deepen:Option<Kind>,
    /// 本幕选择了深化：不送装备，改送一个圣杯滴露
    #[serde(default,skip_serializing_if="std::ops::Not::not")] pub deepened:bool,
}
/// 每幕第一张誓约卡的职业倾向
fn preferred(class:Class,roll:u64)->Kind {
    let kinds:&[Kind]=match class {
        Class::Vagabond=>&[Kind::Guard,Kind::Steel],
        Class::Hero=>&[Kind::Breaker,Kind::Blood],
        Class::Samurai=>&[Kind::Execution,Kind::Combo],
        Class::Astrologer=>&[Kind::Mystic],
        Class::Prophet=>&[Kind::Mystic,Kind::Guard],
        Class::Confessor=>&[Kind::Mystic,Kind::Hunter],
    };
    kinds[roll as usize%kinds.len()]
}
impl State {
    pub fn new(seed:u64,class:Class,act:u8,previous:Option<Kind>,peak:u8)->Self {
        let roll=crate::mix(seed^0xc071ac7);
        let preferred=if class.sorcerer()||class.cleric(){Kind::Mystic}else{
            [Kind::Steel,Kind::Breaker,Kind::Combo,Kind::Guard][roll as usize%4]
        };
        let mut pool:Vec<_>=Kind::LEGACY.into_iter().filter(|k|Some(*k)!=previous).collect();
        pool.sort_by_key(|k|crate::mix(roll^k.index() as u64));
        let first=if Some(preferred)!=previous{preferred}else{pool[0]};
        let second=*pool.iter().find(|k|**k!=first).unwrap();
        Self{act,offers:vec![first,second],chosen:None,gear:[0;3],gear_done:false,peak:peak.min(50),gift:None,rules:0,held:vec![],deepen:None,deepened:false}
    }
    /// 新规则：三份新誓约（不与已有的重复），已有誓约未满级时加一张深化卡。
    /// [HansFix] 首张按职业倾向，其余随机。
    pub fn start(seed:u64,class:Class,act:u8,held:Vec<Oath>,peak:u8)->Self {
        let roll=crate::mix(seed^0xc071ac7);
        let preferred=preferred(class,roll);
        let mut pool:Vec<_>=pool().iter().copied().filter(|k|!held.iter().any(|o|o.kind==*k)).collect();
        pool.sort_by_key(|k|crate::mix(roll^k.index() as u64));
        // v99：首张仍按职业倾向（不在池中时随机）；其余两张随机
        if let Some(at)=pool.iter().position(|k|*k==preferred) {let first=pool.remove(at);pool.insert(0,first);}
        let offers:Vec<Kind>=pool.into_iter().take(offer_count()).collect();
        let deepen=held.iter().rev().filter(|o|o.level<MAX_LEVEL).max_by_key(|o|o.level).map(|o|o.kind);
        Self{act,offers,chosen:None,gear:[0;3],gear_done:false,peak:peak.min(50),gift:None,rules:1,held,deepen,deepened:false}
    }
    /// 进入下一幕：旧规则换一份新契约，新规则保留已有誓约
    pub fn next_act(&self,seed:u64,class:Class,act:u8)->Self {
        if self.rules==0 {Self::new(seed,class,act,self.chosen,self.peak)}else{Self::start(seed,class,act,self.held.clone(),self.peak)}
    }
    /// 两份新誓约；新规则下已有誓约未满级时加一张深化卡
    pub fn cards(&self)->Vec<Card> {
        let mut cards:Vec<_>=self.offers.iter().map(|k|Card::Oath(*k)).collect();
        if self.rules==1 {if let Some(kind)=self.deepen {cards.push(Card::Deepen(kind));}}
        cards
    }
    pub fn options(&self)->usize {self.cards().len()}
    /// 当前生效的誓约及等级；旧规则返回等级 0，表示沿用旧数值
    pub fn active(&self)->Vec<Oath> {
        if self.rules==0 {self.chosen.map(|kind|Oath{kind,level:0}).into_iter().collect()}else{self.held.clone()}
    }
    pub fn level(&self,kind:Kind)->Option<u8> {self.active().into_iter().find(|o|o.kind==kind).map(|o|o.level)}
    pub fn finish_deepen(&mut self)->RuleResult<()> {
        if !self.deepened||self.gear_done {return Err("当前没有待完成的深化".into());}
        self.gear_done=true;Ok(())
    }
    pub fn selection_pending(&self)->bool {self.chosen.is_none()&&!self.gear_done}
    pub fn pending(&self)->bool {!self.gear_done}
    pub fn skip(&mut self)->RuleResult<()> {
        if !self.selection_pending(){return Err("本幕契约已经确定".into());}
        self.gear_done=true;Ok(())
    }
    pub fn choose(&mut self,index:usize,seed:u64,class:Class)->RuleResult<()> {
        if !self.selection_pending(){return Err("本幕契约已经确定".into());}
        let kind=match *self.cards().get(index).ok_or("请选择一份誓约")? {
            Card::Deepen(kind)=>{
                let oath=self.held.iter_mut().find(|o|o.kind==kind&&o.level<MAX_LEVEL).ok_or("誓约已达到最高等级")?;
                oath.level+=1;self.chosen=Some(kind);self.deepened=true;return Ok(());
            },
            Card::Oath(kind)=>kind,
        };
        self.chosen=Some(kind);self.gear=gear(kind,class,seed,self.rules==1);
        if self.rules==1 {self.held.push(Oath{kind,level:1});}
        Ok(())
    }
    pub fn begin_gift(&mut self,index:usize)->RuleResult<()> {
        if self.chosen.is_none()||self.deepened||self.gear_done||index>=3 {return Err("当前不能领取适配装备".into());}
        if self.gift.as_ref().is_some_and(|g|g.index!=index){return Err("装备已开始发放，请重试原选择".into());}
        if self.gift.is_none(){self.gift=Some(Gift{index,delivered:[false;2]});}Ok(())
    }
    pub fn finish_gift(&mut self)->RuleResult<()> {
        if !self.gift.as_ref().is_some_and(|g|g.delivered==[true;2]){return Err("装备尚未发放完成".into());}
        self.gear_done=true;Ok(())
    }
    pub fn skip_gift(&mut self)->RuleResult<()> {
        if self.chosen.is_none()||self.deepened||self.gear_done||self.gift.is_some(){return Err("当前不能跳过装备".into());}
        self.gear_done=true;Ok(())
    }
    pub fn valid(&self,act:u8)->bool {
        let levels:u32=self.held.iter().map(|o|u32::from(o.level)).sum();
        let held_ok=match self.rules {
            0=>self.held.is_empty()&&self.deepen.is_none()&&!self.deepened,
            1=>self.held.iter().enumerate().all(|(i,o)|(1..=MAX_LEVEL).contains(&o.level)&&!self.held[..i].iter().any(|p|p.kind==o.kind))
                &&levels<=u32::from(act)-u32::from(self.chosen.is_none())
                &&self.deepen.is_none_or(|k|self.held.iter().any(|o|o.kind==k&&(o.level<MAX_LEVEL||self.deepened)))
                &&self.offers.iter().all(|k|Some(*k)==self.chosen&&!self.deepened||!self.held.iter().any(|o|o.kind==*k))
                &&self.chosen.is_none_or(|k|self.held.iter().any(|o|o.kind==k))
                &&(!self.deepened||self.chosen.is_some()&&self.chosen==self.deepen),
            _=>false,
        };
        let offers_ok=self.offers.iter().enumerate().all(|(i,k)|!self.offers[..i].contains(k))
            &&if self.rules==0 {self.offers.len()==2&&self.offers.iter().all(|k|Kind::LEGACY.contains(k))}else{(2..=OFFERS).contains(&self.offers.len())};
        self.act==act&&self.peak<=50&&offers_ok&&held_ok
            &&self.chosen.is_none_or(|k|self.offers.contains(&k)||self.deepened)
            &&if self.chosen.is_none()||self.deepened{self.gift.is_none()&&self.gear==[0;3]}else{
                self.gear.iter().enumerate().all(|(i,id)|!self.gear[..i].contains(id)&&crate::rewards::lookup(*id).is_some_and(|r|r.kind==crate::rewards::ItemKind::Weapon))
                    &&self.gift.as_ref().is_none_or(|g|g.index<3&&(!self.gear_done||g.delivered==[true;2]))
            }
    }
}
pub fn reinforcement_level(peak:u8,cap:u8)->u8 {((u16::from(peak.min(50))*u16::from(cap))/50) as u8}
fn gear(kind:Kind,_class:Class,seed:u64,epic:bool)->[u32;3] {
    let mut pool:Vec<u32>=include_str!("../scene-assets/reward-rarities.tsv").lines().filter_map(|line| {
        let (id,tier)=line.split_once('\t')?;let id:u32=id.parse().ok()?;
        let tier=tier.trim().parse::<u8>().ok()?;
        if (if epic{tier!=2}else{tier>1})||!crate::rewards::lookup(id).is_some_and(|i|i.kind==crate::rewards::ItemKind::Weapon){return None;}
        let family=id/1_000_000;
        let matches=match kind {
            Kind::Steel=>(1..=23).contains(&family),
            Kind::Breaker=>matches!(family,3|4|8|12|15|17|18|19|23),
            Kind::Combo=>matches!(family,1|2|5|7|9|10|21|22),
            Kind::Guard=>matches!(family,2|3|11|16|18|31|32),
            Kind::Mystic=>matches!(family,33|34),
            Kind::Hunter=>(40..=44).contains(&family),
            Kind::Execution=>matches!(family,1|5|30),
            Kind::Feather=>matches!(family,1|5|7|21|22),
            Kind::Blood=>(1..=23).contains(&family),
            Kind::Ascetic|Kind::Radiant=>(1..=23).contains(&family)||matches!(family,33|34),
            Kind::Iron|Kind::Greed|Kind::Plague|Kind::Lifeline=>(1..=23).contains(&family),
            Kind::Bulwark=>matches!(family,2|3|11|16|18|31|32),
        };matches.then_some(id)
    }).collect();
    pool.sort_by_key(|id|crate::mix(seed^u64::from(*id)^0x9ea7));
    if kind==Kind::Guard {
        let shield=*pool.iter().find(|id|matches!(**id/1_000_000,31|32)).expect("Missing common/rare shields");
        pool.retain(|id|*id/1_000_000<30);return [shield,pool[0],pool[1]];
    }
    [pool[0],pool[1],pool[2]]
}
pub fn hunter_ammo(weapon:u32)->Option<u32> {
    match weapon/1_000_000 {40|41=>Some(50000000),42=>Some(51000000),43=>Some(52000000),44=>Some(53000000),_=>None}
}

impl crate::Run {
    pub fn enable_contracts(&mut self) {
        if self.contract.is_none() {self.contract=Some(State::start(self.reward_seed(),self.class,self.act,vec![],0));}
    }
    pub fn contract_pending(&self)->bool {self.contract.as_ref().is_some_and(State::pending)}
}
