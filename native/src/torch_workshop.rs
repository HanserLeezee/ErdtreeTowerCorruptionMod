//! [HansFix] 火把工坊：营地铁匠对话里的“火把工坊”打开这个面板。
//! - 合并火把：两把同阶的火把合成一把（火把 → 灼烧的火把 → 黑焰的火把），玩家选保留哪一把的词条与法术战技，补正相加；
//! - 并入武器：火把用掉，武器得到火把的补正（数值相加，每把武器只能并入一次）；
//! - 拆解：名字带火的武器拆成火把（按重量 1~4 把，品质不低于原武器）。
//! 面板只在渲染线程里画；背包清单与实际改动都在游戏线程（runtime 的房间循环）里做。
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use eldenring::cs::{EquipParamWeapon, ItemCategory, PlayerGameData, SoloParamRepository};
use erdtree_tower::torches::{self, Torch};
use fromsoftware_shared::FromStatic;
use hudhook::imgui::{Condition, Ui};

/// 对话菜单里的“火把工坊”（菜单文字表 33 里新加的一条）
pub const TALK_OPTION: i32 = 9500200;
pub const TALK_CHOICE: i32 = 4;

static OPEN: AtomicBool = AtomicBool::new(false);
pub fn open() -> bool { OPEN.load(Ordering::Relaxed) }
pub fn show() { OPEN.store(true, Ordering::Relaxed); REFRESH.store(true, Ordering::Relaxed); }
pub fn close() { OPEN.store(false, Ordering::Relaxed); }
static REFRESH: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, PartialEq)]
pub enum Request { Merge { keep: u16, other: u16 }, IntoWeapon { torch: u16, weapon: u32 }, Dismantle { weapon: u32 } }
static REQUEST: Mutex<Option<Request>> = Mutex::new(None);
fn send(request: Request) { if let Ok(mut r) = REQUEST.lock() { *r = Some(request); } }

/// 背包里的一把武器（游戏线程整理好给面板看）
#[derive(Clone, Debug)]
pub struct BagWeapon { pub id: u32, pub name: String, pub weight: f32, pub quality: u8, pub equipped: bool }
#[derive(Default)]
struct Snapshot { torches: Vec<(Torch, String, bool)>, weapons: Vec<BagWeapon>, merged: Vec<u32>, message: String }
static SNAPSHOT: Mutex<Option<Snapshot>> = Mutex::new(None);

fn weapon_name(id: u32) -> String {
    let base = id / 100 * 100;
    [11, 310, 410].into_iter().find_map(|t| super::hansfix_text::lookup(t, base))
        .or_else(|| [11, 310, 410].into_iter().find_map(|t| super::hansfix_text::lookup(t, erdtree_tower::affixes::base(base))))
        .unwrap_or_else(|| format!("武器 {base}"))
}
fn quality(id: u32) -> u8 {
    let base = id / 100 * 100;
    erdtree_tower::affixes::tier(base).unwrap_or_else(|| super::reward_ui::item_quality(erdtree_tower::affixes::base(base)))
}
fn bag(data: &PlayerGameData) -> Vec<BagWeapon> {
    let Ok(params) = (unsafe { SoloParamRepository::instance() }) else { return Vec::new() };
    let equipped = &data.equipment.chr_asm.gaitem_handles;
    let mut list: Vec<BagWeapon> = Vec::new();
    for entry in data.equipment.equip_inventory_data.items_data.normal_entries().iter().filter_map(|e| e.as_option()) {
        if entry.item_id.category() != ItemCategory::Weapon { continue; }
        let id = entry.item_id.param_id();
        if (50_000_000..60_000_000).contains(&id) { continue; }
        let Some(row) = params.get::<EquipParamWeapon>(id / 100 * 100) else { continue };
        let is_equipped = equipped.contains(&entry.gaitem_handle);
        // 同一种武器只列一次（优先列没装备的那把，拆解与并入时用它）
        if let Some(old) = list.iter_mut().find(|w| w.id / 100 == id / 100) { if old.equipped && !is_equipped { old.id = id; old.equipped = false; } continue; }
        list.push(BagWeapon { id, name: weapon_name(id), weight: row.weight(), quality: quality(id), equipped: is_equipped });
    }
    list
}

