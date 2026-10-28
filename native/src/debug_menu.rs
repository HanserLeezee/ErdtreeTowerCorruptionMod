// 调试菜单：启动目录里有 tower-debug.enable 时可用，F8 打开。只在单人模式下生效，命令交给运行时在安全时机执行。
use std::{collections::VecDeque,sync::{LazyLock,Mutex,atomic::{AtomicBool,Ordering}}};
use hudhook::imgui::{Condition,Ui};
use erdtree_tower::{NodeKind,Run,contracts::Kind,embers::Ember,law::{Law,NODES}};
use crate::enemies::Enemy;

pub enum Command {Jump(usize),Act(u8),Oath(Kind,u8),Ember(Ember,bool),Law(Law),Item(u32),Heal,Runes(u32)}

// tower-debug.enable 文件，或启动参数 debug=1（启动器的「调试模式」）
static ENABLED:LazyLock<bool>=LazyLock::new(||crate::LOG_PATH.with_file_name("tower-debug.enable").exists()||crate::options::get().debug);
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
/// 正在输入的文字框：让输入法的候选框开在它下面（中文输入见 hudhook 的 renderer/ime.rs）
fn ime_here(ui:&Ui) {
    if ui.is_item_active() {let min=ui.item_rect_min();let max=ui.item_rect_max();hudhook::set_ime_position(min[0],max[1]);}
}
fn send(command:Command) {if let Ok(mut queue)=QUEUE.lock(){queue.push_back(command);}}
pub fn take()->Option<Command> {QUEUE.lock().ok()?.pop_front()}
/// 下一场单人战斗改用指定的敌人，生成时取出
pub fn take_roster()->Option<Vec<Enemy>> {ROSTER.lock().ok()?.take()}
fn pending_roster()->Option<String> {ROSTER.lock().ok()?.as_ref().map(|r|r.iter().map(|e|e.name).collect::<Vec<_>>().join("、"))}

