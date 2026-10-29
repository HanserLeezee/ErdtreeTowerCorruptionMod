// 营地调灵：小红帽（补瓶 NPC）的菜单里选“调灵”后弹出骨灰列表，选一个骨灰升到 max(自身, 持有过的最高等级) + 1（最多 +10），
// 低级的好骨灰一次追上来。与补瓶、铁匠共用营地的一次服务（消耗营地强化凭证，凭证没了运行时就结束营地服务）。
// 列表画在叠加层上（原版对话菜单的选项文字只能是预先写好的文本）；选择交给对话的轮询在游戏任务里执行。
use std::sync::{Mutex,atomic::{AtomicU8,Ordering}};
use eldenring::cs::{EquipDataItem,EquipInventoryData,ItemCategory,ItemId,PlayerGameData};
use hudhook::imgui::{Condition,Key,Ui};
use erdtree_tower::rewards::{lookup,spirit_level,spirit_tune_target};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Offer {pub source:u32,pub target:u32}
/// 本局记录的骨灰最高等级（运行时每帧同步）
static PEAK:AtomicU8=AtomicU8::new(0);
pub fn set_peak(peak:u8) {PEAK.store(peak.min(10),Ordering::Relaxed);}

/// 背包里骨灰的最高等级
pub fn inventory_peak(data:&PlayerGameData)->u8 {
    data.equipment.equip_inventory_data.items_data.items()
        .filter(|e|e.quantity>0&&e.item_id.category()==ItemCategory::Goods)
        .filter_map(|e|spirit_level(0x40000000|e.item_id.param_id())).max().unwrap_or(0)
}
fn peak(data:&PlayerGameData)->u8 {PEAK.load(Ordering::Relaxed).max(inventory_peak(data))}
fn choices(owned:&[u32],peak:u8)->Vec<Offer> {
    let mut offers:Vec<Offer>=Vec::new();
    for &source in owned {
        let Some(target)=spirit_tune_target(0x40000000|source,peak).map(|t|t&0x0fffffff) else{continue};
        if owned.contains(&target)||offers.iter().any(|o|o.source==source) {continue;}
        offers.push(Offer{source,target});
    }
    offers.sort_by_key(|o|std::cmp::Reverse(o.target-o.source));
    offers
}
/// 可调灵的骨灰：背包里没到 +10、且还没有持有目标等级的骨灰；升得多的排前面
pub fn offers(data:&PlayerGameData)->Vec<Offer> {
    let owned:Vec<u32>=data.equipment.equip_inventory_data.items_data.items()
        .filter(|e|e.quantity>0&&e.item_id.category()==ItemCategory::Goods).map(|e|e.item_id.param_id())
        .filter(|id|spirit_level(0x40000000|id).is_some()).collect();
    choices(&owned,peak(data))
}
fn name(id:u32)->String {lookup(0x40000000|id).map_or_else(||format!("骨灰 {id}"),|i|i.name.to_owned())}
fn level(id:u32)->u8 {spirit_level(0x40000000|id).unwrap_or(0)}
fn base_name(id:u32)->String {name(erdtree_tower::rewards::spirit_base(0x40000000|id)&0x0fffffff)}