/// 游戏线程：整理面板要看的清单，执行面板发来的操作。返回要提示玩家的话
pub fn tick(run: &mut erdtree_tower::Run, data: &mut PlayerGameData) -> Option<Result<String, String>> {
    if !open() { return None; }
    let request = REQUEST.lock().ok().and_then(|mut r| r.take());
    let result = request.map(|request| execute(run, data, request));
    if result.is_some() || REFRESH.swap(false, Ordering::Relaxed) {
        let weapons = bag(data);
        let torches = run.torches.iter().filter_map(|t| {
            let held = weapons.iter().find(|w| torches::slot_of(w.id) == Some(t.slot));
            held.map(|w| (t.clone(), format!("{}{}", t.prefix(), weapon_name(t.base)), w.equipped))
        }).collect();
        let message = match &result { Some(Ok(m)) => m.clone(), Some(Err(e)) => format!("未完成：{e}"), None => String::new() };
        if let Ok(mut s) = SNAPSHOT.lock() { *s = Some(Snapshot { torches, weapons, merged: run.weapon_merges.iter().map(|m| m.weapon).collect(), message }); }
    }
    result
}
fn execute(run: &mut erdtree_tower::Run, data: &mut PlayerGameData, request: Request) -> Result<String, String> {
    super::log(format!("HANSFIX_WORKSHOP {request:?}"));
    let mut next = run.clone();
    let message = match request {
        Request::Merge { keep, other } => {
            let gone = next.torches.iter().find(|t| t.slot == other).cloned().ok_or("火把不存在")?;
            let merged = next.merge_torches(keep, other)?;
            super::weapons::remove_weapon(data, |id| torches::slot_of(id) == Some(gone.slot))?;
            format!("合成了{}{}", merged.prefix(), weapon_name(merged.base))
        }
        Request::IntoWeapon { torch, weapon } => {
            let used = next.torches.iter().find(|t| t.slot == torch).cloned().ok_or("火把不存在")?;
            let merge = next.merge_into_weapon(torch, weapon)?;
            super::weapons::remove_weapon(data, |id| torches::slot_of(id) == Some(used.slot))?;
            format!("{}并入了火把的补正", weapon_name(merge.weapon))
        }
        Request::Dismantle { weapon } => {
            let params = unsafe { SoloParamRepository::instance() }.map_err(|_| "武器参数未就绪")?;
            let row = params.get::<EquipParamWeapon>(weapon / 100 * 100).ok_or("武器参数不存在")?;
            let name = weapon_name(weapon);
            if torches::slot_of(weapon).is_some() || torches::BASES.contains(&(weapon / 100 * 100)) { return Err("火把不能拆解".into()); }
            let got = next.dismantle(&name, row.weight(), quality(weapon))?;
            super::weapons::remove_weapon(data, |id| id == weapon)?;
            for torch in &got { super::torch_proto::give(torch, &next.torches)?; }
            format!("{name}拆成了 {} 把火把", got.len())
        }
    };
    *run = next;
    Ok(message)
}

