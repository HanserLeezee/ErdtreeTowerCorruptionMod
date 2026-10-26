// 调试菜单：启动目录里有 tower-debug.enable 时可用，F8 打开。只在单人模式下生效，命令交给运行时在安全时机执行。
use std::{collections::VecDeque,sync::{LazyLock,Mutex,atomic::{AtomicBool,Ordering}}};
use hudhook::imgui::{Condition,Ui};
use erdtree_tower::{NodeKind,Run,contracts::Kind,embers::Ember,law::{Law,NODES}};
use crate::enemies::Enemy;

pub enum Command {Jump(usize),Act(u8),Oath(Kind,u8),Ember(Ember,bool),Law(Law),Item(u32),Heal,Runes(u32)}

static ENABLED:LazyLock<bool>=LazyLock::new(||crate::LOG_PATH.with_file_name("tower-debug.enable").exists());
static OPEN:AtomicBool=AtomicBool::new(false);
static QUEUE:Mutex<VecDeque<Command>>=Mutex::new(VecDeque::new());
static ROSTER:Mutex<Option<Vec<Enemy>>>=Mutex::new(None);

pub fn enabled()->bool {*ENABLED}
pub fn open()->bool {OPEN.load(Ordering::Relaxed)}
pub fn toggle() {
    if !enabled() {return;}
    let now=!OPEN.load(Ordering::Relaxed);OPEN.store(now,Ordering::Relaxed);
    crate::log(format!("DEBUG_MENU open={now}"));
}
fn send(command:Command) {if let Ok(mut queue)=QUEUE.lock(){queue.push_back(command);}}
pub fn take()->Option<Command> {QUEUE.lock().ok()?.pop_front()}
/// 下一场单人战斗改用指定的敌人，生成时取出
pub fn take_roster()->Option<Vec<Enemy>> {ROSTER.lock().ok()?.take()}
fn pending_roster()->Option<String> {ROSTER.lock().ok()?.as_ref().map(|r|r.iter().map(|e|e.name).collect::<Vec<_>>().join("、"))}

