use std::{collections::HashMap,sync::LazyLock};
use crate::{Run,Reward,rewards::{Item,ItemKind}};

struct Variant {item:Item,base:u32,tier:u8,profile:Option<u8>}
static VARIANTS:LazyLock<Vec<Variant>>=LazyLock::new(||include_str!("../scene-assets/affix-catalog.txt").lines().chain(include_str!("../scene-assets/affix-v2-catalog.txt").lines()).map(|line| {
    let p:Vec<_>=line.split('|').collect();let tag=if p[0]=="weapon"{0}else{0x10000000};
    Variant{item:Item{id:tag|p[1].parse::<u32>().unwrap(),kind:if tag==0{ItemKind::Weapon}else{ItemKind::Armor},name:p[4]},base:tag|p[2].parse::<u32>().unwrap(),tier:p[3].parse().unwrap(),profile:p.get(5).map(|v|v.parse().unwrap())}
}).collect());
pub static CATALOG:LazyLock<Vec<Item>>=LazyLock::new(||VARIANTS.iter().map(|v|Item{id:v.item.id,kind:v.item.kind,name:v.item.name}).collect());
static BY_ID:LazyLock<HashMap<u32,usize>>=LazyLock::new(||VARIANTS.iter().enumerate().map(|(i,v)|(v.item.id,i)).collect());
static BY_BASE:LazyLock<HashMap<u32,[u32;4]>>=LazyLock::new(|| {
    let mut map=HashMap::new();for v in VARIANTS.iter().filter(|v|v.profile.is_none()){map.entry(v.base).or_insert([0;4])[v.tier as usize]=v.item.id;}map
});
static ADVANCED:LazyLock<HashMap<u32,Vec<[u32;4]>>>=LazyLock::new(|| {
    let mut map=HashMap::new();for v in VARIANTS.iter(){if let Some(profile)=v.profile {
        let groups=map.entry(v.base).or_insert_with(Vec::new);groups.resize(groups.len().max(profile as usize+1),[0;4]);groups[profile as usize][v.tier as usize]=v.item.id;
    }}map
});
static BASE_RARITY:LazyLock<HashMap<u32,u8>>=LazyLock::new(||include_str!("../scene-assets/reward-rarities.tsv").lines().map(|l|{let (id,tier)=l.split_once('\t').unwrap();(id.parse().unwrap(),tier.parse().unwrap())}).collect());
pub fn lookup(id:u32)->Option<&'static Item>{BY_ID.get(&id).map(|i|&CATALOG[*i])}
pub fn base(id:u32)->u32 {BY_ID.get(&id).map_or(id,|i|VARIANTS[*i].base)}
pub fn tier(id:u32)->Option<u8>{BY_ID.get(&id).map(|i|VARIANTS[*i].tier)}
pub fn profile(id:u32)->Option<u8>{BY_ID.get(&id).and_then(|i|VARIANTS[*i].profile)}
pub fn roll(run:&Run,id:u32,entropy:u64,repeat:bool)->Option<u32> {
    if !run.affix_equipment{return Some(id);}
    let groups=if run.affix_rules>=2 {ADVANCED.get(&id).map(Vec::as_slice)}else{BY_BASE.get(&id).map(std::slice::from_ref)};
    let Some(groups)=groups else{return Some(id)};
    let start=if run.affix_rules>=2{crate::mix(run.reward_seed()^entropy^u64::from(id)^0xa4c4_7e99) as usize%groups.len()}else{0};
    let floor=BASE_RARITY.get(&id).copied().unwrap_or(0).min(3);
    let value=crate::mix(run.reward_seed()^entropy^u64::from(id)^0xaff1_600d)%100;
    let selected=(match value {0..=44=>0,45..=74=>1,75..=92=>2,_=>3}).max(floor);
    for n in 0..groups.len() {
        let variants=&groups[(start+n)%groups.len()];
        if let Some(id)=(0..4).map(|offset|(selected+offset)%4).filter(|t|*t>=floor).map(|t|variants[t as usize]).find(|variant|*variant!=0&&!run.claimed_items.contains(variant)){return Some(id);}
    }
    repeat.then_some(groups[start][selected as usize])
}
pub fn rewards(run:&Run,node:usize,mut offers:Vec<Reward>)->Vec<Reward> {
    if !run.affix_equipment{return offers;}
    let entropy=(node as u64).wrapping_mul(7919);
    let normal=run.nodes.get(node).is_some_and(|n|matches!(n.kind,crate::NodeKind::Monster|crate::NodeKind::Chest));
    if normal&&crate::mix(run.reward_seed()^entropy^0xa420)%4==0 {
        if let Some(slot)=offers.iter().position(|r|matches!(r,Reward::Item(id) if crate::rewards::lookup(*id).is_some_and(|i|i.kind==ItemKind::Weapon))) {
            let armor:Vec<_>=crate::rewards::CATALOG.iter().filter(|i|i.kind==ItemKind::Armor).collect();
            let start=crate::mix(entropy^run.reward_seed()) as usize%armor.len();
            if let Some(item)=(0..armor.len()).map(|n|armor[(start+n)%armor.len()]).find(|i|roll(run,i.id,entropy^slot as u64,false).is_some()){offers[slot]=Reward::Item(item.id);}
        }
    }
    let repeat=run.nodes.get(node).is_some_and(|n|n.kind==crate::NodeKind::Boss);
    for (slot,reward) in offers.iter_mut().enumerate(){if let Reward::Item(id)=*reward {
        *reward=roll(run,id,entropy^slot as u64,repeat).map_or(Reward::Gold(1500),Reward::Item);
    }}
    offers
}
/// 盲盒：从本职业可用的武器与防具中抽一件，品质按盲盒档次随机（传说档武器至少为其原有品质）
pub fn mystery_box(run:&Run,node:usize,premium:bool)->Option<u32> {
    let roll=crate::mix(run.reward_seed()^(node as u64).wrapping_mul(7919)^0xb0b0_5e1d^u64::from(premium));
    let value=crate::mix(roll)%100;
    // 奖励装备的最低品质为稀有：普通盲盒 稀有 65／史诗 30／传说 5，高级盲盒 稀有 15／史诗 55／传说 30
    let tier:u8=if premium {match value {0..=14=>1,15..=69=>2,_=>3}} else {match value {0..=64=>1,65..=94=>2,_=>3}};
    // 先定品质，再只从能以该品质出现的装备里抽，说明里写的概率就是实际概率
    let pool:Vec<_>=crate::rewards::CATALOG.iter().filter(|i|(i.kind==ItemKind::Weapon&&i.random_eligible()||i.kind==ItemKind::Armor)&&i.kind.allowed(run.class)
        &&BY_BASE.contains_key(&i.id)&&BASE_RARITY.get(&i.id).copied().unwrap_or(0)<=tier).collect();
    if pool.is_empty() {return None;}
    let start=(roll%pool.len() as u64) as usize;
    // 抽中的装备这局已经拿过时，顺延到下一件没拿过的
    (0..pool.len()).map(|n|pool[(start+n)%pool.len()].id).find_map(|base|variant(run,base,(roll>>32) as usize,tier).filter(|id|!run.claimed_items.contains(id)))
}
fn variant(run:&Run,base:u32,profile:usize,tier:u8)->Option<u32> {
    let id=if run.affix_rules>=2 {let groups=ADVANCED.get(&base)?;groups[profile%groups.len()][usize::from(tier)]}else{BY_BASE.get(&base)?[usize::from(tier)]};
    (id!=0).then_some(id)
}
/// 重铸：换成同一件装备在同品质下的另一种词条组合；有 1/5 的概率品质 +1（这时任意组合都可以）。
/// 精炼（guaranteed）必定品质 +1，传说品质不能精炼
pub fn reforge(run:&Run,node:usize,current:u32,guaranteed:bool)->Option<u32> {
    let (base,tier)=(base(current),tier(current)?);
    if guaranteed&&tier>=3 {return None;}
    let roll=crate::mix(run.reward_seed()^(node as u64).wrapping_mul(7919)^0x4e70_f09e^u64::from(current));
    let upgrade=guaranteed||tier<3&&roll%5==0;
    let target=if upgrade {tier+1}else{tier};
    let mut options:Vec<u32>=ADVANCED.get(&base).into_iter().flatten().map(|g|g[usize::from(target)])
        .chain(BY_BASE.get(&base).map(|g|g[usize::from(target)]).filter(|_|run.affix_rules<2))
        .filter(|id|*id!=0&&*id!=current).collect();
    options.dedup();
    (!options.is_empty()).then(||options[((roll>>8)%options.len() as u64) as usize])
}