#[derive(Default)]
pub struct Panel { tab: usize, first: Option<u16>, second: Option<u16>, keep_second: bool, weapon: Option<u32> }
impl Panel {
    pub fn draw(&mut self, ui: &Ui) {
        if !open() { return; }
        let size = ui.io().display_size;
        let mut keep = true;
        let guard = SNAPSHOT.lock();
        let Ok(guard) = guard else { return };
        ui.window("火把工坊###torch-workshop").opened(&mut keep).position([size[0] * 0.5, size[1] * 0.5], Condition::Appearing).position_pivot([0.5, 0.5])
            .size([size[0] * 0.46, size[1] * 0.7], Condition::Appearing).collapsible(false).build(|| {
            let Some(snap) = guard.as_ref() else { ui.text("正在清点背包…"); return; };
            if !snap.message.is_empty() { ui.text_colored([0.95, 0.80, 0.45, 1.0], &snap.message); ui.separator(); }
            if let Some(_bar) = ui.tab_bar("##workshop-tabs") {
                if let Some(_t) = ui.tab_item("合并火把") { if self.tab != 0 { self.reset(0); } self.merge(ui, snap); }
                if let Some(_t) = ui.tab_item("并入武器") { if self.tab != 1 { self.reset(1); } self.into_weapon(ui, snap); }
                if let Some(_t) = ui.tab_item("拆解") { if self.tab != 2 { self.reset(2); } self.dismantle(ui, snap); }
            }
        });
        if !keep || ui.is_key_pressed_no_repeat(hudhook::imgui::Key::Escape) || ui.is_key_pressed_no_repeat(hudhook::imgui::Key::GamepadFaceRight) { close(); }
    }
    fn reset(&mut self, tab: usize) { *self = Panel { tab, ..Default::default() }; }
    fn torch_row(ui: &Ui, torch: &Torch, name: &str, equipped: bool, selected: bool, id: &str) -> bool {
        let spell = torches::SPELLS.get(usize::from(torch.spell)).map_or("", |s| s.name);
        let label = format!("{name}{}　{}　战技：{spell}##{id}{}", if equipped { "（装备中）" } else { "" }, torch.scaling_line(), torch.slot);
        let clicked = ui.selectable_config(&label).selected(selected).build();
        if ui.is_item_hovered() { ui.tooltip(|| { for line in torch.affix_lines() { ui.text(line); } }); }
        clicked
    }
    fn merge(&mut self, ui: &Ui, snap: &Snapshot) {
        ui.text_wrapped("两把同阶的火把合成一把：火把 + 火把 = 灼烧的火把，灼烧的火把 + 灼烧的火把 = 黑焰的火把。补正相加，词条与法术战技保留你选的那一把。");
        ui.separator();
        ui.text("第一把");
        for (torch, name, equipped) in &snap.torches {
            if torch.merges < 2 && Self::torch_row(ui, torch, name, *equipped, self.first == Some(torch.slot), "a") { self.first = Some(torch.slot); self.second = None; }
        }
        let Some(first) = self.first.and_then(|s| snap.torches.iter().find(|(t, ..)| t.slot == s)) else { return };
        ui.separator(); ui.text("第二把（同阶）");
        for (torch, name, equipped) in snap.torches.iter().filter(|(t, ..)| t.slot != first.0.slot && t.merges == first.0.merges) {
            if Self::torch_row(ui, torch, name, *equipped, self.second == Some(torch.slot), "b") { self.second = Some(torch.slot); }
        }
        let Some(second) = self.second.and_then(|s| snap.torches.iter().find(|(t, ..)| t.slot == s)) else { return };
        ui.separator();
        let spell = |t: &Torch| torches::SPELLS.get(usize::from(t.spell)).map_or("", |s| s.name);
        if ui.radio_button_bool(format!("保留第一把的词条与战技（{}）", spell(&first.0)), !self.keep_second) { self.keep_second = false; }
        if ui.radio_button_bool(format!("保留第二把的词条与战技（{}）", spell(&second.0)), self.keep_second) { self.keep_second = true; }
        let (keep, other) = if self.keep_second { (second, first) } else { (first, second) };
        if other.2 { ui.text_colored([0.9, 0.5, 0.4, 1.0], "要用掉的那把正装备着，请先卸下"); return; }
        let scaling = torches::add_scaling(first.0.scaling, second.0.scaling);
        let preview = Torch { scaling, merges: keep.0.merges + 1, ..keep.0.clone() };
        ui.text(format!("结果：{}{}　{}", preview.prefix(), weapon_name(preview.base), preview.scaling_line()));
        if ui.button("合并") { send(Request::Merge { keep: keep.0.slot, other: other.0.slot }); self.reset(0); }
    }
    fn into_weapon(&mut self, ui: &Ui, snap: &Snapshot) {
        ui.text_wrapped("火把并入武器：火把用掉，武器的补正加上火把的补正。每把武器只能并入一次。");
        ui.separator(); ui.text("火把");
        for (torch, name, equipped) in &snap.torches {
            if Self::torch_row(ui, torch, name, *equipped, self.first == Some(torch.slot), "t") { self.first = Some(torch.slot); }
        }
        ui.separator(); ui.text("武器");
        ui.child_window("##workshop-weapons").size([0.0, -40.0]).build(|| {
            for w in snap.weapons.iter().filter(|w| torches::slot_of(w.id).is_none() && !torches::BASES.contains(&(w.id / 100 * 100))) {
                let merged = snap.merged.contains(&(w.id / 100 * 100));
                let _disabled = ui.begin_disabled(merged);
                let label = format!("{}{}##w{}", w.name, if merged { "（已并入过）" } else { "" }, w.id);
                if ui.selectable_config(&label).selected(self.weapon == Some(w.id)).build() { self.weapon = Some(w.id); }
            }
        });
        let (Some(torch), Some(weapon)) = (self.first.and_then(|s| snap.torches.iter().find(|(t, ..)| t.slot == s)), self.weapon) else { return };
        if torch.2 { ui.text_colored([0.9, 0.5, 0.4, 1.0], "这把火把正装备着，请先卸下"); return; }
        if ui.button("并入") { send(Request::IntoWeapon { torch: torch.0.slot, weapon }); self.reset(1); }
    }
    fn dismantle(&mut self, ui: &Ui, snap: &Snapshot) {
        ui.text_wrapped("名字带火的武器可以拆成火把：重量不到 10 拆 1 把，10 到 15 拆 2 把，15 以上拆 3 把，颜面盾拆 4 把。火把的品质不低于原武器。");
        ui.separator();
        let list: Vec<&BagWeapon> = snap.weapons.iter().filter(|w| torches::fire_named(&w.name) && torches::slot_of(w.id).is_none() && !torches::BASES.contains(&(w.id / 100 * 100))).collect();
        if list.is_empty() { ui.text("背包里没有名字带火的武器"); return; }
        for w in list {
            let _disabled = ui.begin_disabled(w.equipped);
            let tier = ["普通", "稀有", "史诗", "传说"][usize::from(w.quality.min(3))];
            let label = format!("{}{}　{tier}　重量 {:.1} → {} 把##d{}", w.name, if w.equipped { "（装备中）" } else { "" }, w.weight, torches::dismantle_count(&w.name, w.weight), w.id);
            if ui.selectable_config(&label).selected(self.weapon == Some(w.id)).build() { self.weapon = Some(w.id); }
        }
        if let Some(weapon) = self.weapon { if ui.button("拆解") { send(Request::Dismantle { weapon }); self.reset(2); } }
    }
}
