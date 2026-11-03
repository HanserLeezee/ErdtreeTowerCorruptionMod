// 徘徊赐福的实物道具（道具 7500110，scene-builder/RetryItemAssets.cs）：背包里的数量跟着旅程的 Run::retries 走。
// 单人旅程进行中：背包里应有 retries 个——开局给两个（游戏会弹出获得提示），重打本层时少一个；旅程结束、联机或不在旅程里：收回。
// 发放是异步的（ItemGive），发出后等到账或 3 秒超时才再发，免得连发；收回用游戏自己的“从背包移除”函数（与强化石相同）。
use std::time::{Duration,Instant};
use eldenring::cs::{ItemCategory,ItemId,PlayerGameData,EquipInventoryData};
use erdtree_tower::{Phase,Run};

pub const ITEM:u32=7500110;

/// 背包里的徘徊赐福数量
pub fn held(data:&PlayerGameData)->u32 {
    data.equipment.equip_inventory_data.items_data.items()
        .filter(|e|e.item_id.category()==ItemCategory::Goods&&e.item_id.param_id()==ITEM)
        .map(|e|e.quantity).sum()
}
/// 背包里应有的数量：单人、旅程进行中时为剩余次数，否则为 0
pub fn target(run:Option<&Run>,solo:bool)->u32 {
    match run {
        Some(run) if solo&&!matches!(run.phase,Phase::Dead|Phase::Won)=>u32::from(run.retries),
        _=>0,
    }
}
/// 要做的事：发放几个、收回几个（同一时间只会有一种）
#[derive(Debug,PartialEq,Eq)]
pub enum Step {Give(u32),Take(u32),Wait,Done}
pub fn plan(held:u32,target:u32,giving:Option<Duration>)->Step {
    if held>target {return Step::Take(held-target);}
    if held==target {return Step::Done;}
    match giving {Some(since) if since<Duration::from_secs(3)=>Step::Wait,_=>Step::Give(target-held)}
}

#[derive(Default)]
pub struct Sync {giving:Option<Instant>,checked:Option<Instant>}
impl Sync {
    /// 每帧调用；实际每 0.5 秒检查一次
    pub fn tick(&mut self,data:&mut PlayerGameData,run:Option<&Run>,solo:bool) {
        if self.checked.is_some_and(|t|t.elapsed()<Duration::from_millis(500)) {return;}
        self.checked=Some(Instant::now());
        let (have,want)=(held(data),target(run,solo));
        match plan(have,want,self.giving.map(|t|t.elapsed())) {
            Step::Done=>self.giving=None,
            Step::Wait=>{},
            Step::Give(count)=>{
                match super::items::give(&[((0x40000000|ITEM) as i32,count as i32,-1)]) {
                    Ok(())=>{self.giving=Some(Instant::now());super::log(format!("TOWER_RETRY item give {count} (held {have}, run {want})"));},
                    Err(error)=>super::log_repeating(format!("TOWER_RETRY item give failed: {error}")),
                }
            },
            Step::Take(count)=>{
                self.giving=None;
                match take(data,count) {
                    Ok(())=>super::log(format!("TOWER_RETRY item take {count} (held {have}, run {want})")),
                    Err(error)=>super::log_repeating(format!("TOWER_RETRY item take failed: {error}")),
                }
            },
        }
    }
}
/// 从背包移除 count 个（游戏自己的移除函数，与强化石相同）
fn take(data:&mut PlayerGameData,count:u32)->Result<(),String> {
    let function=super::items::checked_function(0x24d1e0,&[0x48,0x89,0x5c,0x24,0x20,0x89,0x54,0x24,0x10,0x55,0x56,0x57,0x41,0x56,0x41,0x57])?;
    let remove:unsafe extern "C" fn(*mut EquipInventoryData,u32,i32)->usize=unsafe {std::mem::transmute(function)};
    let item=ItemId::new(ItemCategory::Goods,ITEM).map_err(|e|e.to_string())?;
    let inventory=&mut data.equipment.equip_inventory_data;
    let index=inventory.items_data.find_item_idx(item).ok_or("背包里没有徘徊赐福")?;
    unsafe {remove(inventory,index,count as i32);}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_follows_the_run_charges() {
        let mut run=Run::new(42,erdtree_tower::Class::Samurai);
        assert_eq!(target(Some(&run),true),2,"a new solo run holds two");
        assert_eq!(target(Some(&run),false),0,"party runs never hold it");
        assert_eq!(target(None,true),0);
        run.retries=1;assert_eq!(target(Some(&run),true),1);
        run.end_on_death();assert_eq!(target(Some(&run),true),0,"taken back when the run ends");
    }
    #[test]
    fn grants_wait_for_delivery_and_extras_are_taken() {
        assert_eq!(plan(0,2,None),Step::Give(2));
        assert_eq!(plan(0,2,Some(Duration::from_secs(1))),Step::Wait,"ItemGive is asynchronous");
        assert_eq!(plan(0,2,Some(Duration::from_secs(4))),Step::Give(2),"retry after the timeout");
        assert_eq!(plan(2,1,None),Step::Take(1),"one used on a retry");
        assert_eq!(plan(2,0,Some(Duration::from_secs(1))),Step::Take(2));
        assert_eq!(plan(1,1,None),Step::Done);
        assert!(ITEM<0x800000,"goods ID must not overlap the indexed gaitem handle bit");
    }
}