/// 调灵：换成目标等级的骨灰，原来在快捷栏、随身包里的位置换成新骨灰，再消耗营地强化凭证
pub fn tune(data:&mut PlayerGameData,offer:Offer)->Result<(),String> {
    if super::smith::remaining(data)!=1||!offers(data).contains(&offer) {return Err("骨灰或营地强化凭证已变化，请重新选择".into());}
    let remove_fn=super::items::checked_function(0x24d1e0,&[0x48,0x89,0x5c,0x24,0x20,0x89,0x54,0x24,0x10,0x55,0x56,0x57,0x41,0x56,0x41,0x57])?;
    let source=ItemId::new(ItemCategory::Goods,offer.source).map_err(|e|e.to_string())?;
    let target=ItemId::new(ItemCategory::Goods,offer.target).map_err(|e|e.to_string())?;
    let voucher=ItemId::new(ItemCategory::Goods,super::smith::VOUCHER).map_err(|e|e.to_string())?;
    let index=data.equipment.equip_inventory_data.items_data.find_item_idx(source).ok_or("骨灰已不在背包里")? as i32;
    let equipment=&data.equipment;
    let bound=|slots:&[EquipDataItem],ids:&[eldenring::cs::OptionalItemId]|->Vec<usize> {
        slots.iter().zip(ids).enumerate().filter(|(_,(slot,id))|slot.index==index||id.as_valid()==Some(source)).map(|(i,_)|i).collect()
    };
    let quick=bound(&equipment.equip_item_data.quick_slots,&equipment.equipment_entries.quick_tems);
    let pouch=bound(&equipment.equip_item_data.pouch_slots,&equipment.equipment_entries.pouch);
    super::items::give(&[((0x40000000|offer.target) as i32,1,-1)])?;
    if data.equipment.equip_inventory_data.items_data.get_entry(target).is_none() {return Err("调灵后的骨灰未进入背包，没有消耗任何东西".into());}
    let remove:unsafe extern "C" fn(*mut EquipInventoryData,u32,i32)->usize=unsafe {std::mem::transmute(remove_fn)};
    for item in [source,voucher] {
        let inventory=&mut data.equipment.equip_inventory_data;
        let index=inventory.items_data.find_item_idx(item).ok_or("调灵材料发生变化")?;
        unsafe {remove(inventory,index,1);}
    }
    let items=&data.equipment.equip_inventory_data.items_data;
    let (handle,index)=(items.get_entry(target).ok_or("调灵后的骨灰句柄不存在")?.gaitem_handle,items.find_item_idx(target).ok_or("调灵后的骨灰索引不存在")? as i32);
    let equipment=&mut data.equipment;
    for &i in &quick {equipment.equip_item_data.quick_slots[i]=EquipDataItem{gaitem_handle:handle,index};equipment.equipment_entries.quick_tems[i]=target.into();}
    for &i in &pouch {equipment.equip_item_data.pouch_slots[i]=EquipDataItem{gaitem_handle:handle,index};equipment.equipment_entries.pouch[i]=target.into();}
    if super::smith::remaining(data)!=0 {return Err("营地强化凭证消耗未确认，请停止操作".into());}
    set_peak(PEAK.load(Ordering::Relaxed).max(level(offer.target)));
    super::log(format!("Spirit ash tuned: {} -> {} (quick {quick:?}, pouch {pouch:?}), voucher consumed",offer.source,offer.target));
    Ok(())
}

/// 叠加层上的骨灰列表：打开 → 选中（或取消）→ 对话轮询取走结果
enum Picker {Closed,Open{offers:Vec<Offer>,focus:usize,confirm:bool},Done(Option<Offer>)}
static PICKER:Mutex<Picker>=Mutex::new(Picker::Closed);
pub fn open_picker(offers:Vec<Offer>) {if let Ok(mut p)=PICKER.lock(){*p=Picker::Open{offers,focus:0,confirm:false};}}
pub fn active()->bool {PICKER.lock().is_ok_and(|p|matches!(*p,Picker::Open{..}))}
/// None：还在选；Some(None)：取消；Some(Some(offer))：选定
pub fn take_result()->Option<Option<Offer>> {
    let mut p=PICKER.lock().ok()?;
    match std::mem::replace(&mut *p,Picker::Closed) {Picker::Done(result)=>Some(result),Picker::Closed=>Some(None),open=>{*p=open;None}}
}
/// 离开营地或对话中断时关掉列表
pub fn close() {if let Ok(mut p)=PICKER.lock(){if matches!(*p,Picker::Open{..}){*p=Picker::Done(None);}}}

