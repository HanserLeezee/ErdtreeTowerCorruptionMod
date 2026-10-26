// 苦难律法：开局前自选的难度节点。节点沿路线解锁，同一路线效果逐级叠加；分数只用于评级与纪录。
pub struct Node {pub name:&'static str,pub branch:usize,pub parent:Option<usize>,pub points:u32,pub effect:&'static str,pub icon:u32,pub col:u8,pub row:u8}
pub const BRANCHES:[&str;4]=["敌人","我方","经济","规则"];
const fn n(name:&'static str,branch:usize,parent:Option<usize>,points:u32,effect:&'static str,icon:u32,col:u8,row:u8)->Node {Node{name,branch,parent,points,effect,icon,col,row}}
pub const NODES:[Node;27]=[
    n("强敌 I",0,None,10,"所有敌人强度 +1 档",10125,1,0),
    n("强敌 II",0,Some(0),20,"所有敌人强度再 +1 档",10125,2,0),
    n("强敌 III",0,Some(1),30,"所有敌人强度再 +2 档",10125,3,0),
    n("词条蔓延 I",0,None,10,"Boss 词条点数预算 +2",10094,1,1),
    n("词条蔓延 II",0,Some(3),20,"Boss 词条点数预算再 +2",10094,2,1),
    n("精英词条",0,Some(4),20,"精英战的敌人也获得词条",10094,3,1),
    n("坚不可摧 I",0,None,10,"敌人受到的削韧 -30%",12084,1,2),
    n("坚不可摧 II",0,Some(6),10,"敌人受到的削韧再 -20%",12084,2,2),
    n("节制 I",1,None,10,"圣杯瓶上限 -1",402,1,3),
    n("节制 II",1,Some(8),10,"圣杯瓶上限再 -1",402,2,3),
    n("节制 III",1,Some(9),20,"圣杯瓶上限再 -2",402,3,3),
    n("脆弱 I",1,None,10,"我方受到的伤害 +10%",13170,1,4),
    n("脆弱 II",1,Some(11),10,"我方受到的伤害再 +10%",13170,2,4),
    n("脆弱 III",1,Some(12),20,"我方受到的伤害再 +15%",13170,3,4),
    n("孤狼",1,Some(13),30,"无法使用骨灰",104,4,4),
    n("疲惫 I",1,None,10,"精力恢复速度 -20%",18160,1,5),
    n("疲惫 II",1,Some(15),20,"精力恢复速度再 -20%",18160,2,5),
    n("断念 I",1,None,10,"专注消耗 +25%",6504,1,6),
    n("断念 II",1,Some(17),10,"专注消耗再 +25%",6504,2,6),
    n("贫瘠 I",2,None,10,"卢恩获取 -25%",10047,1,7),
    n("贫瘠 II",2,Some(19),10,"卢恩获取再 -25%",10047,2,7),
    n("奸商 I",2,Some(20),10,"商店价格 +30%",11161,3,7),
    n("奸商 II",2,Some(21),10,"商店价格再 +30%",11161,4,7),
    n("吝啬的赐福",2,Some(22),20,"战后奖励三选一改为二选一",18440,5,7),
    n("限时 I",3,None,20,"每场战斗限时 3 分钟，超时后持续掉血",10324,1,8),
    n("限时 II",3,Some(24),20,"战斗限时缩短为 2 分钟",10324,2,8),
    n("嫉妒的世界",3,Some(25),30,"你喝圣杯瓶时，所有敌人回复 10% 生命",401,3,8),
];
pub const GRADES:[(u32,&str);4]=[(70,"老练"),(140,"艰难"),(210,"噩梦"),(280,"危机")];

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,serde::Serialize,serde::Deserialize)]
#[serde(transparent)]
pub struct Law(pub u32);
impl Law {
    pub fn has(self,id:usize)->bool {id<NODES.len()&&self.0&(1<<id)!=0}
    pub fn available(self,id:usize)->bool {id<NODES.len()&&NODES[id].parent.is_none_or(|p|self.has(p))}
    pub fn valid(self)->bool {self.0>>NODES.len()==0&&(0..NODES.len()).all(|id|!self.has(id)||self.available(id))}
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
        (0..7).rev().map(|i|DIGITS[((value>>(i*5))&31) as usize] as char).collect()
    }
    pub fn parse(code:&str)->Option<Self> {
        const DIGITS:&str="0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        let code=code.trim().to_ascii_uppercase().replace(['O'],"0").replace(['I','L'],"1");
        if code.is_empty()||code.len()>7 {return None;}
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
        assert_eq!(Law((1<<NODES.len())-1).score(),420);
    }
    #[test]
    fn selection_follows_paths_and_cascades() {
        let law=Law::default().toggle(1);assert_eq!(law,Law::default(),"locked until the parent is chosen");
        let law=Law::default().toggle(0).toggle(1).toggle(2);assert_eq!(law.score(),60);
        assert_eq!(law.toggle(1),Law::default().toggle(0),"removing a node removes its successors");
        let all=(0..NODES.len()).fold(Law::default(),|l,id|l.toggle(id));
        assert_eq!(all.score(),420);assert_eq!(all.grade(),Some("危机"));assert!(all.valid());
        assert_eq!(Law::default().grade(),None);assert_eq!(Law::default().toggle(24).toggle(25).toggle(26).toggle(0).grade(),Some("老练"));
        assert!(!Law(1<<1).valid());
    }
    #[test]
    fn codes_round_trip_and_reject_typos() {
        let laws=[Law::default(),Law::default().toggle(0).toggle(8).toggle(9),(0..NODES.len()).fold(Law::default(),|l,id|l.toggle(id))];
        for law in laws {let code=law.code();assert_eq!(code.len(),7);assert_eq!(Law::parse(&code),Some(law));assert_eq!(Law::parse(&code.to_lowercase()),Some(law));}
        let code=laws[1].code();let mut broken:Vec<char>=code.chars().collect();broken[6]=if broken[6]=='0'{'1'}else{'0'};
        assert_eq!(Law::parse(&broken.into_iter().collect::<String>()),None);
        assert_eq!(Law::parse(""),None);assert_eq!(Law::parse("ZZZZZZZZ"),None);
    }
}