#[derive(Default)]
pub struct Menu {enemy_filter:String,item_query:String,items:Vec<(u32,String)>,searched:String,count:i32}
const LEVELS:[&str;4]=["无","I","II","III"];
impl Menu {
    pub fn draw(&mut self,ui:&Ui,view:Option<&crate::runtime::View>) {
        if !open() {return;}
        let size=ui.io().display_size;
        let mut keep=true;
        ui.window("爬塔调试  [F8]").opened(&mut keep).position([size[0]*0.55,size[1]*0.06],Condition::FirstUseEver).size([size[0]*0.42,size[1]*0.86],Condition::FirstUseEver).build(|| {
            let Some(view)=view else {ui.text("运行时尚未就绪");return;};
            ui.text_colored([0.9,0.8,0.5,1.0],&view.status);
            let Some(run)=view.run.as_ref() else {ui.text("尚未开始旅程：在大赐福开始新局后再使用");return;};
            ui.text(format!("第 {} 幕 · 已走 {} 层 · {:?}{}",run.act,run.visited.len(),run.phase,if run.debug{" · 调试旅程"}else{""}));
            if let Some(_bar)=ui.tab_bar("##debug-tabs") {
                if let Some(_tab)=ui.tab_item("战斗") {self.combat(ui,run);}
                if let Some(_tab)=ui.tab_item("构筑") {self.build(ui,run);}
                if let Some(_tab)=ui.tab_item("物品") {self.items(ui);}
                if let Some(_tab)=ui.tab_item("状态") {self.state(ui,run);}
            }
        });
        if !keep {OPEN.store(false,Ordering::Relaxed);}
    }
    fn combat(&mut self,ui:&Ui,run:&Run) {
        ui.text("跳到节点（路线图上会变成可选）");
        ui.child_window("##debug-nodes").size([0.0,220.0]).build(|| {
            for layer in 0..run.floors() {
                ui.text(format!("第 {:>2} 层",layer+1));
                for (id,node) in run.nodes.iter().enumerate().filter(|(_,n)|n.layer==layer) {
                    ui.same_line();
                    let label=format!("{}##node-{id}",node.kind.label());
                    if ui.small_button(label) {send(Command::Jump(id));}
                }
            }
        });
        ui.separator();
        ui.text(format!("下一场战斗的敌人：{}",pending_roster().unwrap_or_else(||"按路线正常抽取".into())));
        if ui.small_button("恢复正常抽取") {if let Ok(mut roster)=ROSTER.lock(){*roster=None;}}
        if self.count<1 {self.count=1;}
        ui.set_next_item_width(120.0);ui.input_int("数量",&mut self.count).build();self.count=self.count.clamp(1,4);
        ui.set_next_item_width(240.0);ui.input_text("筛选##enemy",&mut self.enemy_filter).build();
        let filter=self.enemy_filter.trim().to_owned();
        ui.child_window("##debug-enemies").size([0.0,0.0]).build(|| {
            ui.text_colored([0.7,0.7,0.7,1.0],"怪物池");
            for enemy in crate::enemies::debug_pool().into_iter().filter(|e|filter.is_empty()||e.name.contains(filter.as_str())) {
                let tier=match enemy.kind {NodeKind::Monster=>"普通",NodeKind::Elite=>"强敌",_=>"Boss"};
                let label=format!("第{}幕 {tier} {}{}##pool-{}",enemy.act,enemy.name,if enemy.dlc{"（DLC）"}else{""},enemy.npc);
                if ui.selectable(label) {set_roster(vec![enemy;self.count as usize]);}
            }
            ui.text_colored([0.7,0.7,0.7,1.0],"Boss 组（整组生成）");
            for (index,parts) in crate::enemies::debug_encounters().into_iter().enumerate() {
                let names=parts.iter().map(|e|e.name).collect::<Vec<_>>();
                if !filter.is_empty()&&!names.iter().any(|n|n.contains(filter.as_str())) {continue;}
                let mut unique=names.clone();unique.dedup();
                if ui.selectable(format!("第{}幕 {}（{} 只）##boss-{index}",parts[0].act,unique.join("＋"),parts.len())) {set_roster(parts);}
            }
        });
    }
    fn build(&mut self,ui:&Ui,run:&Run) {
        ui.text("誓约等级");
        for kind in Kind::ALL {
            let level=run.contract.as_ref().and_then(|c|c.level(kind)).unwrap_or(0);
            ui.text(format!("{}：{}",kind.name(),LEVELS[usize::from(level.min(3))]));
            for (target,label) in LEVELS.iter().enumerate() {
                ui.same_line_with_pos(220.0+target as f32*56.0);
                if ui.small_button(format!("{label}##oath-{}-{target}",kind.index())) {send(Command::Oath(kind,target as u8));}
            }
        }
        ui.separator();ui.text("余烬");
        for (i,ember) in Ember::ALL.into_iter().enumerate() {
            if i%3!=0 {ui.same_line_with_pos(10.0+(i%3) as f32*200.0);}
            let mut held=run.embers.contains(&ember);
            if ui.checkbox(format!("{}##ember-{i}",ember.name()),&mut held) {send(Command::Ember(ember,held));}
        }
        ui.separator();
        ui.text(format!("苦难律法：{} 分 · 律法码 {}",run.law.score(),run.law.code()));
        ui.same_line();if ui.small_button("清空律法") {send(Command::Law(Law::default()));}
        // 与律法界面相同：一条路线一行，从左到右依次解锁
        let mut row=None;
        for (id,node) in NODES.iter().enumerate() {
            if row==Some(node.row) {ui.same_line_with_pos(10.0+f32::from(node.col-1)*150.0);} else {row=Some(node.row);}
            let mut has=run.law.has(id);
            let _disabled=ui.begin_disabled(!run.law.available(id));
            if ui.checkbox(format!("{}##law-{id}",node.name),&mut has) {send(Command::Law(run.law.toggle(id)));}
        }
    }
    fn items(&mut self,ui:&Ui) {
        ui.text("按名称或说明搜索，例如：葛孚雷、致命一击、传说·");
        ui.set_next_item_width(320.0);
        let enter=ui.input_text("##item-query",&mut self.item_query).enter_returns_true(true).build();
        ui.same_line();
        if (ui.button("搜索")||enter)||self.searched!=self.item_query&&self.item_query.chars().count()>=2 {
            self.items=crate::reward_ui::search_items(&self.item_query,80);self.searched=self.item_query.clone();
        }
        ui.text(format!("{} 条结果（最多显示 80 条）",self.items.len()));
        ui.child_window("##debug-items").size([0.0,0.0]).build(|| {
            for (id,text) in &self.items {
                if ui.small_button(format!("获得##item-{id}")) {send(Command::Item(*id));}
                ui.same_line();ui.text(text);
            }
        });
    }
    fn state(&mut self,ui:&Ui,run:&Run) {
        ui.text("切换幕（回到该幕起点，生命与圣杯瓶回满）");
        for act in 1..=3u8 {
            if act>1 {ui.same_line();}
            if ui.button(format!("第 {act} 幕")) {send(Command::Act(act));}
        }
        ui.separator();
        if ui.button("回满生命、专注与圣杯瓶") {send(Command::Heal);}
        if ui.button("卢恩 +10000") {send(Command::Runes(10000));}
        ui.same_line();if ui.button("卢恩 +100000") {send(Command::Runes(100000));}
        ui.separator();
        ui.text_wrapped(format!("种子 {}　职业 {}　卢恩 {}　圣杯瓶 {}/{}",run.initial_seed.unwrap_or(run.seed),crate::map_ui::class_name(run.class),run.gold,run.flasks,run.flask_limit));
        ui.text_wrapped("使用过调试功能的旅程会标记为调试旅程：不计入苦难律法纪录，结算页会注明。联机时调试命令不会执行。");
    }
}
fn set_roster(roster:Vec<Enemy>) {
    crate::log(format!("DEBUG_MENU next roster: {}",roster.iter().map(|e|e.name).collect::<Vec<_>>().join(",")));
    if let Ok(mut slot)=ROSTER.lock(){*slot=Some(roster);}
}
