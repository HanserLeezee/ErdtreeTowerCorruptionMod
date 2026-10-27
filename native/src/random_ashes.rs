use std::{collections::HashSet,sync::LazyLock};
use eldenring::cs::{EquipParamCustomWeapon,EquipParamGem,SoloParamRepository};
use fromsoftware_shared::FromStatic;

pub const SHOP_FIRST:u32=9500000;
// 没有 DLC 时 DLC 战灰不进池（DLL 启动时已按启动参数设好 DLC 模式，池子第一次用到时才建）
static POOL:LazyLock<Vec<i32>>=LazyLock::new(||include_str!("../../scene-assets/unrestricted-ashes.txt").lines().map(|s|s.parse().unwrap()).filter(|gem:&i32|erdtree_tower::dlc::gem_allowed(*gem as u32)).collect());
static WEAPONS:LazyLock<HashSet<u32>>=LazyLock::new(||include_str!("../../scene-assets/random-ash-weapon-rows.txt").lines().map(|s|s.parse().unwrap()).collect());
static PROTECTED:LazyLock<std::collections::HashMap<u32,Vec<i32>>>=LazyLock::new(||include_str!("../../scene-assets/protected-ash-pools.tsv").lines().map(|line|{
    let p:Vec<_>=line.split('\t').collect();(p[0].parse().unwrap(),p[2].split(',').filter(|s|!s.is_empty()).map(|s|s.parse().unwrap()).filter(|gem:&i32|erdtree_tower::dlc::gem_allowed(*gem as u32)).collect())
}).collect());
pub fn eligible(id:u32)->bool {id>>28==0&&WEAPONS.contains(&(id/100*100))&&PROTECTED.get(&(id/100*100)).is_none_or(|pool|!pool.is_empty())}
fn choose(entropy:u64)->i32 {POOL[(entropy%POOL.len() as u64) as usize]}
fn choose_for(weapon:u32,entropy:u64)->i32 {
    if !eligible(weapon){return -1;}
    PROTECTED.get(&(weapon/100*100)).map_or_else(||choose(entropy),|pool|pool[(entropy%pool.len() as u64) as usize])
}
pub fn preview(run:&erdtree_tower::Run,node:usize,slot:usize,weapon:u32)->i32 {
    if !eligible(weapon){return -1;}
    let mut n=run.reward_seed()^u64::from(run.act).wrapping_mul(0x9e3779b97f4a7c15)^(node as u64).wrapping_mul(0xbf58476d1ce4e5b9)^(slot as u64).wrapping_mul(0x94d049bb133111eb)^u64::from(weapon/100*100)^0x4153485052455631;
    n=(n^(n>>30)).wrapping_mul(0xbf58476d1ce4e5b9);n=(n^(n>>27)).wrapping_mul(0x94d049bb133111eb);choose_for(weapon,n^(n>>31))
}
pub fn description(gem:i32)->Option<(&'static str,&'static str)> {
    static TEXT:LazyLock<std::collections::HashMap<i32,(&str,&str)>>=LazyLock::new(||include_str!("../../scene-assets/random-ash-details.tsv").lines().map(|line|{let mut p=line.splitn(3,'\t');(p.next().unwrap().parse().unwrap(),(p.next().unwrap(),p.next().unwrap()))}).collect());
    TEXT.get(&gem).copied()
}
#[link(name="bcrypt")]
unsafe extern "system" {fn BCryptGenRandom(handle:*mut std::ffi::c_void,buffer:*mut u8,len:u32,flags:u32)->i32;}
fn draw(weapon:u32)->Result<i32,String> {
    let mut entropy=0u64;
    if unsafe {BCryptGenRandom(std::ptr::null_mut(),(&mut entropy as *mut u64).cast(),8,2)}<0{return Err("战灰随机源暂不可用，未发放装备".into());}
    Ok(choose_for(weapon,entropy))
}
fn plan(id:i32,quantity:i32,gem:i32,random:bool,draw:&mut impl FnMut()->Result<i32,String>)->Result<Vec<(i32,i32,i32)>,String> {
    if quantity<=0{return Err("无效物品数量".into());}
    if !random||gem>=0{return Ok(vec![(id,quantity,gem)]);}
    (0..quantity).map(|_|Ok((id,1,draw()?))).collect()
}
pub fn give(entries:&[(i32,i32,i32)])->Result<(),String> {
    if entries.len()>2{return Err("无效物品请求".into());}
    let mut grants=Vec::new();
    for &(id,qty,gem) in entries {grants.extend(plan(id,qty,gem,eligible(id as u32),&mut ||draw(id as u32))?);}
    let params=unsafe {SoloParamRepository::instance()}.map_err(|_|"战灰参数未加载")?;
    for &(_,_,gem) in &grants {if gem>=0&&params.get::<EquipParamGem>(gem as u32).is_none(){return Err("随机战灰参数缺失，未发放装备".into());}}
    for batch in grants.chunks(2){super::items::give(batch)?;}
    Ok(())
}
fn custom_fields(weapon:u32,gem:i32)->(i32,u8,i32) {((weapon/100*100) as i32,(weapon%100) as u8,gem)}
pub fn shop_weapon(params:&mut SoloParamRepository,slot:u32,weapon:u32)->Result<i32,String> {
    if slot>=super::native_shop::STOCK_COUNT{return Err("无效商店槽位".into());}
    let gem=draw(weapon)?;
    if gem<0{return Err("该武器没有兼容随机战灰".into());}
    if params.get::<EquipParamGem>(gem as u32).is_none(){return Err("随机战灰参数缺失".into());}
    let row=params.get_mut::<EquipParamCustomWeapon>(SHOP_FIRST+slot).ok_or("随机战灰商店参数缺失，请完整更新模组")?;
    let (base,level,gem)=custom_fields(weapon,gem);
    row.set_base_wep_id(base);row.set_reinforce_lv(level);row.set_gem_id(gem);
    super::log(format!("Random ash shop offer: slot={slot} weapon={weapon} gem={gem}"));
    Ok((SHOP_FIRST+slot) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shield_and_bow_previews_never_leave_their_native_pool() {
        let run=erdtree_tower::Run::new(42,erdtree_tower::Class::Samurai);
        for line in include_str!("../../scene-assets/protected-ash-pools.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();let id:u32=p[0].parse().unwrap();
            let pool:Vec<i32>=p[2].split(',').filter(|s|!s.is_empty()).map(|s|s.parse().unwrap()).collect();
            if pool.is_empty(){assert!(!eligible(id),"Protected weapon {id} must retain its default skill");}
            for entropy in 0..pool.len(){assert_eq!(choose_for(id,entropy as u64),pool[entropy]);}
            for node in 0..20 {
                let gem=preview(&run,node,0,id);
                assert!(if pool.is_empty(){gem==-1}else{pool.contains(&gem)},"Incompatible gem {gem} for protected weapon {id}");
            }
        }
        for entropy in 0..POOL.len(){assert_eq!(choose_for(1000000,entropy as u64),choose(entropy as u64));}
    }
    #[test]
    fn preview_survives_reload_and_varies_by_reward_slot_and_act() {
        let run=erdtree_tower::Run::new(42,erdtree_tower::Class::Samurai);
        let loaded=erdtree_tower::Run::decode(&run.encode().unwrap()).unwrap();
        assert_eq!(preview(&run,3,0,1000000),preview(&loaded,3,0,1000000));
        let choices:HashSet<_>=(0..100).map(|node|preview(&run,node,0,1000000)).collect();
        assert!(choices.len()>30);
        let slots:HashSet<_>=(0..100).map(|slot|preview(&run,3,slot,1000000)).collect();assert!(slots.len()>30);
        let gem=preview(&run,3,0,1000000);
        assert_eq!(plan(1000025,1,gem,true,&mut ||Err("Must not reroll a previewed reward".into())).unwrap(),vec![(1000025,1,gem)]);
        assert_eq!(preview(&run,3,0,1000000),preview(&run,3,0,1000025));
        assert_eq!(preview(&run,3,0,0x40000000|8590),-1);
        for gem in POOL.iter(){let (name,description)=description(*gem).unwrap();assert!(!name.is_empty()&&!description.is_empty());}
    }
    #[test]
    fn each_acquisition_draws_again_and_explicit_gems_are_preserved() {
        let mut draw=0;
        let mut roll=||{draw+=1;Ok(100+draw)};
        assert_eq!(plan(1000000,2,-1,true,&mut roll).unwrap(),vec![(1000000,1,101),(1000000,1,102)]);
        assert_eq!(plan(1000000,1,999,true,&mut roll).unwrap(),vec![(1000000,1,999)]);
        assert_eq!(plan(50000000,30,-1,false,&mut roll).unwrap(),vec![(50000000,30,-1)]);
        assert_eq!(draw,2);
    }
    #[test]
    fn shop_refresh_preserves_gem_and_affinity_while_updating_level() {
        assert_eq!(custom_fields(160001215,400000),(160001200,15,400000));
        assert_eq!(custom_fields(160001218,400000),(160001200,18,400000));
    }
    #[test]
    fn base_and_dlc_ashes_are_all_reachable_without_weapon_type_filtering() {
        assert!(POOL.len()>100);assert!(POOL.iter().any(|id|*id>=400000));
        let seen:std::collections::HashSet<_>=(0..POOL.len()).map(|n|choose(n as u64)).collect();
        assert_eq!(seen.len(),POOL.len());
        assert!(WEAPONS.contains(&1000000));
        assert!(!WEAPONS.contains(&110000));
        assert!(!WEAPONS.contains(&50000000));
    }
}