/// 斗蛐蛐实验：下一场生成两只敌人，给它们挂上改阵营的效果（scene-builder bout-patch，9680000+阵营值），
/// 玩家在这场里不受伤害，记录哪一只先倒下。用来实机确认怪物能否互相攻击
pub const BOUT_FIRST:i32=9680000;
pub const BOUT_TEAMS:[u8;6]=[29,24,26,27,30,34];
#[derive(Clone,Copy,Debug)]
pub struct BoutSetup {pub team:u8,pub both:bool}
static BOUT:Mutex<Option<BoutSetup>>=Mutex::new(None);
pub fn take_bout()->Option<BoutSetup> {BOUT.lock().ok()?.take()}
#[derive(Default)]
pub struct Bout {setup:Option<BoutSetup>,applied:Vec<eldenring::cs::FieldInsHandle>,started:Option<std::time::Instant>,done:bool,guarded:bool}
impl Bout {
    pub fn begin(&mut self,setup:BoutSetup) {
        crate::log(format!("BOUT begin team={} both={}",setup.team,setup.both));
        *self=Self{setup:Some(setup),..Default::default()};
    }
    pub fn active(&self)->bool {self.setup.is_some()}
    /// 每帧：敌人就位后挂阵营效果，玩家不受伤害；第一只敌人倒下时记下胜者并恢复玩家
    pub fn tick(&mut self,world:&mut eldenring::cs::WorldChrMan,foes:&[(eldenring::cs::FieldInsHandle,&'static str)])->Option<String> {
        use eldenring::cs::ChrInsExt;
        let setup=self.setup?;
        if self.done {return None;}
        if !self.guarded {if let Some(player)=world.main_player.as_mut() {player.chr_ins.debug_flags.set_disabled_hit(true);self.guarded=true;}}
        let effect=BOUT_FIRST+i32::from(setup.team);
        for (index,(handle,name)) in foes.iter().enumerate() {
            if self.applied.contains(handle)||!(setup.both||index==1) {continue;}
            if let Some(chr)=world.chr_ins_by_handle_mut(handle) {chr.apply_speffect(effect,false);self.applied.push(*handle);crate::log(format!("BOUT team {} applied to {name}",setup.team));}
        }
        if foes.len()<2||self.applied.len()<usize::from(setup.both)+1 {return None;}
        let started=*self.started.get_or_insert_with(std::time::Instant::now);
        let alive:Vec<_>=foes.iter().map(|(handle,_)|world.chr_ins_by_handle(handle).is_some_and(|c|c.modules.data.hp>0&&!c.chr_flags1c5.death_flag())).collect();
        let hp:Vec<_>=foes.iter().map(|(handle,_)|world.chr_ins_by_handle(handle).map_or(0,|c|c.modules.data.hp)).collect();
        let message=if alive.iter().filter(|a|!**a).count()>=1 {
            let winner=alive.iter().position(|a|*a).map_or("同归于尽".to_owned(),|i|format!("{} 获胜",foes[i].1));
            format!("斗蛐蛐实验（阵营 {}）：{winner}，用时 {} 秒；现在可以动手清场",setup.team,started.elapsed().as_secs())
        } else if started.elapsed()>std::time::Duration::from_secs(120) {
            format!("斗蛐蛐实验（阵营 {}）：120 秒内没有分出胜负，剩余生命 {:?}；可能没有互相攻击",setup.team,hp)
        } else {return None;};
        self.done=true;self.release(world);crate::log(format!("BOUT result {message}"));Some(message)
    }
    fn release(&mut self,world:&mut eldenring::cs::WorldChrMan) {
        if self.guarded {if let Some(player)=world.main_player.as_mut() {player.chr_ins.debug_flags.set_disabled_hit(false);}self.guarded=false;}
    }
    /// 战斗结束或中断时调用，保证玩家恢复正常
    pub fn end(&mut self,world:&mut eldenring::cs::WorldChrMan) {if self.active() {self.release(world);*self=Self::default();}}
}

#[derive(Default)]
pub struct Menu {enemy_filter:String,item_query:String,items:Vec<(u32,String)>,searched:String,count:i32,bout:[usize;2],bout_team:usize,bout_both:bool}
const LEVELS:[&str;4]=["无","I","II","III"];
impl Menu {
    pub fn draw(&mut self,ui:&Ui,view:Option<&crate::runtime::View>) {
        if !open() {return;}
        let size=ui.io().display_size;
        let mut keep=true;
        ui.window(format!("爬塔调试  [{}]###tower-debug",crate::options::label(erdtree_tower::options::Hotkey::Debug))).opened(&mut keep).position([size[0]*0.55,size[1]*0.06],Condition::FirstUseEver).size([size[0]*0.42,size[1]*0.86],Condition::FirstUseEver).build(|| {
            let Some(view)=view else {ui.text("运行时尚未就绪");return;};
            ui.text_colored([0.9,0.8,0.5,1.0],&view.status);
            let Some(run)=view.run.as_ref() else {ui.text("尚未开始旅程：在大赐福开始新局后再使用");return;};
            ui.text(format!("第 {} 幕 · 已走 {} 层 · {:?}{}",run.act,run.visited.len(),run.phase,if run.debug{" · 调试旅程"}else{""}));
            if let Some(_bar)=ui.tab_bar("##debug-tabs") {
                if let Some(_tab)=ui.tab_item("战斗") {self.combat(ui,run);}
                if let Some(_tab)=ui.tab_item("构筑") {self.build(ui,run);}
                if let Some(_tab)=ui.tab_item("物品") {self.items(ui);}
                if let Some(_tab)=ui.tab_item("状态") {self.state(ui,run);}
                if let Some(_tab)=ui.tab_item("场地") {arenas(ui,run);}
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
        ui.text_colored([0.95,0.75,0.45,1.0],"斗蛐蛐实验：下一场生成两只敌人并改阵营，你在这场不会受伤，请不要攻击");
        let pool=crate::enemies::debug_pool();
        let names:Vec<String>=pool.iter().map(|e|format!("第{}幕 {}",e.act,e.name)).collect();
        for (slot,label) in ["红方##bout-a","蓝方（改阵营）##bout-b"].into_iter().enumerate() {
            ui.set_next_item_width(260.0);ui.combo_simple_string(label,&mut self.bout[slot],&names);
        }
        let teams:Vec<String>=BOUT_TEAMS.iter().map(|t|format!("阵营 {t}")).collect();
        ui.set_next_item_width(120.0);ui.combo_simple_string("##bout-team",&mut self.bout_team,&teams);
        ui.same_line();ui.checkbox("两只都改阵营",&mut self.bout_both);
        ui.same_line();
        if ui.small_button("设为下一场##bout") {
            let (a,b)=(pool[self.bout[0].min(pool.len()-1)],pool[self.bout[1].min(pool.len()-1)]);
            set_roster(vec![a,b]);
            if let Ok(mut bout)=BOUT.lock(){*bout=Some(BoutSetup{team:BOUT_TEAMS[self.bout_team.min(BOUT_TEAMS.len()-1)],both:self.bout_both});}
        }
        ui.separator();
        // 血条颜色标签测试：需要先在 Windows 上运行 text-update 写入测试文字
        let mut tags=crate::boss_affixes::bar_tag_test();
        if ui.checkbox("血条颜色标签测试（主 Boss 血条换成带颜色标签的测试文字）",&mut tags) {crate::boss_affixes::set_bar_tag_test(tags);}
        // 物品名颜色标签测试：text-update 把原版道具“饶舌头颅”（2207）改成带颜色标签的名字，领取后在背包里查看
        ui.same_line();
        if ui.small_button("发放颜色标签测试道具") {
            match crate::items::give(&[(0x40000000|2207,1,-1)]) {Ok(())=>crate::log("Color tag test goods given"),Err(error)=>crate::log(format!("Color tag test goods failed: {error}"))}
        }
        ui.separator();
        ui.text(format!("下一场战斗的敌人：{}",pending_roster().unwrap_or_else(||"按路线正常抽取".into())));
        if ui.small_button("恢复正常抽取") {if let Ok(mut roster)=ROSTER.lock(){*roster=None;}if let Ok(mut bout)=BOUT.lock(){*bout=None;}}
        if self.count<1 {self.count=1;}
        ui.set_next_item_width(120.0);ui.input_int("数量",&mut self.count).build();self.count=self.count.clamp(1,4);
        ui.set_next_item_width(240.0);ui.input_text("筛选##enemy",&mut self.enemy_filter).build();ime_here(ui);
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
                let trial=if parts[0].retired {"（试验）"}else{""};
                if ui.selectable(format!("第{}幕 {}（{} 只）{trial}##boss-{index}",parts[0].act,unique.join("＋"),parts.len())) {set_roster(parts);}
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
        // 传说武器流光：配色只在本次游戏内生效；“预览”让任意近战武器都发光
        ui.text("传说武器流光");
        let labels:Vec<_>=crate::weapon_glow::Style::ALL.iter().map(|s|s.label()).collect();
        let mut index=crate::weapon_glow::Style::ALL.iter().position(|s|*s==crate::weapon_glow::style()).unwrap_or(0);
        ui.set_next_item_width(320.0);
        if ui.combo_simple_string("##glow-style",&mut index,&labels) {crate::weapon_glow::set_style(crate::weapon_glow::Style::ALL[index]);}
        ui.same_line();
        let mut preview=crate::weapon_glow::preview();
        if ui.checkbox("预览：手上任意近战武器都发光",&mut preview) {crate::weapon_glow::set_preview(preview);}
        ui.separator();
        ui.text("按名称或说明搜索，例如：葛孚雷、致命一击、传说·");
        ui.set_next_item_width(320.0);
        let enter=ui.input_text("##item-query",&mut self.item_query).enter_returns_true(true).build();ime_here(ui);
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
        crate::fancy_text::text_wrapped(ui,format!("种子 {}　职业 {}　卢恩 {}　圣杯瓶 {}/{}",run.initial_seed.unwrap_or(run.seed),crate::map_ui::class_name(run.class),run.gold,run.flasks,run.flask_limit));
        crate::fancy_text::text_wrapped(ui,"使用过调试功能的旅程会标记为调试旅程：不计入苦难律法纪录，结算页会注明。联机时调试命令不会执行。");
    }
}
/// 主题幕的战斗场地：查看本幕主题与场地、指定下一战的场地（单人）、记下当前位置（地图块内坐标与朝向）用于调整场地坐标
fn arenas(ui:&Ui,run:&Run) {
    let theme=erdtree_tower::themes::for_run(run);
    let current=crate::arenas::for_run(run,false);
    crate::fancy_text::text_wrapped(ui,format!("本幕主题：{}（{}）　场地键：{}",theme.name,theme.region,if theme.arena.is_empty(){"（用竞技场）"}else{theme.arena}));
    crate::fancy_text::text_wrapped(ui,format!("下一战（单人）：{}　地图块 {:#010x}",current.name,current.block));
    ui.separator();
    ui.text("下一战指定场地（单人；联机始终用竞技场）");
    let chosen=crate::arenas::override_key();
    if ui.radio_button_bool("按主题",chosen.is_none()) {crate::arenas::set_override(None);}
    let built=crate::arenas::built();
    if built.is_empty() {crate::fancy_text::text_wrapped(ui,"还没有生成好的场地：在 Windows 上运行 scene-builder arena-build 后，把输出的 arenas-built.txt 与地图放进 scene-assets。");}
    for (key,name) in built {
        if ui.radio_button_bool(format!("{name}##arena-{key}"),chosen==Some(key)) {crate::arenas::set_override(Some(key));}
    }
    ui.separator();
    if ui.button("记下当前位置") {
        let line=match unsafe {eldenring::cs::PlayerIns::local_player()} {
            Ok(player)=>{
                let p=player.chr_ins.modules.physics.position;let block=i32::from(player.current_block_id).to_be_bytes();
                let yaw=player.chr_ins.modules.physics.orientation.to_euler_angles().1.to_degrees();
                format!("m{:02}_{:02}_{:02}_{:02}|{:.3}|{:.3}|{:.3}|{:.1}",block[0],block[1],block[2],block[3],p.0-player.block_position.x,p.1-player.block_position.y,p.2-player.block_position.z,yaw)
            },
            Err(_)=>"角色未加载".into(),
        };
        let path=crate::LOG_PATH.with_file_name("tower-arena-points.txt");
        let saved=std::fs::OpenOptions::new().create(true).append(true).open(&path).and_then(|mut f|{use std::io::Write;writeln!(f,"{line}")});
        crate::log(format!("ARENA_POINT {line}"));
        POINT.with(|p|*p.borrow_mut()=format!("{line}{}",if saved.is_ok(){"（已写入 tower-arena-points.txt）"}else{"（写入失败，见日志）"}));
    }
    POINT.with(|p|if !p.borrow().is_empty(){crate::fancy_text::text_wrapped(ui,&*p.borrow());});
}
thread_local! {static POINT:std::cell::RefCell<String>=const {std::cell::RefCell::new(String::new())};}
fn set_roster(roster:Vec<Enemy>) {
    crate::log(format!("DEBUG_MENU next roster: {}",roster.iter().map(|e|e.name).collect::<Vec<_>>().join(",")));
    if let Ok(mut slot)=ROSTER.lock(){*slot=Some(roster);}
}
