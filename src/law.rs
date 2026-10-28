// 苦难律法：开局前自选的难度节点。节点沿路线解锁，同一路线效果逐级叠加；分数只用于评级与纪录。
pub struct Node {pub name:&'static str,pub branch:usize,pub parent:Option<usize>,pub points:u32,pub effect:&'static str,pub icon:u32,pub col:u8,pub row:u8}
pub const BRANCHES:[&str;4]=["敌人","我方","经济","规则"];
const fn n(name:&'static str,branch:usize,parent:Option<usize>,points:u32,effect:&'static str,icon:u32,col:u8,row:u8)->Node {Node{name,branch,parent,points,effect,icon,col,row}}
pub const NODES:[Node;32]=[
    n("强敌 I",0,None,10,"敌人强度 +1 档：生命 +10%，攻击 +6%",10125,1,0),
    n("强敌 II",0,Some(0),20,"再 +1 档：累计生命 +21%，攻击 +13%",10125,2,0),
    n("强敌 III",0,Some(1),30,"再 +2 档：累计生命 +45%，攻击 +28%",10125,3,0),
    n("词条蔓延 I",0,None,10,"Boss 词条点数预算 +2",10094,1,1),
    n("词条蔓延 II",0,Some(3),20,"Boss 词条点数按最高的 6 点抽取",10094,2,1),
    n("精英词条",0,Some(4),20,"精英战的敌人也获得词条",10094,3,1),
    n("坚不可摧 I",0,None,10,"敌人受到的削韧 -30%",12084,1,2),
    n("坚不可摧 II",0,Some(6),10,"敌人受到的削韧累计 -44%",12084,2,2),
    n("节制 I",1,None,10,"红圣杯瓶上限 -1",402,1,3),
    n("节制 II",1,Some(8),10,"蓝圣杯瓶上限也 -1",402,2,3),
    n("节制 III",1,Some(9),20,"红蓝圣杯瓶上限再各 -1",402,3,3),
    n("脆弱 I",1,None,10,"我方受到的伤害 +10%",13170,1,4),
    n("脆弱 II",1,Some(11),10,"我方受到的伤害累计 +20%",13170,2,4),
    n("脆弱 III",1,Some(12),20,"我方受到的伤害累计 +35%",13170,3,4),
    n("孤狼",1,Some(13),30,"无法召唤骨灰，奖励与商店不再出现骨灰",104,4,4),
    n("疲惫 I",1,None,10,"精力恢复速度约 -20%",18160,1,5),
    n("疲惫 II",1,Some(15),20,"精力恢复速度约 -40%",18160,2,5),
    n("断念 I",1,None,10,"魔法、祷告与战技的专注消耗 +25%",6504,1,6),
    n("断念 II",1,Some(17),10,"专注消耗累计 +50%",6504,2,6),
    n("贫瘠 I",2,None,10,"击杀与战斗奖励的卢恩 -15%",10047,1,7),
    n("贫瘠 II",2,Some(19),10,"击杀与战斗奖励的卢恩累计 -30%",10047,2,7),
    n("奸商 I",2,Some(20),10,"商店价格 +15%",11161,3,7),
    n("奸商 II",2,Some(21),10,"商店价格累计 +30%",11161,4,7),
    n("吝啬的赐福",2,Some(22),20,"战后奖励三选一改为二选一",18440,5,7),
    n("限时 I",3,None,20,"每场战斗限时 3 分钟，超时后每秒失去 2% 生命",10324,1,8),
    n("限时 II",3,Some(24),20,"战斗限时缩短为 2 分钟",10324,2,8),
    n("嫉妒的世界",3,Some(25),30,"你喝圣杯瓶时，所有敌人回复 10% 生命",401,3,8),
    n("元素庇护 I",0,None,10,"敌人受到魔力、火、雷、圣伤害 -10%，不影响物理伤害",12084,1,9),
    n("元素庇护 II",0,Some(27),10,"敌人受到魔力、火、雷、圣伤害累计 -20%",12084,2,9),
    n("异常耐性 I",0,None,10,"敌人中毒、腐败、出血、冻伤、睡眠、发狂、死亡耐性 +15%，原有免疫不变",12084,1,10),
    n("异常耐性 II",0,Some(29),10,"敌人异常耐性累计 +30%，原有免疫不变",12084,2,10),
    // 律法以 32 位记录，这是最后一个空位
    n("词条强化",0,Some(5),20,"Boss 与精英的词条数值约提高一半：减伤、攻击、回复、吸血、反弹、加速都更强",10094,4,1),
];pub const GRADES:[(u32,&str);4]=[(70,"老练"),(140,"艰难"),(210,"噩梦"),(280,"危机")];

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,serde::Serialize,serde::Deserialize)]
#[serde(transparent)]
pub struct Law(pub u32);
impl Law {
    pub fn is_empty(self)->bool {self.0==0}
    fn chain(self,ids:&[usize])->u8 {ids.iter().take_while(|&&id|self.has(id)).count() as u8}
    /// 强敌：敌人额外档位（0、1、2、4）
    pub fn enemy_tiers(self)->u8 {[0,1,2,4][usize::from(self.chain(&[0,1,2]))]}
    pub fn enemy_level(self)->u8 {self.chain(&[0,1,2])}
    /// 词条蔓延：Boss 词条预算（不超过组合表上限）
    pub fn affix_budget(self,base:u8,max:u8)->u8 {match self.chain(&[3,4]) {0=>base,1=>(base+2).min(max),_=>max}}
    pub fn elite_affixes(self)->bool {self.has(5)}
    /// 词条强化：Boss 词条使用强化后的数值
    pub fn empowered_affixes(self)->bool {self.has(31)}
    pub fn toughness(self)->u8 {self.chain(&[6,7])}
    pub fn elemental_resistance(self)->u8 {self.chain(&[27,28])}
    pub fn status_resistance(self)->u8 {self.chain(&[29,30])}
    /// 节制：红、蓝圣杯瓶上限各减少多少
    pub fn flask_cut(self)->[u8;2] {[[0,0],[1,0],[1,1],[2,2]][usize::from(self.chain(&[8,9,10]))]}
    pub fn fragile(self)->u8 {self.chain(&[11,12,13])}
    pub fn lone_wolf(self)->bool {self.has(14)}
    pub fn fatigue(self)->u8 {self.chain(&[15,16])}
    pub fn focus(self)->u8 {self.chain(&[17,18])}
    /// 贫瘠：卢恩获取百分比
    pub fn poverty(self)->u8 {self.chain(&[19,20])}
    pub fn rune_percent(self)->u32 {[100,85,70][usize::from(self.poverty())]}
    pub fn runes(self,amount:u32)->u32 {if self.rune_percent()==100 {amount}else{(u64::from(amount)*u64::from(self.rune_percent())/100) as u32}}
    /// 奸商：商店价格百分比
    pub fn price_percent(self)->u32 {[100,115,130][usize::from(self.chain(&[21,22]))]}
    pub fn stingy(self)->bool {self.has(23)}
    /// 限时：每场战斗的秒数
    pub fn time_limit(self)->Option<u64> {match self.chain(&[24,25]) {0=>None,1=>Some(180),_=>Some(120)}}
    pub fn envy_world(self)->bool {self.has(26)}
    pub fn has(self,id:usize)->bool {id<NODES.len()&&self.0&(1<<id)!=0}
    pub fn available(self,id:usize)->bool {id<NODES.len()&&NODES[id].parent.is_none_or(|p|self.has(p))}
    pub fn valid(self)->bool {u64::from(self.0)>>NODES.len()==0&&(0..NODES.len()).all(|id|!self.has(id)||self.available(id))}
    /// 选择需要上一节点已选；取消会连带取消其后的节点
    pub fn toggle(self,id:usize)->Self {
        if self.has(id) {
            let mut next=self.0&!(1<<id);
            loop {let before=next;for (i,node) in NODES.iter().enumerate(){if node.parent.is_some_and(|p|next&(1<<p)==0){next&=!(1<<i);}}if next==before{break;}}
            Law(next)
        } else if self.available(id) {Law(self.0|1<<id)} else {self}
    }
    pub fn score(self)->u32 {(0..NODES.len()).filter(|&id|self.has(id)).map(|id|NODES[id].points).sum()}
    pub fn grade(self)->Option<&'static str> {GRADES.iter().rev().find(|(min,_)|self.score()>=*min).map(|(_,name)|*name)}
    pub fn selected(self)->Vec<usize> {
        let mut ids:Vec<_>=(0..NODES.len()).filter(|&id|self.has(id)).collect();
        ids.sort_by_key(|&id|(std::cmp::Reverse(NODES[id].points),id));ids
    }
    /// 律法码：节点位图加 5 位校验，Crockford Base32，便于分享
    pub fn code(self)->String {
        const DIGITS:&[u8]=b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        let value=(u64::from(self.0)<<5)|checksum(self.0);
        (0..if value<(1u64<<35){7}else{8}).rev().map(|i|DIGITS[((value>>(i*5))&31) as usize] as char).collect()
    }
    pub fn parse(code:&str)->Option<Self> {
        const DIGITS:&str="0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        let code=code.trim().to_ascii_uppercase().replace(['O'],"0").replace(['I','L'],"1");
        if code.is_empty()||code.len()>8 {return None;}
        let value=code.chars().try_fold(0u64,|v,c|DIGITS.find(c).map(|d|v<<5|d as u64))?;
        let law=Law(u32::try_from(value>>5).ok()?);
        (value&31==checksum(law.0)&&law.valid()).then_some(law)
    }
}
fn checksum(mask:u32)->u64 {u64::from((0..32).map(|i|((mask>>i)&1)*(i+7)).fold(11u32,|a,b|a.wrapping_mul(31).wrapping_add(b))%32)}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graph_is_well_formed() {
        for (id,node) in NODES.iter().enumerate() {
            if let Some(p)=node.parent {assert!(p<id&&NODES[p].branch==node.branch&&NODES[p].row==node.row&&NODES[p].col+1==node.col,"{}",node.name);}
            else {assert_eq!(node.col,1,"{}",node.name);}
            assert!(NODES.iter().enumerate().all(|(o,other)|o==id||(other.col,other.row)!=(node.col,node.row)));
        }
        assert_eq!(Law(((1u64<<NODES.len())-1) as u32).score(),480);
    }
    #[test]
    fn selection_follows_paths_and_cascades() {
        let law=Law::default().toggle(1);assert_eq!(law,Law::default(),"locked until the parent is chosen");
        let law=Law::default().toggle(0).toggle(1).toggle(2);assert_eq!(law.score(),60);
        assert_eq!(law.toggle(1),Law::default().toggle(0),"removing a node removes its successors");
        let all=(0..NODES.len()).fold(Law::default(),|l,id|l.toggle(id));
        assert_eq!(all.score(),480);assert_eq!(all.grade(),Some("危机"));assert!(all.valid());
        assert_eq!(Law::default().grade(),None);assert_eq!(Law::default().toggle(24).toggle(25).toggle(26).toggle(0).grade(),Some("老练"));
        assert!(!Law(1<<1).valid());
    }
    #[test]
    fn effects_follow_each_route() {
        let law=|ids:&[usize]|ids.iter().fold(Law::default(),|l,&id|l.toggle(id));
        assert_eq!((law(&[]).enemy_tiers(),law(&[0]).enemy_tiers(),law(&[0,1]).enemy_tiers(),law(&[0,1,2]).enemy_tiers()),(0,1,2,4));
        assert_eq!((law(&[]).flask_cut(),law(&[8]).flask_cut(),law(&[8,9]).flask_cut(),law(&[8,9,10]).flask_cut()),([0,0],[1,0],[1,1],[2,2]));
        assert_eq!((law(&[]).affix_budget(2,6),law(&[3]).affix_budget(2,6),law(&[3,4]).affix_budget(2,6),law(&[3]).affix_budget(6,6)),(2,4,6,6));
        assert_eq!((law(&[19]).runes(1000),law(&[19,20]).runes(1000),law(&[]).runes(1000)),(850,700,1000));
        assert_eq!((law(&[19,20,21]).price_percent(),law(&[19,20,21,22]).price_percent()),(115,130));
        assert_eq!((law(&[24]).time_limit(),law(&[24,25]).time_limit(),law(&[]).time_limit()),(Some(180),Some(120),None));
        let all=law(&(0..NODES.len()).collect::<Vec<_>>());
        assert!(all.lone_wolf()&&all.stingy()&&all.envy_world()&&all.elite_affixes()&&all.empowered_affixes());
        assert!(!law(&[3,4]).empowered_affixes()&&law(&[3,4,5,31]).empowered_affixes());
        assert_eq!((all.fragile(),all.fatigue(),all.focus(),all.toughness()),(3,2,2,2));
    }
    #[test]
    fn codes_round_trip_and_reject_typos() {
        let laws=[Law::default(),Law::default().toggle(0).toggle(8).toggle(9),(0..NODES.len()).fold(Law::default(),|l,id|l.toggle(id))];
        for law in laws {let code=law.code();assert!((7..=8).contains(&code.len()));assert_eq!(Law::parse(&code),Some(law));assert_eq!(Law::parse(&code.to_lowercase()),Some(law));}
        let code=laws[1].code();let mut broken:Vec<char>=code.chars().collect();broken[6]=if broken[6]=='0'{'1'}else{'0'};
        assert_eq!(Law::parse(&broken.into_iter().collect::<String>()),None);
        assert_eq!(Law::parse(""),None);assert_eq!(Law::parse("ZZZZZZZZ"),None);
    }
    #[test]
    fn runs_keep_the_law_and_apply_its_rules() {
        use crate::{Class,Phase,Reward,Run,NodeKind,rewards::{lookup,ItemKind}};
        let law=|ids:&[usize]|ids.iter().fold(Law::default(),|l,&id|l.toggle(id));
        let plain=Run::new(7,Class::Samurai);
        assert!(!plain.encode().unwrap().contains("\"law\""),"saves without a law keep the old format");
        let mut run=plain.clone();run.set_law(law(&[8,9,10,19,20,21,22,23])).unwrap();
        assert_eq!((run.flask_limit,run.flasks),(6,6));
        assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
        assert!(Run::decode(&plain.encode().unwrap()).unwrap().law.is_empty());
        let mut bad=run.clone();bad.law=Law(1<<1);assert!(bad.validate().is_err());
        let mut started=plain.clone();let id=started.available()[0];started.enter(id).unwrap();assert!(started.set_law(law(&[0])).is_err());
        let mut lone=0;
        for seed in 0..300 {
            let mut free=Run::new(seed,Class::Astrologer);let mut bound=free.clone();bound.set_law(law(&[11,12,13,14,19,20,21,22,23])).unwrap();
            let id=free.available()[0];assert_eq!(free.nodes[id].kind,NodeKind::Monster);
            for r in [&mut free,&mut bound] {r.enter(id).unwrap();r.win_combat().unwrap();}
            let (Phase::Reward{offers:a,..},Phase::Reward{offers:b,..})=(&free.phase,&bound.phase) else{panic!()};
            assert_eq!(b.len(),2);
            assert!(b.iter().all(|r|!matches!(r,Reward::Item(i) if lookup(*i).is_some_and(|i|i.kind==ItemKind::Spirit))));
            lone+=a.iter().filter(|r|matches!(r,Reward::Item(i) if lookup(*i).is_some_and(|i|i.kind==ItemKind::Spirit))).count();
            for (x,y) in a.iter().zip(b) {if let (Reward::Gold(x),Reward::Gold(y))=(x,y){assert_eq!(*y,bound.law.runes(*x));}}
            assert_eq!(Run::decode(&bound.encode().unwrap()).unwrap(),bound);
            bound.claim_reward(1).unwrap();
        }
        assert!(lone>0,"without the law spirits still appear");
        let mut capped=plain.clone();capped.set_law(law(&[8])).unwrap();
        for _ in 0..4 {capped.grant(Reward::FlaskCapacity).unwrap();}
        assert_eq!(capped.flask_limit,17);assert!(capped.validate().is_ok());
        assert!(capped.align_party_start(capped.seed).is_ok(),"matching party routes preserve the selected law");
    }
}
