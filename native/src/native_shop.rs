use eldenring::cs::{
    CSEventFlagMan, EquipParamWeapon, EquipParamGoods, EquipParamCustomWeapon, ItemCategory, PlayerGameData, ShopLineupParam,
    SoloParamRepository,
};
use erdtree_tower::{Blessing, BlessingKind, NodeKind, Run};
use fromsoftware_shared::FromStatic;
pub const GOODS: u32 = 9500000;
pub const STOCK_COUNT: u32 = 37;
fn key_offer(roll:u64)->bool {roll%100<20}
fn tear_offer(roll:u64)->bool {roll%100<50}
fn whetblade_offer(roll:u64,claimed:&[u32])->Option<i32> {
    if roll%100>=30{return None;}
    let pool:Vec<_>=(8970..=8974).filter(|id|!claimed.contains(&(0x40000000|*id))).collect();
    if pool.is_empty(){None}else{Some(pool[(roll/100) as usize%pool.len()] as i32)}
}
fn withdraw_owned_whetblades(stock:&mut [Stock],run:&Run) {
    for item in stock.iter_mut().filter(|s|s.kind==3&&(8970..=8974).contains(&s.id)) {
        if run.claimed_items.contains(&(0x40000000|item.id as u32)){item.quantity=0;}
    }
}
fn refresh_special_prices(stock:&mut [Stock],run:&Run) {
    for item in stock.iter_mut().filter(|s|s.kind==3) {
        let base=match item.id {8000=>2500,8970..=8974=>6000,id if id==erdtree_tower::elite::TEAR as i32=>5000,_=>continue};
        item.price=super::law::shop_price(run.law,super::passives::shop_price(run.class,base));
    }
}
#[test]
fn key_shop_roll_has_exact_twenty_percent_and_one_key_for_2500() {
    assert_eq!((0..100).filter(|&r|key_offer(r)).count(),20);
    let mut count=0;
    for seed in 0..1000 {
        let run=Run::new(seed,erdtree_tower::Class::Samurai);
        let rows=stock(&run);
        if let Some(key)=rows.iter().find(|s|s.kind==3&&s.id==8000) {
            count+=1;assert_eq!((key.price,key.quantity),(2500,1));
        }
        assert_eq!(rows,stock(&run));
    }
    assert!((160..=240).contains(&count));
}
#[test]
fn sacred_tear_has_half_chance_one_stock_and_is_independent_of_keys() {
    assert_eq!((0..100).filter(|&r|tear_offer(r)).count(),50);
    let mut count=0;let mut combinations=std::collections::HashSet::new();
    for seed in 0..1000 {
        let run=Run::new(seed,erdtree_tower::Class::Samurai);let rows=stock(&run);
        assert_eq!(rows.len(),STOCK_COUNT as usize);assert_eq!(rows,stock(&run));
        let tears:Vec<_>=rows.iter().filter(|s|s.kind==3&&s.id==erdtree_tower::elite::TEAR as i32).collect();
        assert!(tears.len()<=1);
        if let Some(tear)=tears.first(){count+=1;assert_eq!((tear.price,tear.quantity),(5000,1));}
        combinations.insert((!tears.is_empty(),rows.iter().any(|s|s.kind==3&&s.id==8000)));
    }
    assert!((440..=560).contains(&count),"Tear frequency: {count}/1000");assert_eq!(combinations.len(),4);
}
#[test]
fn saved_special_prices_update_without_restocking_or_double_discounting() {
    let run=Run::new(42,erdtree_tower::Class::Prisoner);
    let mut saved=vec![Stock{kind:3,id:8000,price:4000,quantity:0},Stock{kind:3,id:10020,price:5000,quantity:1},Stock{kind:3,id:900,price:123,quantity:3}];
    refresh_special_prices(&mut saved,&run);
    assert_eq!((saved[0].price,saved[0].quantity),(2000,0));assert_eq!((saved[1].price,saved[1].quantity),(4000,1));
    let once=saved.clone();refresh_special_prices(&mut saved,&run);assert_eq!(saved,once);assert_eq!(saved[2].price,123);
}
const STOCK_FLAGS: u32 = 45020800;
#[test]
fn whetblades_have_thirty_percent_chance_cost_6000_and_skip_owned_blades() {
    assert_eq!((0..100).filter(|&roll|whetblade_offer(roll,&[]).is_some()).count(),30);
    let all=(8970..=8974).map(|id|0x40000000|id).collect::<Vec<u32>>();
    assert!((0..1000).all(|roll|whetblade_offer(roll,&all).is_none()));
    let mut seen=std::collections::HashSet::new();let mut count=0;let mut combinations=std::collections::HashSet::new();
    for seed in 0..1000 {
        let mut run=Run::new(seed,erdtree_tower::Class::Samurai);let rows=stock(&run);
        let blades:Vec<_>=rows.iter().filter(|s|s.kind==3&&(8970..=8974).contains(&s.id)).collect();assert!(blades.len()<=1);
        assert_eq!(rows.len(),STOCK_COUNT as usize);assert_eq!(rows,stock(&run));
        if let Some(blade)=blades.first(){count+=1;seen.insert(blade.id);assert_eq!((blade.price,blade.quantity),(6000,1));}
        combinations.insert((!blades.is_empty(),rows.iter().any(|s|s.kind==3&&s.id==10020),rows.iter().any(|s|s.kind==3&&s.id==8000)));
        run.claimed_items.extend(all.iter().copied());assert!(stock(&run).iter().all(|s|s.kind!=3||!(8970..=8974).contains(&s.id)));
    }
    assert!((240..=360).contains(&count),"Whetblade frequency: {count}/1000");assert_eq!(seen.len(),5);assert_eq!(combinations.len(),8);
    for id in 8970..=8974 {let owned=all.iter().copied().filter(|v|*v!=(0x40000000|id)).collect::<Vec<_>>();assert_eq!(whetblade_offer(0,&owned),Some(id as i32));}
}
#[test]
fn owned_whetblades_are_hidden_without_refilling_other_saved_stock() {
    let mut run=Run::new(42,erdtree_tower::Class::Prisoner);run.claimed_items.push(0x40000000|8970);
    let mut rows=vec![Stock{kind:3,id:8970,price:6000,quantity:1},Stock{kind:3,id:8971,price:6000,quantity:0},Stock{kind:3,id:900,price:123,quantity:3}];
    withdraw_owned_whetblades(&mut rows,&run);refresh_special_prices(&mut rows,&run);
    assert_eq!((rows[0].price,rows[0].quantity),(4800,0));assert_eq!((rows[1].price,rows[1].quantity),(4800,0));assert_eq!((rows[2].price,rows[2].quantity),(123,3));
}
fn withdrawn(item:&Stock)->bool {item.kind==3&&matches!(item.id,1150|1160|1170|1180|1190|1200)}
fn withdraw_goods(stock:&mut [Stock]) {
    for item in stock.iter_mut().filter(|item|withdrawn(item)) {item.quantity=0;}
}
#[test]
fn livers_and_fowl_feet_cannot_be_bought_in_new_or_saved_stock() {
    for seed in 0..100 {
        let rows=stock(&Run::new(seed,erdtree_tower::Class::Samurai));
        assert!(rows.iter().filter(|item|withdrawn(item)).all(|item|item.quantity==0));
        assert!(!rows.iter().any(|item|item.kind==3&&matches!(item.id,1190|1200)));
    }
    let mut saved=vec![Stock{kind:3,id:1190,price:300,quantity:5},Stock{kind:3,id:1150,price:300,quantity:10},Stock{kind:3,id:900,price:300,quantity:10}];
    withdraw_goods(&mut saved);
    assert_eq!(saved.iter().map(|item|item.quantity).collect::<Vec<_>>(),[0,0,10]);
    assert_eq!(saved.iter().map(|item|item.id).collect::<Vec<_>>(),[1190,1150,900]);
}
#[test]
fn dlc_shop_pools_are_reachable_with_affixes_and_native_categories() {
    let mut seen=[false;6];
    for seed in 0..2000 {
        for item in stock(&Run::new(seed,erdtree_tower::Class::Astrologer)) {
            let base=erdtree_tower::affixes::base(if item.kind==1 {0x10000000|item.id as u32}else{item.id as u32})&0x0fffffff;
            seen[0]|=item.kind==0&&base==64500000;
            seen[1]|=item.kind==1&&(5000000..6000000).contains(&base);
            seen[2]|=item.kind==2&&item.id==8220;
            seen[3]|=item.kind==3&&item.id==2004320;
            seen[4]|=item.kind==4&&item.id==409000;
            seen[5]|=item.kind==3&&(2011000..=2011070).contains(&item.id);
        }
        if seen.iter().all(|s|*s){break;}
    }
    assert_eq!(seen,[true;6]);
}
fn weapon_price(rarity:u8)->i32 {match rarity {0|1=>3000,2=>5000,_=>8000}}
#[test]
fn new_shop_sells_affixed_equipment_and_legacy_stock_is_preserved() {
    for seed in 0..40 {
        let mut run=Run::new(seed,erdtree_tower::Class::Astrologer);
        let items=stock(&run);assert_eq!(items,stock(&run));
        for item in items.iter().filter(|i|i.quantity==1&&matches!(i.kind,0|1)) {
            let id=if item.kind==1{0x10000000|item.id as u32}else{item.id as u32};
            assert!(erdtree_tower::affixes::tier(id).is_some());
        }
        run.affix_equipment=false;
        assert!(stock(&run).iter().filter(|i|i.kind==0).all(|i|i.id<100000000));
    }
}
fn talisman_price(tier:u8)->i32 {match tier {0=>1000,1=>2000,2=>4000,_=>8000}}
#[test]
fn randomized_talismans_reach_the_shop_with_matching_quality_prices() {
    let mut seen=std::collections::HashSet::new();
    for seed in 0..2000 {
        let run=Run::new(seed,erdtree_tower::Class::Samurai);
        let ids:Vec<_>=stock(&run).iter().filter(|i|i.kind==2).map(|i|i.id).collect();assert_eq!(ids.iter().collect::<std::collections::HashSet<_>>().len(),ids.len());
        for item in stock(&run).iter().filter(|i|i.kind==2) {
            let id=0x20000000|item.id as u32;
            if let Some(tier)=erdtree_tower::talismans::tier(id){seen.insert(tier);assert_eq!(item.price,talisman_price(tier));}
        }
        let mut old=run;old.talisman_affixes=false;
        assert!(stock(&old).iter().filter(|i|i.kind==2).all(|i|erdtree_tower::talismans::lookup(0x20000000|i.id as u32).is_none()));
    }
    assert_eq!(seen.len(),4);
}
#[test]
fn talisman_prices_follow_card_quality_and_class_discount() {
    for (tier,price) in [(0,1000),(1,2000),(2,4000),(3,8000)] {
        assert_eq!(talisman_price(tier),price);
        assert_eq!(super::passives::shop_price(erdtree_tower::Class::Prisoner,price),price*4/5);
    }
    for item in stock(&Run::new(42,erdtree_tower::Class::Samurai)).iter().filter(|s|s.kind==2) {
        assert_eq!(item.price,talisman_price(super::reward_ui::item_quality(0x20000000|item.id as u32)));
    }
}
fn is_spell(item:&Stock)->bool {
    use erdtree_tower::rewards::{lookup,ItemKind};
    item.kind==3 && lookup(0x40000000|item.id as u32)
        .is_some_and(|item|matches!(item.kind,ItemKind::Sorcery|ItemKind::Incantation))
}
#[test]
fn larval_tear_is_always_available_for_5000_runes() {
    for seed in 0..64 {
        let rows=stock(&Run::new(seed,erdtree_tower::Class::Samurai));
        let tears:Vec<_>=rows.iter().filter(|s|s.kind==3&&s.id==8185).collect();
        assert_eq!(tears.len(),1);
        assert_eq!((tears[0].price,tears[0].quantity),(5000,-1));
    }
}
#[test]
fn crystal_shop_roll_is_rare_stable_and_costs_6000() {
    let mut count=0;
    for seed in 0..1000 {
        let run=Run::new(seed,erdtree_tower::Class::Samurai);
        let rows=stock(&run);
        if let Some(item)=rows.iter().find(|s|s.kind==3&&erdtree_tower::rewards::lookup(0x40000000|s.id as u32).is_some_and(|i|i.kind==erdtree_tower::rewards::ItemKind::Crystal)) {
            count+=1;assert_eq!((item.price,item.quantity),(6000,1));
            assert_eq!(rows,stock(&run));
        }
    }
    assert!((25..=75).contains(&count));
}
fn stock_flag(index:u32)->u32 {if index<21 {STOCK_FLAGS+index*8}else if index<26 {45020700+(index-21)*8}else{45020600+(index-26)*8}}
#[test]
fn random_shop_only_sells_base_talismans() {
    for seed in 0..500 {
        let run=Run::new(seed,erdtree_tower::Class::Samurai);
        for item in stock(&run).iter().filter(|s|s.kind==2) {
            assert!(erdtree_tower::rewards::lookup(0x20000000|item.id as u32).unwrap().random_eligible());
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stock {
    pub kind: u8,
    pub id: i32,
    pub price: i32,
    pub quantity: i16,
}
fn hash(mut n: u64) -> u64 {
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    n ^ (n >> 31)
}
fn select(mut pool:Vec<Stock>,count:usize,seed:u64,visit:usize)->Vec<Stock> {
    assert!(pool.len()>=count);
    pool.sort_by_key(|s|hash(seed^s.id as u64));
    (0..count).map(|i|pool[(visit*count+i)%pool.len()].clone()).collect()
}
/// 没有 DLC 时不进货的物品（按商店类别换回带标记的物品编号再判断；战灰单独判断）
fn offered(item:&Stock)->bool {
    let id=item.id as u32;
    match item.kind {0=>erdtree_tower::dlc::item_allowed(id),1=>erdtree_tower::dlc::item_allowed(0x10000000|id),2=>erdtree_tower::dlc::item_allowed(0x20000000|id),
        3=>erdtree_tower::dlc::item_allowed(0x40000000|id),4=>erdtree_tower::dlc::gem_allowed(id),_=>true}
}
fn extras(group:&str)->Vec<Stock> {
    include_str!("../../scene-assets/shop-extras.txt").lines().filter_map(|line| {
        let p:Vec<_>=line.split('|').collect();
        (p[0]==group).then(||Stock{kind:p[1].parse().unwrap(),id:p[2].parse().unwrap(),price:p[3].parse().unwrap(),quantity:p[4].parse().unwrap()})
    }).filter(|item|(group!="food"||!withdrawn(item))&&offered(item)).collect()
}
pub fn stock(run:&Run)->Vec<Stock> {
    use erdtree_tower::{Class,rewards::{CATALOG,ItemKind}};
    let visit=run.visited.iter().filter(|&&i|run.nodes[i].kind==NodeKind::Shop).count();
    let catalog:Vec<_>=include_str!("../../scene-assets/shop-catalog.txt").lines().map(|line| {
        let p:Vec<_>=line.split('|').collect();
        Stock{kind:p[0].parse().unwrap(),id:p[1].parse().unwrap(),price:p[2].parse().unwrap(),quantity:1}
    }).filter(offered).collect();
    let mut result=Vec::new();
    let catalyst=match run.class {Class::Astrologer|Class::Prisoner=>Some("staff"),Class::Prophet|Class::Confessor=>Some("seal"),_=>None};
    if let Some(group)=catalyst {result.extend(select(extras(group),2,run.seed,visit));}
    result.extend(select(catalog.iter().filter(|s|s.kind==0).cloned().collect(),if catalyst.is_some(){3}else{5},run.seed,visit));
    for (kind,equip,price) in [(ItemKind::Sorcery,3,1800),(ItemKind::Incantation,3,1800),(ItemKind::Armor,1,1500)] {
        result.extend(select(CATALOG.iter().filter(|s|s.kind==kind&&erdtree_tower::dlc::item_allowed(s.id)).map(|s|Stock{kind:equip,id:(s.id&0x0fffffff) as i32,price,quantity:1}).collect(),3,run.seed,visit));
    }
    // 原来卖投掷壶的一格改卖第二种飞刀：投掷壶要有龟裂壶才能持有，商店不再发放龟裂壶
    if tear_offer(hash(run.seed ^ visit as u64 ^ 0x74656172)) {
        result.push(Stock{kind:3,id:erdtree_tower::elite::TEAR as i32,price:5000,quantity:1});
    } else {result.extend(select(extras("food"),1,run.seed,visit));}
    result.extend(select(extras("knife"),1,run.seed,visit));
    let knives=extras("knife").into_iter().filter(|s|!result.iter().any(|r|r.kind==s.kind&&r.id==s.id)).collect();
    if let Some(id)=whetblade_offer(hash(run.seed ^ visit as u64 ^ 0x77686574),&run.claimed_items) {
        result.push(Stock{kind:3,id,price:6000,quantity:1});
    } else {result.extend(select(knives,1,run.seed^0x6b6e_6966,visit));}
    result.extend(select(catalog.iter().filter(|s|s.kind==4).cloned().collect(),2,run.seed,visit));
    let pool=CATALOG.iter().filter(|s|s.kind==ItemKind::Talisman&&s.random_eligible()).map(|s|Stock{kind:2,id:(s.id&0x0fffffff) as i32,price:talisman_price(super::reward_ui::item_quality(s.id)),quantity:1}).collect();
    result.extend(select(pool,2,run.seed,visit));
    // 召唤师的商店常备骨灰
    if !run.law.lone_wolf()&&(run.summoner()||hash(run.seed ^ visit as u64 ^ 0xa549)%4==0) {
        result.extend(select(CATALOG.iter().filter(|s|s.kind==ItemKind::Spirit&&erdtree_tower::dlc::item_allowed(s.id)).map(|s|Stock{kind:3,id:(s.id&0x0fffffff) as i32,price:2000,quantity:1}).collect(),1,run.seed,visit));
    } else {
        let pool=extras("food").into_iter().filter(|s|!result.iter().any(|r|r.kind==s.kind&&r.id==s.id)).collect();
        result.extend(select(pool,1,run.seed,visit));
    }
    for item in &mut result {
        if item.kind==3 {item.id=(erdtree_tower::rewards::random_spirit_level(0x40000000|item.id as u32,run.reward_seed()^visit as u64)&0x0fffffff) as i32;}
    }
    result.push(Stock{kind:3,id:8185,price:5000,quantity:-1});
    if let Some(id)=erdtree_tower::rewards::crystal_offer(run,visit as u64,0x5a09) {
        result[21]=Stock{kind:3,id:(id&0x0fffffff) as i32,price:6000,quantity:1};
    }
    if key_offer(hash(run.seed ^ visit as u64 ^ 0x8000cafe)) {
        result.push(Stock{kind:3,id:8000,price:2500,quantity:1});
    } else {
        let pool=extras("food").into_iter().filter(|s|!result.iter().any(|r|r.kind==s.kind&&r.id==s.id)).collect();
        result.extend(select(pool,1,run.seed ^ 0x8000cafe,visit));
    }
    result.push(Stock{kind:0,id:50000000,price:20,quantity:100});
    let arrows=[50010000,50090000,50100000,50200000];
    let arrow=arrows[hash(run.reward_seed() ^ visit as u64 ^ 0xa440) as usize%arrows.len()];
    result.push(Stock{kind:0,id:arrow,price:60,quantity:20});
    result.extend(extras("resistance"));
    affix_stock(run,visit,&mut result);
    withdraw_goods(&mut result);
    result
}
fn affix_stock(run:&Run,visit:usize,result:&mut [Stock]) {
    let mut talismans=std::collections::HashSet::new();
    for (slot,item) in result.iter_mut().enumerate() {
        if item.kind==2&&item.quantity==1 {
            let original=0x20000000|item.id as u32;
            let mut id=erdtree_tower::talismans::roll(run,original,(visit as u64).wrapping_mul(7919)^slot as u64,true);
            if !talismans.insert(id){id=original;talismans.insert(id);}
            item.id=(id&0x0fffffff) as i32;item.price=talisman_price(super::reward_ui::item_quality(id));
        }
        if item.quantity==1&&matches!(item.kind,0|1) {
            let tag=if item.kind==0{0}else{0x10000000};
            if let Some(id)=erdtree_tower::affixes::roll(run,tag|item.id as u32,(visit as u64).wrapping_mul(7919)^slot as u64,true){item.id=(id&0x0fffffff) as i32;}
        }
    }
}
pub fn prepare(run: &Run, node: usize, refresh: bool, data:&mut PlayerGameData) -> Result<erdtree_tower::ShopStock, String> {
    let params = unsafe { SoloParamRepository::instance_mut() }.map_err(|_| "商店参数未加载")?;
    let saved=run.shop_stock.as_ref().filter(|saved|!refresh&&saved.node==node&&saved.items.len()==STOCK_COUNT as usize);
    let mut stock=if let Some(saved)=saved {saved.items.iter().map(|[kind,id,price,quantity]|Stock{kind:*kind as u8,id:*id,price:*price,quantity:*quantity as i16}).collect()}else{
        let tools=super::loot::casting_tools(data);
        let mut stock = stock(run);
        filter_spells(&mut stock,tools,run.reward_seed());
        affix_stock(run,run.visited.iter().filter(|&&i|run.nodes[i].kind==NodeKind::Shop).count(),&mut stock);
        for item in &mut stock {
            let rarity=if item.kind==0&&(item.id<50000000||erdtree_tower::affixes::tier(item.id as u32).is_some()) {
                Some(params.get::<EquipParamWeapon>(item.id as u32).ok_or("商店武器参数不存在")?.rarity())
            } else if is_spell(item) {
                Some(params.get::<EquipParamGoods>(item.id as u32).ok_or("商店法术参数不存在")?.rarity())
            } else {None};
            if let Some(rarity)=rarity {item.price=weapon_price(rarity);}
        }
        for item in &mut stock {item.price=super::law::shop_price(run.law,super::passives::shop_price(run.class,item.price));}
        stock
    };
    withdraw_goods(&mut stock);
    withdraw_owned_whetblades(&mut stock,run);
    refresh_special_prices(&mut stock,run);
    if stock.len()!=STOCK_COUNT as usize {return Err("商店货单数量与原生商品槽位不符".into());}
    for i in 0..STOCK_COUNT {
        if params.get::<ShopLineupParam>(GOODS + i).is_none() {
            return Err("缺少原生商店参数，请使用完整启动包".into());
        }
    }
    for item in &stock {
        let category=match item.kind {
            0=>ItemCategory::Weapon,1=>ItemCategory::Protector,2=>ItemCategory::Accessory,3=>ItemCategory::Goods,4=>ItemCategory::Gem,
            _=>return Err("无效商店商品类型".into()),
        };
        let id=eldenring::cs::ItemId::new(category,item.id as u32).map_err(|e|e.to_string())?;
        if params.get_equip_param(id).is_none() {
            return Err(format!("商品参数不存在：{} / {}",item.kind,item.id));
        }
    }
    let flags = unsafe { CSEventFlagMan::instance_mut() }.map_err(|_| "商店库存标记未加载")?;
    for flag in (0..STOCK_COUNT).flat_map(|i|stock_flag(i)..stock_flag(i)+8).filter(|_|refresh) {
        flags.virtual_memory_flag.set_flag(flag, false);
    }
    let peak=super::smith::inventory_peak(data,params);
    let granted:Vec<_>=stock.iter().map(|item|if item.kind==0 {super::smith::acquisition_id(item.id as u32,peak,params).map(|id|id as i32)}else{Ok(item.id)}).collect::<Result<_,String>>()?;
    let mut shop_items=Vec::with_capacity(stock.len());
    for (i,item) in stock.iter().enumerate() {
        let custom=item.kind==0&&item.quantity==1&&super::random_ashes::eligible(granted[i] as u32);
        shop_items.push(if custom {(super::random_ashes::shop_weapon(params,i as u32,granted[i] as u32)?,5)}else{(granted[i],item.kind)});
    }
    for (i, item) in stock.iter().enumerate() {
        let row = params.get_mut::<ShopLineupParam>(GOODS + i as u32).unwrap();
        row.set_equip_id(shop_items[i].0);
        row.set_equip_type(shop_items[i].1);
        row.set_value(item.price);
        row.set_cost_type(0);
        row.set_mtrl_id(-1);
        row.set_sell_quantity(item.quantity);
        row.set_set_num(1);
        row.set_icon_id(-1);
        row.set_name_msg_id(-1);
        row.set_menu_title_msg_id(-1);
        row.set_menu_icon_id(-1);
        row.set_value_add(0);
        row.set_value_magnification(1.0);
    }
    super::log(format!("Native rune shop stock: {stock:?}"));
    Ok(erdtree_tower::ShopStock{node,items:stock.iter().map(|s|[i32::from(s.kind),s.id,s.price,i32::from(s.quantity)]).collect()})
}
pub fn refresh_weapon_levels(data:&PlayerGameData)->Result<(),String> {
    let params=unsafe {SoloParamRepository::instance_mut()}.map_err(|_|"商店参数未加载")?;
    let peak=super::smith::inventory_peak(data,params);
    let mut updates=Vec::new();
    let mut custom_updates=Vec::new();
    for id in GOODS..GOODS+STOCK_COUNT {
        let row=params.get::<ShopLineupParam>(id).ok_or("商店货单尚未就绪")?;
        if row.equip_type()==5&&row.equip_id()>=super::random_ashes::SHOP_FIRST as i32&&row.equip_id()<(super::random_ashes::SHOP_FIRST+STOCK_COUNT) as i32 {
            let custom=params.get::<EquipParamCustomWeapon>(row.equip_id() as u32).ok_or("随机战灰商品参数缺失")?;
            let weapon=custom.base_wep_id() as u32+u32::from(custom.reinforce_lv());
            let next=super::smith::acquisition_id(weapon,peak,params)?;
            custom_updates.push((row.equip_id() as u32,next));continue;
        }
        if row.equip_type()!=0||row.equip_id()<0 {continue;}
        let base=row.equip_id() as u32/100*100;
        updates.push((id,super::smith::acquisition_id(base,peak,params)? as i32));
    }
    for (row,item) in updates {params.get_mut::<ShopLineupParam>(row).unwrap().set_equip_id(item);}
    for (id,weapon) in custom_updates {
        let custom=params.get_mut::<EquipParamCustomWeapon>(id).unwrap();
        custom.set_base_wep_id((weapon/100*100) as i32);custom.set_reinforce_lv((weapon%100) as u8);
    }
    super::log(format!("Shop weapon levels refreshed: bag_peak={peak}"));
    Ok(())
}
fn filter_spells(stock:&mut [Stock],tools:[bool;2],seed:u64) {
    use erdtree_tower::rewards::{CATALOG,ItemKind,castable};
    for index in 0..stock.len() {
        if stock[index].kind==3&&!castable(0x40000000|stock[index].id as u32,tools) {
            let pool:Vec<_>=CATALOG.iter().filter(|i|i.kind==ItemKind::Weapon&&i.random_eligible()
                &&!stock.iter().any(|s|s.kind==0&&s.id==i.id as i32)).collect();
            stock[index]=Stock{kind:0,id:pool[hash(seed^index as u64) as usize%pool.len()].id as i32,price:3000,quantity:1};
        }
    }
}
#[test]
fn shop_catalyst_filter_preserves_stock_indices_and_allowed_spells() {
    for tools in [[false,false],[true,false],[false,true],[true,true]] {
        let original=stock(&Run::new(42,erdtree_tower::Class::Astrologer));
        let mut filtered=original.clone();filter_spells(&mut filtered,tools,42);
        assert_eq!(filtered.len(),STOCK_COUNT as usize);
        for (before,after) in original.iter().zip(&filtered) {
            if before.kind==3&&!erdtree_tower::rewards::castable(0x40000000|before.id as u32,tools) {
                assert_eq!(after.kind,0);
            } else {assert_eq!(before,after);}
        }
        let mut repeated=original;filter_spells(&mut repeated,tools,42);assert_eq!(filtered,repeated);
    }
}
pub fn pending(data: &PlayerGameData) -> Vec<usize> {
    data.equipment
        .equip_inventory_data
        .items_data
        .items()
        .filter(|e| {
            e.quantity > 0
                && e.item_id.category() == ItemCategory::Goods
                && (GOODS..GOODS + 8).contains(&e.item_id.param_id())
        })
        .map(|e| (e.item_id.param_id() - GOODS) as usize)
        .collect()
}
pub fn consume(data: &mut PlayerGameData) {
    for e in data.equipment.equip_inventory_data.items_data.items_mut() {
        if e.item_id.category() == ItemCategory::Goods
            && (GOODS..GOODS + 8).contains(&e.item_id.param_id())
        {
            e.quantity = 0;
        }
    }
}
pub fn add_blessings(run: &mut Run, indices: &[usize]) {
    for &index in indices {
        let kind = BlessingKind::ALL[index];
        if let Some(b) = run.blessings.iter_mut().find(|b| b.kind == kind) {
            b.level = (b.level + 1).min(3);
        } else {
            run.blessings.push(Blessing { kind, level: 1 });
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn every_shop_keeps_ten_boluses_and_withdraws_livers() {
        for class in erdtree_tower::Class::ALL {for seed in 0..32 {
            let rows=super::stock(&erdtree_tower::Run::new(seed,class));
            for id in [900,910,920,930,940,950,960,1150,1160,1170,1180] {
                let matches:Vec<_>=rows.iter().filter(|s|s.kind==3&&s.id==id).collect();
                assert_eq!(matches.len(),1,"missing or duplicate medicine {id}");
                assert_eq!((matches[0].quantity,matches[0].price),(if id>=1150 {0}else{10},300));
            }
        }}
    }
    use super::*;
    #[test]
    fn each_shop_has_fixed_arrow_supply_and_one_stable_element() {
        let mut seen=std::collections::HashSet::new();
        for class in erdtree_tower::Class::ALL {for seed in 0..100 {
            let run=Run::new_personal(seed,class,seed+1000);
            let rows=stock(&run);
            assert_eq!(rows,stock(&run));
            let arrows:Vec<_>=rows.iter().filter(|s|s.kind==0&&(50000000..60000000).contains(&s.id)).collect();
            assert_eq!(arrows.len(),2);
            assert_eq!((arrows[0].id,arrows[0].quantity,arrows[0].price),(50000000,100,20));
            assert!([50010000,50090000,50100000,50200000].contains(&arrows[1].id));
            assert_eq!((arrows[1].quantity,arrows[1].price),(20,60));
            seen.insert(arrows[1].id);
        }}
        assert_eq!(seen.len(),4);
    }
    #[test]
    fn spell_pricing_excludes_other_goods_and_discounts_after_rarity() {
        use erdtree_tower::{Class,rewards::{CATALOG,ItemKind}};
        for item in CATALOG.iter() {
            let stock=Stock{kind:3,id:(item.id&0x0fffffff) as i32,price:1800,quantity:1};
            if item.id>>28==4 {assert_eq!(is_spell(&stock),matches!(item.kind,ItemKind::Sorcery|ItemKind::Incantation));}
        }
        for id in [8000,8185,10020,11000] {assert!(!is_spell(&Stock{kind:3,id,price:5000,quantity:1}));}
        for (rarity,price,discounted) in [(0,3000,2400),(1,3000,2400),(2,5000,4000),(3,8000,6400)] {
            assert_eq!(weapon_price(rarity),price);
            assert_eq!(super::super::passives::shop_price(Class::Prisoner,weapon_price(rarity)),discounted);
        }
    }
    #[test]
    fn weapon_prices_increase_with_native_rarity() {
        assert_eq!(weapon_price(1),3000);
        assert_eq!(weapon_price(2),5000);
        assert_eq!(weapon_price(3),8000);
        assert!((0..u8::MAX).all(|r|weapon_price(r)<=weapon_price(r+1)));
    }
    #[test]
    fn expanded_stock_has_required_categories_quantities_and_class_catalysts() {
        use erdtree_tower::{Class,rewards::{CATALOG,ItemKind}};
        for class in Class::ALL {
            for seed in 0..64 {
                let run=Run::new(seed,class);let rows=stock(&run);
                assert_eq!(rows.len(),STOCK_COUNT as usize);
                assert_eq!(rows.iter().map(|s|(s.kind,s.id)).collect::<std::collections::HashSet<_>>().len(),rows.len());
                for kind in [ItemKind::Sorcery,ItemKind::Incantation,ItemKind::Armor] {
                    assert_eq!(rows.iter().filter(|s|CATALOG.iter().any(|c|c.kind==kind&&c.id==erdtree_tower::affixes::base(if s.kind==1{0x10000000|s.id as u32}else{0x40000000|s.id as u32})&&s.kind==if kind==ItemKind::Armor {1}else{3})).count(),3);
                }
                assert_eq!(rows.iter().filter(|s|s.kind==0&&s.quantity==1).count(),5);
                let spirit=rows.iter().filter(|s|s.kind==3&&erdtree_tower::rewards::lookup(0x40000000|s.id as u32).is_some_and(|c|c.kind==ItemKind::Spirit)).count();
                let crystal=rows.iter().any(|s|s.kind==3&&CATALOG.iter().any(|i|i.id==0x40000000|s.id as u32&&i.kind==ItemKind::Crystal));
                assert_eq!(spirit,usize::from(!crystal&&(run.summoner()||hash(seed ^ 0xa549)%4==0)));
                for (group,quantity) in [("food",5),("knife",20)] {
                    let pool=extras(group);
                    let selected:Vec<_>=rows.iter().filter(|s|pool.iter().any(|p|p.kind==s.kind&&p.id==s.id)).collect();
                    assert_eq!(selected.len(),if group=="knife"{2-usize::from(rows.iter().any(|s|s.kind==3&&(8970..=8974).contains(&s.id)))}else{usize::from(!rows.iter().any(|s|s.kind==3&&s.id==erdtree_tower::elite::TEAR as i32))+usize::from(spirit==0&&!crystal)+usize::from(!rows.iter().any(|s|s.kind==3&&s.id==8000))});
                    assert!(selected.iter().all(|s|s.quantity==quantity));
                }
                assert!(rows.iter().all(|s|!extras("pot").iter().any(|p|p.kind==s.kind&&p.id==s.id)),"the shop no longer sells throwing pots");
                let catalyst=match class {Class::Astrologer|Class::Prisoner=>Some("staff"),Class::Prophet|Class::Confessor=>Some("seal"),_=>None};
                if let Some(group)=catalyst {
                    assert_eq!(rows.iter().filter(|s|extras(group).iter().any(|p|p.kind==s.kind&&p.id as u32==erdtree_tower::affixes::base(s.id as u32))).count(),2);
                }
            }
        }
        let flags:Vec<_>=(0..STOCK_COUNT).flat_map(|i|stock_flag(i)..stock_flag(i)+8).collect();
        assert!(flags.iter().all(|f|*f<super::super::spirits::SUMMONED_FLAG));
        assert_eq!(flags.iter().collect::<std::collections::HashSet<_>>().len(),flags.len());
    }
    #[test]
    fn shop_spirits_cover_all_levels_in_every_act_and_keep_their_price() {
        use erdtree_tower::rewards::{lookup,ItemKind};
        for act in 1..=3 {
            let mut levels=std::collections::BTreeSet::new();
            for seed in 0..300 {
                let mut run=Run::new(seed,erdtree_tower::Class::Samurai);run.act=act;
                let first=stock(&run);let second=stock(&run);
                assert!(first.iter().zip(second.iter()).all(|(a,b)|a.id==b.id&&a.kind==b.kind));
                for item in first.iter().filter(|s|s.kind==3&&lookup(0x40000000|s.id as u32).is_some_and(|i|i.kind==ItemKind::Spirit)) {
                    assert_eq!(item.price,2000);assert!((0..=10).contains(&(item.id%1000)));levels.insert(item.id%1000);
                }
            }
            assert_eq!(levels,(0..=10).collect());
        }
    }
    #[test]
    fn same_shop_is_stable_next_shop_has_different_stock_and_all_categories() {
        let mut run = Run::new(42, erdtree_tower::Class::Samurai);
        let first = stock(&run);
        assert_eq!(first, stock(&run));
        assert_eq!(first.len(), STOCK_COUNT as usize);
        assert_eq!(first.iter().filter(|s|s.kind==2).count(),2);
        assert!(first.iter().all(|s|s.kind!=3 || !(9500000..9500008).contains(&s.id)));
        assert_eq!(first.iter().filter(|s| s.kind == 0&&s.quantity==1).count(), 5);
        assert_eq!(first.iter().filter(|s| s.kind == 4).count(), 2);
        let shop = run
            .nodes
            .iter()
            .position(|n| n.kind == NodeKind::Shop)
            .unwrap();
        run.visited.push(shop);
        assert_ne!(first, stock(&run));
        assert!(stock(&run).iter().all(|s| s.price > 0));
    }
    #[test]
    fn capped_blessings_are_not_sold_and_shop_remains_full() {
        let mut run = Run::new(1, erdtree_tower::Class::Samurai);
        run.blessings = BlessingKind::ALL
            .iter()
            .map(|&kind| Blessing { kind, level: 3 })
            .collect();
        let rows = stock(&run);
        assert_eq!(rows.len(), STOCK_COUNT as usize);
        assert_eq!(
            rows.iter()
                .map(|s| (s.kind, s.id))
                .collect::<std::collections::HashSet<_>>()
                .len(),
            STOCK_COUNT as usize
        );
        assert!(
            !rows
                .iter()
                .any(|s| (GOODS..GOODS + 8).contains(&(s.id as u32)))
        );
    }
}