pub fn draw(ui:&Ui) {
    let Ok(mut picker)=PICKER.lock() else{return};
    let Picker::Open{offers,focus,confirm}=&mut *picker else{return};
    let pressed=|keys:&[Key]|keys.iter().any(|k|ui.is_key_pressed(*k));
    let mut result:Option<Option<Offer>>=None;
    if *confirm {
        if pressed(&[Key::Enter,Key::GamepadFaceDown]) {result=Some(Some(offers[*focus]));}
        else if pressed(&[Key::Escape,Key::GamepadFaceRight]) {*confirm=false;}
    } else {
        if pressed(&[Key::UpArrow,Key::W,Key::GamepadDpadUp]) {*focus=(*focus+offers.len()-1)%offers.len();}
        if pressed(&[Key::DownArrow,Key::S,Key::GamepadDpadDown]) {*focus=(*focus+1)%offers.len();}
        if pressed(&[Key::Enter,Key::GamepadFaceDown]) {*confirm=true;}
        else if pressed(&[Key::Escape,Key::GamepadFaceRight]) {result=Some(None);}
    }
    let size=ui.io().display_size;let s=(size[1]/1080.0).max(0.7);
    let gold=[0.92,0.80,0.52,1.0];let dim=[0.72,0.68,0.60,1.0];
    ui.window("调灵##spirit-tune").position([size[0]*0.5,size[1]*0.5],Condition::Always).position_pivot([0.5,0.5])
        .size([560.0*s,0.0],Condition::Always).title_bar(false).resizable(false).movable(false).collapsible(false).bg_alpha(0.94)
        .build(|| {
            let scale=ui.push_style_var(hudhook::imgui::StyleVar::ItemSpacing([8.0*s,8.0*s]));
            ui.text_colored(gold,"调灵 · 强化骨灰");
            ui.text_colored(dim,"选一个骨灰升到「持有过的最高等级 +1」，低级骨灰一次追上来。\n会用掉本次营地服务（不能再补瓶或强化武器、护符）。");
            ui.separator();
            for (i,offer) in offers.iter().enumerate() {
                let label=format!("{}　+{} → +{}##tune{i}",base_name(offer.source),level(offer.source),level(offer.target));
                if ui.selectable_config(&label).selected(i==*focus).size([0.0,30.0*s]).build() {*focus=i;*confirm=true;}
                if ui.is_item_hovered()&&!*confirm {*focus=i;}
            }
            ui.separator();
            if *confirm {
                let offer=offers[*focus];
                ui.text_colored(gold,format!("将「{}」从 +{} 调灵到 +{}？",base_name(offer.source),level(offer.source),level(offer.target)));
                if ui.button_with_size("确认（Enter / A）",[200.0*s,34.0*s]) {result=Some(Some(offer));}
                ui.same_line();
                if ui.button_with_size("返回（Esc / B）",[200.0*s,34.0*s]) {*confirm=false;}
            } else if ui.button_with_size("离开（Esc / B）",[200.0*s,34.0*s]) {result=Some(None);}
            scale.pop();
        });
    if let Some(result)=result {*picker=Picker::Done(result);}
}

#[cfg(test)]
mod tests {
    #[test]
    fn choices_skip_maxed_and_already_owned_targets_and_catch_up_low_ashes() {
        let ash=erdtree_tower::rewards::CATALOG.iter().find(|i|i.kind==erdtree_tower::rewards::ItemKind::Spirit).unwrap().id&0x0fffffff;
        let other=erdtree_tower::rewards::CATALOG.iter().filter(|i|i.kind==erdtree_tower::rewards::ItemKind::Spirit).nth(1).unwrap().id&0x0fffffff;
        let offers=super::choices(&[ash,other+6],4);
        assert_eq!(offers,vec![super::Offer{source:ash,target:ash+5},super::Offer{source:other+6,target:other+7}]);
        assert!(super::choices(&[ash+10],3).is_empty());
        assert!(super::choices(&[ash,ash+5],4).iter().all(|o|o.source!=ash),"the target level is already held");
        assert!(super::choices(&[110000],0).is_empty(),"not a spirit ash");
    }
}
