use eldenring::cs::{ItemCategory,PlayerGameData,EquipParamGoods,SoloParamRepository};
use fromsoftware_shared::{FromStatic,NonEmptyIteratorMutExt};
use std::{sync::Mutex,time::{Duration,Instant}};
pub const FIRST:u32=1130392;
#[derive(Default)]
struct SyncState {last:Option<Instant>,owner:Option<u32>,attempted:[Option<u8>;3]}
impl SyncState {
    fn request(&mut self,index:usize,wanted:u8,missing:u32)->bool {
        if self.attempted[index]==Some(wanted){return false;}
        self.attempted[index]=Some(wanted);missing>0
    }
}
static STATE:Mutex<SyncState>=Mutex::new(SyncState{last:None,owner:None,attempted:[None;3]});
fn capped(held:u32,remaining:u32)->(u32,u32) {let keep=held.min(remaining);(keep,remaining-keep)}
fn trim<'a>(entries:impl Iterator<Item=&'a mut eldenring::cs::EquipInventoryDataListEntry>,remaining:&mut [u32;3]) {
    for entry in entries {
        if entry.item_id.category()!=ItemCategory::Goods {continue;}
        let Some(index)=entry.item_id.param_id().checked_sub(FIRST).filter(|&i|i<3) else{continue;};
        let wanted=&mut remaining[index as usize];(entry.quantity,*wanted)=capped(entry.quantity,*wanted);
    }
}

// Inventory bells mirror the saved run; they never authorize stock increases themselves.
pub fn sync(data:&mut PlayerGameData,counts:[u8;3])->Result<(),String> {
    let mut state=STATE.lock().map_err(|_|"铃珠同步锁不可用")?;
    if state.owner!=Some(data.character_id){*state=SyncState{owner:Some(data.character_id),..Default::default()};}
    if state.last.is_some_and(|t|t.elapsed()<Duration::from_secs(2)){return Ok(());}
    state.last=Some(Instant::now());
    let params=unsafe {SoloParamRepository::instance()}.map_err(|_|"铃珠参数未加载")?;
    if (0..3).any(|i|params.get::<EquipParamGoods>(FIRST+i).is_none_or(|p|p.disable_param_nt())){return Err("铃珠物品参数缺失或禁用，已停止发放，请更新完整资源".into());}
    let mut remaining=counts.map(u32::from);
    for inventory in std::iter::once(&mut data.equipment.equip_inventory_data).chain(data.storage.as_mut().map(|s|&mut **s)) {
        let items=&mut inventory.items_data;
        trim(items.normal_entries_mut().iter_mut().non_empty(),&mut remaining);
        let mut solo=remaining;trim(items.key_entries_mut().iter_mut().non_empty(),&mut solo);
        let mut multi=remaining;trim(items.multiplay_key_entries_mut().iter_mut().non_empty(),&mut multi);
        remaining=std::array::from_fn(|i|solo[i].min(multi[i]));
    }
    for (i,&quantity) in remaining.iter().enumerate() {
        if state.request(i,counts[i],quantity) {
            super::log(format!("MERCHANT_BELL deliver_once kind={i} target={} missing={quantity}",counts[i]));
            super::items::give(&[((0x40000000|FIRST+i as u32) as i32,quantity as i32,-1)])?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bell_ids_survive_the_goods_grant_path_and_match_existing_truncated_items() {
        for i in 0..3 {assert!(FIRST+i<=0x7fffff);assert_eq!(FIRST+i,(9519000+i)&0x7fffff);}
    }
    #[test]
    fn excess_bells_are_trimmed_and_split_stacks_do_not_duplicate_the_allowance() {
        assert_eq!(capped(18,1),(1,0));assert_eq!(capped(99,0),(0,0));
        let (first,left)=capped(2,3);let(second,left)=capped(8,left);
        assert_eq!((first,second,left),(2,1,0));
    }
    #[test]
    fn failed_or_delayed_inventory_confirmation_never_reissues_the_same_award() {
        let mut state=SyncState::default();assert!(state.request(1,1,1));
        for _ in 0..100 {assert!(!state.request(1,1,1));}
        assert!(state.request(1,2,1));assert!(!state.request(1,2,0));
        assert!(!state.request(1,0,0));assert!(state.request(1,1,1));
    }
    #[test]
    fn existing_inventory_marks_the_target_delivered_without_giving_more() {
        let mut state=SyncState::default();assert!(!state.request(0,1,0));assert!(!state.request(0,1,1));
        assert!(state.request(2,1,1));assert!(!state.request(2,1,1));
    }
}
