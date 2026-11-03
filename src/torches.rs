//! [HansFix] 火把：每把火把是独立的一件装备（占 regulation 里预留的一个火把位 24800000 + 位号×100），
//! 各自带：原版火把外观、补正（力气/灵巧/智力/信仰/感应里随机 1~5 项）、3 个词条、一个法术战技、合成次数。
//! 法术战技是只能装在火把上的专用战灰（990000 + 法术序号），火把之间可以互换；
//! 左手举起火把（防御）满 10 秒自动释放这个法术，不做动作。
//! 合成次数 0/1/2 对应 火把 / 灼烧的火把 / 黑焰的火把。
use std::sync::LazyLock;
use serde::{Deserialize,Serialize};

#[cfg(test)]
thread_local! {
    /// 测试：其他测试固定了奖励页内容，只在专门的测试里打开火把卡
    pub static TEST_OFFERS:std::cell::Cell<bool>=const {std::cell::Cell::new(false)};
}
pub const SLOT_FIRST:u32=24800000;
pub const SLOTS:u16=512;
pub const GEM_FIRST:u32=990000;
pub const ART_FIRST:u32=9600;
pub const FLASH_FIRST:u32=9600300;
/// 原版火把外观（本体 6 把 + DLC 的娜娜亚烛火）；喟叹头颅等同类武器不算火把
pub const BASES:[u32;7]=[24000000,24020000,24040000,24050000,24060000,24070000,24500000];
pub const STATS:[&str;5]=["力气","灵巧","智力","信仰","感应"];
pub const LETTERS:[&str;7]=["","E","D","C","B","A","S"];
/// 字母档位换成 +0 时的补正数值（取各档中间值）
pub const VALUES:[f32;7]=[0.0,15.0,40.0,75.0,115.0,155.0,190.0];
/// 火把的词条池（与装备词条同一套效果行：9850000 + 种类×10 + 品质）
/// 堕落商店：每件可替换的商品换成火把的概率（%），火把的品质与基础售价
pub const SHOP_REPLACE:u64=10;
pub const SHOP_QUALITY:u8=2;
pub const SHOP_PRICE:i32=5000;
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
/// 补正数值对应的字母档位（与游戏的划分相同：E <25，D <60，C <90，B <140，A <175，S 以上）
pub fn letter_of(value:f32)->u8 {
    if value<=0.0 {0} else if value<25.0 {1} else if value<60.0 {2} else if value<90.0 {3} else if value<140.0 {4} else if value<175.0 {5} else {6}
}
/// 两组补正相加（数值相加，再按总和定字母）
pub fn add_scaling(a:[u8;5],b:[u8;5])->[u8;5] {
    std::array::from_fn(|i|letter_of(VALUES[usize::from(a[i].min(6))]+VALUES[usize::from(b[i].min(6))]))
}
/// 并入过火把的武器：武器（去掉强化等级的编号）与并入的火把补正档位
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct WeaponMerge {pub weapon:u32,pub scaling:[u8;5]}
impl WeaponMerge {pub fn values(&self)->[f32;5] {self.scaling.map(|l|VALUES[usize::from(l.min(6))])}}
/// 名字里带火的武器可以在铁匠处拆成火把：火字旁的字（含“火”“炎”本身），另加“灵”“灭”；四点底（灬）不算
pub const FIRE_CHARS:&str="火炎灯灶灿炀炉炊炒炕炖炜炝炫炬炭炮炯炳炷炸炼炽烁烂烃烘烙烛烟烤烦烧烨烩烫烬烯烽焊焐焕焖焗焙焜焰煅煌煜煤煨煸熄熔熘熠燃燎燔燠燥燧爆爝灼灵灭";
pub fn fire_named(name:&str)->bool {name.chars().any(|c|FIRE_CHARS.contains(c))}
/// 拆成几把火把：重量 <10 一把，10~<15 两把，15 及以上三把；颜面盾四把
pub fn dismantle_count(name:&str,weight:f32)->u8 {
    if name.contains("颜面盾") {4} else if weight>=15.0 {3} else if weight>=10.0 {2} else {1}
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
        let slot=self.free_torch_slots(&[]).next().ok_or("火把位已用完")?;
        self.torch_rolls=self.torch_rolls.wrapping_add(1);
        let seed=crate::mix(self.reward_seed()^salt^u64::from(self.torch_rolls).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let torch=roll(seed,slot,quality,crate::dlc::mode()==crate::dlc::Mode::Full);
        self.torches.push(torch.clone());
        Ok(torch)
    }
    /// 调试：指定法术的火把
    pub fn new_torch_with_spell(&mut self,quality:u8,spell:u8)->crate::RuleResult<Torch> {
        let mut torch=self.new_torch(quality,0xde_b0^u64::from(spell))?;
        torch.spell=spell%SPELLS.len() as u8;
        if let Some(t)=self.torches.iter_mut().find(|t|t.slot==torch.slot) {t.spell=torch.spell;}
        Ok(torch)
    }
    /// 空闲的火把位：本局的火把、商店里摆着的火把与 extra 都不占
    pub fn free_torch_slots<'a>(&'a self,extra:&'a [Torch])->impl Iterator<Item=u16>+'a {
        let shop=self.shop_stock.as_ref().map_or(&[][..],|s|&s.torches[..]);
        (0..SLOTS).filter(move|s|!self.torches.iter().chain(shop).chain(extra).any(|t|t.slot==*s))
    }
    /// 堕落商店的火把：必定有 1 把（room 为真时追加在货单末尾，否则替换一件可替换的商品），
    /// 其余每件可替换的商品（replaceable）各有 10% 换成随机火把。返回（替换第几件 / None=追加，火把）
    pub fn shop_torches(&self,node:usize,replaceable:&[bool],room:bool)->Vec<(Option<usize>,Torch)> {
        let seed=crate::mix(self.reward_seed()^0x7043_5e11^(node as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)^u64::from(self.torch_rolls).wrapping_mul(0xbf58_476d_1ce4_e5b9));
        let dlc=crate::dlc::mode()==crate::dlc::Mode::Full;
        let mut picks:Vec<Option<usize>>=Vec::new();
        if room {picks.push(None);}
        for (i,&ok) in replaceable.iter().enumerate() {if ok&&crate::mix(seed^0x10^i as u64)%100<SHOP_REPLACE {picks.push(Some(i));}}
        if !room&&picks.is_empty() {if let Some(i)=replaceable.iter().position(|&ok|ok) {picks.push(Some(i));}}
        let mut result:Vec<(Option<usize>,Torch)>=Vec::new();
        for (n,pick) in picks.into_iter().enumerate() {
            let held:Vec<Torch>=result.iter().map(|(_,t)|t.clone()).collect();
            let Some(slot)=self.free_torch_slots(&held).next() else{break};
            result.push((pick,roll(crate::mix(seed^0x7_0c4^n as u64),slot,SHOP_QUALITY,dlc)));
        }
        result
    }
    /// 火把 + 火把：两把合成次数相同（火把 / 灼烧的火把）才能合，保留 keep 的词条与法术战技，补正相加；
    /// other 用掉。返回合成后的火把
    pub fn merge_torches(&mut self,keep:u16,other:u16)->crate::RuleResult<Torch> {
        if keep==other {return Err("请选择两把不同的火把".into());}
        let a=self.torches.iter().position(|t|t.slot==keep).ok_or("火把不存在")?;
        let b=self.torches.iter().position(|t|t.slot==other).ok_or("火把不存在")?;
        if self.torches[a].merges!=self.torches[b].merges {return Err("只能合并同一阶的火把（火把 + 火把，灼烧的火把 + 灼烧的火把）".into());}
        if self.torches[a].merges>=2 {return Err("黑焰的火把不能再合并".into());}
        let scaling=add_scaling(self.torches[a].scaling,self.torches[b].scaling);
        let torch=&mut self.torches[a];torch.merges+=1;torch.scaling=scaling;
        let merged=torch.clone();
        self.torches.retain(|t|t.slot!=other);
        Ok(merged)
    }
    /// 火把并入武器：武器得到火把的补正（数值相加），火把用掉；每把武器只能并入一次
    pub fn merge_into_weapon(&mut self,torch:u16,weapon:u32)->crate::RuleResult<WeaponMerge> {
        let weapon=weapon/100*100;
        if slot_of(weapon).is_some()||BASES.contains(&weapon) {return Err("火把之间请用“合并火把”".into());}
        if (50_000_000..60_000_000).contains(&weapon) {return Err("箭矢不能并入火把".into());}
        if self.weapon_merges.iter().any(|m|m.weapon==weapon) {return Err("这把武器已经并入过火把".into());}
        let index=self.torches.iter().position(|t|t.slot==torch).ok_or("火把不存在")?;
        let merge=WeaponMerge{weapon,scaling:self.torches[index].scaling};
        self.torches.remove(index);self.weapon_merges.push(merge.clone());
        Ok(merge)
    }
    /// 拆解名字带火的武器：按重量得到 1~4 把火把，品质随机但不低于原武器
    pub fn dismantle(&mut self,name:&str,weight:f32,quality:u8)->crate::RuleResult<Vec<Torch>> {
        if !fire_named(name) {return Err("只有名字带火的武器可以拆成火把".into());}
        let count=dismantle_count(name,weight);
        let mut result=Vec::new();
        for n in 0..count {
            let roll=crate::mix(self.reward_seed()^0xd15_a7^u64::from(self.torch_rolls).wrapping_mul(31)^u64::from(n));
            let rolled=match roll%100 {0..40=>0,40..70=>1,70..90=>2,_=>3};
            result.push(self.new_torch(rolled.max(quality.min(3)),0xd15^u64::from(n))?);
        }
        Ok(result)
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

#[cfg(test)]
mod smith_tests {
    use super::*;
    #[test]
    fn letters_follow_the_summed_values() {
        assert_eq!(letter_of(0.0),0);assert_eq!(letter_of(15.0),1);assert_eq!(letter_of(40.0),2);assert_eq!(letter_of(75.0),3);
        assert_eq!(letter_of(115.0),4);assert_eq!(letter_of(155.0),5);assert_eq!(letter_of(190.0),6);
        for l in 1..=6 {assert_eq!(letter_of(VALUES[l]),l as u8,"each letter's own value maps back to it");}
        // E + E = 30 → D；C + C = 150 → A；S + 任何 → S
        assert_eq!(add_scaling([1,3,6,0,0],[1,3,1,0,2]),[2,5,6,0,2]);
    }
    #[test]
    fn torch_merges_need_matching_tiers_and_keep_the_chosen_skill() {
        let mut run=crate::Run::new(3,crate::Class::Hero);
        let a=run.new_torch(1,1).unwrap();let b=run.new_torch(1,2).unwrap();let c=run.new_torch(1,3).unwrap();
        let merged=run.merge_torches(b.slot,a.slot).unwrap();
        assert_eq!((merged.slot,merged.spell,&merged.affixes,merged.merges),(b.slot,b.spell,&b.affixes,1));
        assert_eq!(merged.scaling,add_scaling(a.scaling,b.scaling));assert_eq!(merged.prefix(),"灼烧的");
        assert!(run.torch(a.weapon_id()).is_none(),"the other torch is used up");
        assert!(run.merge_torches(b.slot,c.slot).is_err(),"a scorched torch only merges with another scorched torch");
        let d=run.new_torch(1,4).unwrap();let e=run.new_torch(1,5).unwrap();
        run.merge_torches(d.slot,e.slot).unwrap();
        let black=run.merge_torches(b.slot,d.slot).unwrap();assert_eq!((black.merges,black.prefix()),(2,"黑焰的"));
        let f=run.new_torch(1,6).unwrap();let g=run.new_torch(1,7).unwrap();run.merge_torches(f.slot,g.slot).unwrap();
        let h=run.new_torch(1,8).unwrap();let i=run.new_torch(1,9).unwrap();run.merge_torches(h.slot,i.slot).unwrap();
        run.merge_torches(f.slot,h.slot).unwrap();
        assert!(run.merge_torches(b.slot,f.slot).is_err(),"black-flame torches are final");
        assert!(run.merge_torches(c.slot,c.slot).is_err());
    }
    #[test]
    fn a_weapon_takes_one_torch_and_dismantling_follows_weight() {
        let mut run=crate::Run::new(4,crate::Class::Hero);
        let a=run.new_torch(0,1).unwrap();let b=run.new_torch(0,2).unwrap();
        let merge=run.merge_into_weapon(a.slot,2000005).unwrap();assert_eq!((merge.weapon,merge.scaling),(2000000,a.scaling));
        assert!(run.merge_into_weapon(b.slot,2000003).is_err(),"once per weapon");
        assert!(run.merge_into_weapon(b.slot,b.weapon_id()).is_err()&&run.merge_into_weapon(b.slot,24000000).is_err(),"torches merge with torches");
        assert!(run.merge_into_weapon(b.slot,50000000).is_err(),"ammo is not a weapon");
        assert_eq!(run.torches.len(),1);
        for (name,weight,count) in [("火焰巨剑",9.9,1),("火焰巨剑",10.0,2),("熔炉大锤",15.0,3),("颜面盾",8.0,4)] {assert_eq!(dismantle_count(name,weight),count);}
        assert!(fire_named("灵火斧")&&fire_named("灭火者")&&fire_named("烛台剑"));
        assert!(!fire_named("熊爪")&&!fire_named("照耀长剑")&&!fire_named("长剑"),"four-dot bottoms are not fire");
        let got=run.dismantle("熔炉大锤",16.0,2).unwrap();assert_eq!(got.len(),3);assert!(got.iter().all(|t|t.quality>=2));
        assert!(run.dismantle("长剑",3.0,0).is_err());
        assert_eq!(crate::Run::decode(&run.encode().unwrap()).unwrap(),run);
    }
}

#[cfg(test)]
mod shop_tests {
    #[test]
    fn fallen_shops_always_hold_one_torch_and_replace_about_a_tenth_of_the_rest() {
        let (mut replaced,mut eligible)=(0,0);
        for seed in 0..400u64 {
            let mut run=crate::Run::new(seed,crate::Class::Hero);
            run.new_torch(1,0).unwrap();
            let replaceable:Vec<bool>=(0..30).map(|i|i%3!=0).collect();
            let plan=run.shop_torches(5,&replaceable,true);
            assert_eq!(plan.iter().filter(|(i,_)|i.is_none()).count(),1,"exactly one extra torch");
            assert!(plan.iter().all(|(i,_)|i.is_none_or(|i|replaceable[i])),"only replaceable goods turn into torches");
            let slots:std::collections::HashSet<_>=plan.iter().map(|(_,t)|t.slot).collect();
            assert_eq!(slots.len(),plan.len());assert!(!slots.contains(&run.torches[0].slot),"never reuse an owned torch slot");
            replaced+=plan.len()-1;eligible+=20;
            assert_eq!(run.shop_torches(5,&replaceable,true),plan,"same shop, same torches");
            let full=run.shop_torches(5,&replaceable,false);
            assert!(full.iter().all(|(i,_)|i.is_some())&&!full.is_empty(),"a full shop still gets a torch by replacement");
            // 摆在商店里的火把占着火把位：同时拿到的新火把不会撞位
            run.shop_stock=Some(crate::ShopStock{node:5,items:vec![],torches:plan.iter().map(|(_,t)|t.clone()).collect()});
            let fresh=run.new_torch(0,1).unwrap();assert!(!slots.contains(&fresh.slot));
            let stock=run.shop_stock.clone().unwrap();assert_eq!(serde_json::from_str::<crate::ShopStock>(&serde_json::to_string(&stock).unwrap()).unwrap(),stock);
        }
        // 没有火把的货单照旧序列化（不多出字段），旧存档也能读
        assert_eq!(serde_json::to_string(&crate::ShopStock{node:1,items:vec![[0,1,2,3]],torches:vec![]}).unwrap(),r#"{"node":1,"items":[[0,1,2,3]]}"#);
        let rate=replaced as f64/eligible as f64;assert!((0.07..0.13).contains(&rate),"{rate}");
    }
}

#[cfg(test)]
mod offer_tests {
    #[test]
    fn fallen_mode_pages_offer_a_torch_about_a_third_of_the_time_and_claiming_it_rolls_one() {
        super::TEST_OFFERS.with(|t|t.set(true));
        let (mut pages,mut torches)=(0,0);
        for seed in 0..300u64 {
            let mut run=crate::Run::new(seed,crate::Class::Hero);run.wander=true;
            let id=run.nodes.iter().position(|n|n.kind==crate::NodeKind::Monster).unwrap();
            let offers=run.offers_for_test(id);pages+=1;
            let n=offers.iter().filter(|r|matches!(r,crate::Reward::Torch(_))).count();assert!(n<=1);torches+=n;
            if let Some(index)=offers.iter().position(|r|matches!(r,crate::Reward::Torch(_))) {
                run.enter(id).unwrap();run.win_combat().unwrap();
                let crate::Phase::Reward{ref offers,..}=run.phase else{panic!()};
                if offers.get(index).is_some_and(|r|matches!(r,crate::Reward::Torch(_))) {run.claim_reward(index).unwrap();assert_eq!(run.torches.len(),1);}
            }
        }
        assert!((60..=120).contains(&torches),"{torches} torches in {pages} pages");
    }
}
