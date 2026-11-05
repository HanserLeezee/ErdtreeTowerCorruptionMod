use crate::runtime::{self, Action, View};
use erdtree_tower::{Class, NodeKind, Phase, Reward, Run};
use hudhook::imgui::{Condition, ImColor32, StyleColor, Ui};
use std::sync::atomic::{AtomicU64, Ordering};


#[derive(Default)]
struct MapScroll {
    target: Option<f32>,
    anchor: Option<([f32; 2], f32)>,
    dragging: bool,
    last_time: f64,
}

impl MapScroll {
    fn gesture(&mut self, mouse: [f32; 2], start: bool, down: bool, current: f32, max: f32) -> bool {
        if start {
            self.anchor = Some((mouse, current));
            self.dragging = false;
            self.target = Some(current);
        }
        if let Some((origin, scroll)) = self.anchor {
            if (mouse[0]-origin[0]).hypot(mouse[1]-origin[1]) > 6.0 { self.dragging = true; }
            if self.dragging { self.target = Some((scroll + origin[1] - mouse[1]).clamp(0.0, max)); }
        }
        let blocked = self.dragging;
        if !down { self.anchor = None; self.dragging = false; }
        blocked
    }

    fn step(&mut self, current: f32, max: f32, wheel: f32, dt: f32, direct: bool) -> f32 {
        let target = (self.target.unwrap_or(current) - wheel * 120.0).clamp(0.0, max);
        self.target = Some(target);
        if direct || (target-current).abs() < 0.25 { target }
        else { current + (target-current) * (1.0-(-18.0*dt.clamp(0.0,0.1)).exp()) }
    }
}

thread_local! {static SCROLL: std::cell::RefCell<MapScroll> = std::cell::RefCell::new(MapScroll::default());}
thread_local! {
    /// 侧栏图例里鼠标所在的节点类型（侧栏先画，路线图同一帧读取）
    static LEGEND_HOVER:std::cell::Cell<Option<NodeKind>>=const {std::cell::Cell::new(None)};
}
#[cfg(test)]
static TEST_TARGET:std::sync::Mutex<([f32;2],bool)>=std::sync::Mutex::new(([0.0,0.0],false));
#[cfg(test)]
static TEST_FUTURE:std::sync::Mutex<([f32;2],bool)>=std::sync::Mutex::new(([0.0,0.0],false));
#[cfg(test)]
static TEST_ENTER:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(usize::MAX);

#[cfg(test)]
thread_local!{static TEST_ENTERED:std::cell::Cell<usize>=const {std::cell::Cell::new(usize::MAX)};}
fn enter_node(id:usize) {
    #[cfg(test)]
    TEST_ENTER.store(id,Ordering::Relaxed);
    #[cfg(test)]
    TEST_ENTERED.with(|e|e.set(id));
    runtime::submit(Action::Enter(id));
}

#[cfg(test)]
mod tests {
    #[test]
    fn latency_text_distinguishes_host_missing_data_and_measured_zero() {
        use erdtree_tower::party_latency::Status;
        assert_eq!(super::latency_label(&Status::Host),"房主");
        assert_eq!(super::latency_label(&Status::Measured(80)),"80 ms");
        assert_eq!(super::latency_label(&Status::Measured(0)),"0 ms");
        assert_eq!(super::latency_label(&Status::Unavailable),"暂无响应");
        assert_eq!(super::latency_label(&Status::Waiting),"检测中");
        assert_eq!(super::latency_label(&Status::Unsupported),"暂不支持");
    }
    #[test]
    fn legend_names_each_voter_and_marks_the_local_player() {
        assert_eq!(super::legend_line(0,Some("Tarnished".into()),true,false),"1  Tarnished（你）  ·  未选");
        assert_eq!(super::legend_line(2,None,false,true),"3  队员 3  ·  已选");
        assert_eq!(super::VOTER_COLORS.len(),erdtree_tower::party_roster::MAX_PLAYERS);
    }
    #[test]
    fn map_drag_suppresses_release_but_preserves_clicks() {
        let mut s = super::MapScroll::default();
        assert!(!s.gesture([50.0,50.0],true,true,100.0,500.0));
        assert!(!s.gesture([52.0,52.0],false,false,100.0,500.0));
        s.gesture([50.0,50.0],true,true,100.0,500.0);
        assert!(s.gesture([50.0,100.0],false,true,100.0,500.0));
        assert_eq!(s.step(100.0,500.0,0.0,0.016,true),50.0);
        assert!(s.gesture([50.0,50.0],false,false,50.0,500.0));
        assert!(!s.gesture([50.0,50.0],false,false,100.0,500.0));
    }

    #[test]
    fn wheel_scroll_eases_and_clamps_at_both_ends() {
        let mut s = super::MapScroll::default();
        let first=s.step(100.0,500.0,-1.0,1.0/60.0,false);
        assert!(first>100.0 && first<220.0);
        let mut current=first;
        for _ in 0..60 {current=s.step(current,500.0,0.0,1.0/60.0,false);}
        assert_eq!(current,220.0);
        assert_eq!(s.step(current,500.0,-100.0,0.016,true),500.0);
        assert_eq!(s.step(500.0,500.0,100.0,0.016,true),0.0);
    }
    /// Runs inside the map test: imgui allows a single context at a time.
    fn read_only_peek_renders_in_any_phase_and_never_enters_nodes(context:&mut hudhook::imgui::Context) {
        crate::map_art::preview();crate::oath_art::preview();
        context.io_mut().display_size = [1920.0, 1080.0];
        let mut run=erdtree_tower::Run::new(42,erdtree_tower::Class::Samurai);
        let first=run.available()[0];run.enter(first).unwrap();
        let mut view=crate::runtime::view().unwrap();view.ready=true;view.busy=false;
        super::TEST_ENTERED.with(|e|e.set(usize::MAX));super::set_peek(false);super::set_peek(true);super::peek_toggled(-10.0);
        for _ in 0..3 {assert!(!super::draw_peek(context.frame(),&run,&view));assert!(context.render().total_vtx_count>0);}
        context.io_mut().add_key_event(hudhook::imgui::Key::GamepadFaceDown,true);
        super::draw_peek(context.frame(),&run,&view);let data=context.render();
        if let Ok(path)=std::env::var("ERDTREE_UI_PREVIEW") {
            crate::reward_ui::write_peek_frame(&std::path::PathBuf::from(path).join("map-peek-1920.draw"),data,1920,1080);
        }
        context.io_mut().add_key_event(hudhook::imgui::Key::GamepadFaceDown,false);
        assert_eq!(super::TEST_ENTERED.with(|e|e.get()),usize::MAX,"the peek map is read-only");
        context.io_mut().add_key_event(hudhook::imgui::Key::Escape,true);
        assert!(super::draw_peek(context.frame(),&run,&view),"Esc closes the peek");context.render();
        context.io_mut().add_key_event(hudhook::imgui::Key::Escape,false);
        super::draw_peek(context.frame(),&run,&view);context.render();
        // 偷看地图的开关是全局的：测试结束时关上，免得影响其他测试的输入捕获判断
        super::set_peek(false);
    }
    #[test]
    fn full_route_renders_during_loading_and_after_player_ready() {
        let mut context = hudhook::imgui::Context::create();
        context.set_ini_filename(None);
        context.io_mut().display_size = [1920.0, 1080.0];
        context.io_mut().delta_time = 1.0 / 60.0;
        let font=std::fs::read("C:/Windows/Fonts/simhei.ttf").unwrap();
        context.fonts().add_font(&[hudhook::imgui::FontSource::TtfData{data:&font,size_pixels:20.0,config:Some(hudhook::imgui::FontConfig{glyph_ranges:hudhook::imgui::FontGlyphRanges::chinese_full(),..Default::default()})}]);
        context.fonts().build_rgba32_texture();
        crate::reward_ui::test_pad_frames(&mut context);
        let run = erdtree_tower::Run::new(42, erdtree_tower::Class::Samurai);
        let mut view = crate::runtime::view().unwrap();
        for ready in [false, true, true] {
            view.ready = ready;
            super::draw(context.frame(), &run, &view, "HP 455/455");
            assert!(context.render().total_vtx_count > 0);
        }
        // [HansFix] The wander toggle only exists with the wander marker.
        crate::hansfix::set(true,false);
        super::draw(context.frame(), &run, &view, "HP 455/455");context.render();
        super::draw(context.frame(), &run, &view, "HP 455/455");
        let data=context.render();
        if let Ok(path)=std::env::var("ERDTREE_UI_PREVIEW") {
            crate::reward_ui::write_peek_frame(&std::path::PathBuf::from(path).join("map-wander-1920.draw"),data,1920,1080);
        }
        // [HansFix] 堕落·向下：红色的可选关卡与向下次数
        let mut red=erdtree_tower::Run::new(42,erdtree_tower::Class::Samurai);red.set_wander(true).unwrap();
        for _ in 0..6 {
            if red.available().iter().any(|&id|red.descent_choice(id))&&red.descents()>0 {break;}
            let options=red.available();
            let pick=options.iter().copied().find(|&id|red.descent_choice(id)&&red.descents()==0).unwrap_or(options[0]);
            red.enter(pick).unwrap();
            match red.nodes[pick].kind {
                erdtree_tower::NodeKind::Monster|erdtree_tower::NodeKind::Elite=>{red.win_combat().unwrap();red.decline_reward().unwrap();},
                erdtree_tower::NodeKind::Camp=>red.leave_camp().unwrap(),erdtree_tower::NodeKind::Shop=>red.leave_shop().unwrap(),
                erdtree_tower::NodeKind::Chest=>red.finish_chest().unwrap(),erdtree_tower::NodeKind::Event=>red.resolve_event(false).unwrap(),_=>break,
            }
        }
        assert!(red.available().iter().any(|&id|red.descent_choice(id)),"a downward choice is on the map");
        super::draw(context.frame(), &red, &view, "HP 455/455");context.render();
        super::draw(context.frame(), &red, &view, "HP 455/455");
        let data=context.render();
        if let Ok(path)=std::env::var("ERDTREE_UI_PREVIEW") {
            crate::reward_ui::write_peek_frame(&std::path::PathBuf::from(path).join("map-descent-1920.draw"),data,1920,1080);
        }
        // [HansFix] 堕落挑战开启后：黑夜、只能向下、堕落首领点亮
        let mut fall=erdtree_tower::Run::new(42,erdtree_tower::Class::Samurai);fall.set_wander(true).unwrap();
        let camp=fall.red_camp().unwrap();
        let mut prev=vec![usize::MAX;fall.nodes.len()];let mut queue:std::collections::VecDeque<usize>=fall.start_nodes().into();
        for &s in &fall.start_nodes() {prev[s]=s;}
        while let Some(n)=queue.pop_front() {for &m in &fall.nodes[n].next {if prev[m]==usize::MAX {prev[m]=n;queue.push_back(m);}}}
        let mut path=vec![camp];while prev[*path.last().unwrap()]!=*path.last().unwrap() {path.push(prev[*path.last().unwrap()]);}
        path.reverse();
        let clear=|r:&mut erdtree_tower::Run,id:usize| match r.nodes[id].kind {
            erdtree_tower::NodeKind::Monster|erdtree_tower::NodeKind::Elite=>{r.win_combat().unwrap();r.decline_reward().unwrap();},
            erdtree_tower::NodeKind::Camp=>r.leave_camp().unwrap(),erdtree_tower::NodeKind::Shop=>r.leave_shop().unwrap(),
            erdtree_tower::NodeKind::Chest=>r.finish_chest().unwrap(),erdtree_tower::NodeKind::Event=>r.resolve_event(false).unwrap(),_=>{},
        };
        for &id in &path {fall.enter(id).unwrap();clear(&mut fall,id);}
        let mut gray=erdtree_tower::Run::new(42,erdtree_tower::Class::Samurai);gray.set_wander(true).unwrap();
        let first=gray.available()[0];gray.enter(first).unwrap();clear(&mut gray,first);
        let back=path[path.len()-2];fall.enter(back).unwrap();clear(&mut fall,back);
        assert!(fall.fallen_active());
        // 一路向下走到起点那排：下一步就是堕落首领
        while !fall.available().contains(&fall.fallen_id().unwrap()) {let next=fall.available()[0];fall.enter(next).unwrap();clear(&mut fall,next);}
        // 整张地图一屏画完，便于看到下缘之外的堕落首领
        crate::map_art::preview();
        context.io_mut().display_size=[1920.0,2500.0];
        for (name,run) in [("map-fallen-camp-1920.draw",&gray),("map-fallen-active-1920.draw",&fall)] {
            for _ in 0..3 {super::draw(context.frame(), run, &view, "HP 455/455");context.render();}
            super::draw(context.frame(), run, &view, "HP 455/455");let data=context.render();
            if let Ok(path)=std::env::var("ERDTREE_UI_PREVIEW") {crate::reward_ui::write_peek_frame(&std::path::PathBuf::from(path).join(name),data,1920,2500);}
        }
        context.io_mut().display_size=[1920.0,1080.0];
        crate::hansfix::set(false,false);
        crate::reward_ui::test_frames(&mut context);
        crate::run_screen::test_frames(&mut context);
        read_only_peek_renders_in_any_phase_and_never_enters_nodes(&mut context);
        context.io_mut().config_flags.insert(hudhook::imgui::ConfigFlags::NAV_ENABLE_GAMEPAD);
        context.io_mut().backend_flags.insert(hudhook::imgui::BackendFlags::HAS_GAMEPAD);
        // Esc 与 B 都不关路线图（draw 不再返回“关闭”）
        for key in [hudhook::imgui::Key::Escape,hudhook::imgui::Key::GamepadFaceRight] {
            context.io_mut().add_key_event(key,true);
            super::draw(context.frame(),&run,&view,"");
            context.render();
            context.io_mut().add_key_event(key,false);
            super::draw(context.frame(),&run,&view,"");context.render();
        }
        for (width,height) in [(1280,720),(1920,1080),(3440,1440)] {
            crate::map_art::preview();
            context.io_mut().display_size=[width as f32,height as f32];
            for _ in 0..3 {super::draw(context.frame(),&run,&view,"生命 455 / 455    专注 82 / 82");context.render();}
            context.io_mut().mouse_pos=super::TEST_TARGET.lock().unwrap().0;
            for _ in 0..20 {super::draw(context.frame(),&run,&view,"生命 455 / 455    专注 82 / 82");context.render();}
            let target=*super::TEST_TARGET.lock().unwrap();
            assert!(target.1,"available node should respond to pointer hover: {width}x{height}, {:?}",target.0);
            context.io_mut().config_flags.insert(hudhook::imgui::ConfigFlags::NAV_ENABLE_GAMEPAD);
            context.io_mut().backend_flags.insert(hudhook::imgui::BackendFlags::HAS_GAMEPAD);
            context.io_mut().add_key_event(hudhook::imgui::Key::GamepadFaceDown,true);
            super::TEST_ENTER.store(usize::MAX,super::Ordering::Relaxed);
            super::draw(context.frame(),&run,&view,"");context.render();
            assert!(super::TEST_TARGET.lock().unwrap().1,"A must retain the node under the map pointer");
            assert_eq!(super::TEST_ENTER.load(super::Ordering::Relaxed),run.available()[0],"A must activate the pointed node");
            super::TEST_ENTER.store(usize::MAX,super::Ordering::Relaxed);
            super::draw(context.frame(),&run,&view,"");context.render();
            assert_eq!(super::TEST_ENTER.load(super::Ordering::Relaxed),usize::MAX,"Holding A must not repeat travel");
            context.io_mut().add_key_event(hudhook::imgui::Key::GamepadFaceDown,false);
            super::draw(context.frame(),&run,&view,"");context.render();
            let target=super::TEST_TARGET.lock().unwrap().0;
            crate::reward_ui::test_map_pad(context.io_mut(),target,false);
            super::draw(context.frame(),&run,&view,"");context.render();
            crate::reward_ui::test_map_pad(context.io_mut(),target,true);
            context.io_mut().add_mouse_pos_event([0.0,0.0]);
            context.io_mut().add_key_event(hudhook::imgui::Key::GamepadFaceDown,false);
            super::TEST_ENTER.store(usize::MAX,super::Ordering::Relaxed);
            super::draw(context.frame(),&run,&view,"");context.render();
            assert_eq!(super::TEST_ENTER.load(super::Ordering::Relaxed),run.available()[0],"Map must retain controller input when backend events overwrite ImGui input");
            crate::reward_ui::test_map_pad(context.io_mut(),target,true);
            super::TEST_ENTER.store(usize::MAX,super::Ordering::Relaxed);
            super::draw(context.frame(),&run,&view,"");context.render();
            assert_eq!(super::TEST_ENTER.load(super::Ordering::Relaxed),usize::MAX);
            crate::reward_ui::poll_pad(context.io_mut(),false,false);
            super::draw(context.frame(),&run,&view,"");context.render();
            context.io_mut().add_mouse_pos_event(super::TEST_FUTURE.lock().unwrap().0);
            for _ in 0..20 {super::draw(context.frame(),&run,&view,"生命 455 / 455    专注 82 / 82");context.render();}
            assert!(super::TEST_FUTURE.lock().unwrap().1,"future nodes must support hover without unlocking travel");
            for mode in [crate::map_ink::Mode::Draw,crate::map_ink::Mode::Erase] {
                crate::map_ink::test_mode(mode);
                context.io_mut().mouse_pos=super::TEST_TARGET.lock().unwrap().0;
                super::TEST_ENTER.store(usize::MAX,super::Ordering::Relaxed);
                context.io_mut().mouse_down[0]=true;
                super::draw(context.frame(),&run,&view,"");context.render();
                context.io_mut().mouse_down[0]=false;
                super::draw(context.frame(),&run,&view,"");context.render();
                assert_eq!(super::TEST_ENTER.load(super::Ordering::Relaxed),usize::MAX,"Drawing and erasing must never activate a node");
            }
            crate::map_ink::test_mode(crate::map_ink::Mode::Select);
            let before=super::TEST_TARGET.lock().unwrap().0[1];
            context.io_mut().mouse_pos=[width as f32*0.5,height as f32*0.5];
            super::draw(context.frame(),&run,&view,"");context.render();
            context.io_mut().mouse_down[0]=true;
            super::draw(context.frame(),&run,&view,"");context.render();
            context.io_mut().mouse_pos[1]+=100.0;
            for _ in 0..3 {super::draw(context.frame(),&run,&view,"");context.render();}
            assert!((super::TEST_TARGET.lock().unwrap().0[1]-before-100.0).abs()<2.0,"drag should move the map with the pointer");
            context.io_mut().mouse_down[0]=false;
            super::draw(context.frame(),&run,&view,"");context.render();
            super::draw(context.frame(),&run,&view,"生命 455 / 455    专注 82 / 82");
            let data=context.render();assert!(data.total_vtx_count>0);
            if let Ok(path)=std::env::var("ERDTREE_UI_PREVIEW") {
                use std::io::Write;
                let mut f=std::fs::File::create(std::path::Path::new(&path).join(format!("map-{width}-{height}.draw"))).unwrap();
                writeln!(f,"{width} {height}").unwrap();
                for list in data.draw_lists(){for command in list.commands(){if let hudhook::imgui::DrawCmd::Elements{count,cmd_params:p}=command {
                    writeln!(f,"C {} {} {} {} {}",p.texture_id.id(),p.clip_rect[0],p.clip_rect[1],p.clip_rect[2],p.clip_rect[3]).unwrap();
                    for triangle in list.idx_buffer()[p.idx_offset..p.idx_offset+count].chunks_exact(3) {
                        write!(f,"T").unwrap();for index in triangle {let v=list.vtx_buffer()[*index as usize+p.vtx_offset];write!(f," {} {} {} {} {} {} {} {}",v.pos[0],v.pos[1],v.uv[0],v.uv[1],v.col[0],v.col[1],v.col[2],v.col[3]).unwrap();}writeln!(f).unwrap();
                    }
                }}}
            }
        }
    }
}

pub fn class_name(class: Class) -> &'static str {
    match class {
        Class::Vagabond => "守誓骑士",
        Class::Hero => "狂战士",
        Class::Samurai => "影刃",
        Class::Astrologer => "星辉术士",
        Class::Prophet => "祷誓者",
        Class::Confessor => "召唤师",
    }
}


/// [HansFix7] N toggles a read-only copy of the route map at any time during a journey.
static PEEK:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
pub fn peek()->bool {PEEK.load(Ordering::Relaxed)}
static PEEK_FOCUS:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
pub fn set_peek(open:bool) {set_peek_because(open,"key");}
pub fn set_peek_because(open:bool,reason:&str) {if PEEK.swap(open,Ordering::Relaxed)!=open {PEEK_FOCUS.store(open,Ordering::Relaxed);crate::log(format!("MAP_PEEK open={open} reason={reason}"));}}
/// [HansFix] 同一次按键在捕获输入切换时会被游戏再报一次“按下”：开关后 0.4 秒内的再次切换与关闭一律忽略
static PEEK_AT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
pub fn peek_settled(now:f64)->bool {now-f64::from_bits(PEEK_AT.load(Ordering::Relaxed))>=0.4}
pub fn peek_toggled(now:f64) {PEEK_AT.store(now.to_bits(),Ordering::Relaxed);}
/// Read-only map: shows the route, drawings and the current position; nothing can be entered or drawn.
/// Returns true when the player asks to close it (B on a controller).
pub fn draw_peek(ui: &Ui, run: &Run, view: &View)->bool {
    if peek_settled(ui.time())&&(ui.is_key_pressed_no_repeat(hudhook::imgui::Key::GamepadFaceRight)||ui.is_key_pressed_no_repeat(hudhook::imgui::Key::Escape)) {return true;}
    let size=ui.io().display_size;
    let scale=(size[0]/1600.0).min(size[1]/900.0).clamp(0.65,1.6);
    let margin=18.0*scale;
    let _background=ui.push_style_color(StyleColor::WindowBg,[0.04,0.05,0.04,0.96]);
    let _text=ui.push_style_color(StyleColor::Text,[0.84,0.72,0.46,1.0]);
    ui.window("黄金树爬塔##peek").position([0.0,0.0],Condition::Always).size(size,Condition::Always)
        .flags(hudhook::imgui::WindowFlags::NO_NAV)
        .title_bar(false).resizable(false).movable(false).collapsible(false).scroll_bar(false).build(|| {
            crate::oath_art::background(&ui.get_window_draw_list(),size);
            ui.set_window_font_scale(scale);
            ui.set_cursor_pos([margin,margin*0.4]);
            ui.text(format!("只读地图 · 第 {} 幕{} · 按 {} 关闭",run.act,if run.wander {" · 堕落模式"} else {""},crate::hansfix::map_label()));
            ui.set_cursor_pos([margin,margin+18.0*scale]);
            let _paper=ui.push_style_color(StyleColor::ChildBg,[0.84,0.79,0.66,1.0]);
            let _ink=ui.push_style_color(StyleColor::Text,[0.20,0.17,0.11,1.0]);
            ui.child_window("peek-content").flags(hudhook::imgui::WindowFlags::NO_NAV).scroll_bar(false).scrollable(false)
                .size([size[0]-2.0*margin,size[1]-2.0*margin-18.0*scale]).build(|| route_with(ui,run,false,&view.ballots,true));
        });
    false
}
/// 选路阶段的路线图常驻：没有关闭键（手柄 B 原来能关，关掉后手柄没有办法再打开，键盘也一直关不掉）
pub fn draw(ui: &Ui, run: &Run, view: &View, vitals: &str) {
    let size=ui.io().display_size;
    let scale=(size[0]/1600.0).min(size[1]/900.0).clamp(0.65,1.6);
    let left=0.0;let right=236.0*scale;let margin=18.0*scale;
    let gold=[0.84,0.72,0.46,1.0];
    let _background=ui.push_style_color(StyleColor::WindowBg,[0.04,0.05,0.04,0.96]);
    let _text=ui.push_style_color(StyleColor::Text,gold);
    let _button=ui.push_style_color(StyleColor::Button,[0.28,0.23,0.13,1.0]);
    let _hover=ui.push_style_color(StyleColor::ButtonHovered,[0.45,0.35,0.18,1.0]);
    ui.window("黄金树爬塔##route").position([0.0,0.0],Condition::Always).size(size,Condition::Always)
        .flags(hudhook::imgui::WindowFlags::NO_NAV)
        .title_bar(false).resizable(false).movable(false).collapsible(false).scroll_bar(false).build(|| {
            crate::oath_art::background(&ui.get_window_draw_list(),size);
            ui.set_window_font_scale(scale);
            sidebar(ui,run,view,vitals,[size[0]-right,0.0],[right,size[1]],scale);
            ui.set_cursor_pos([left+margin,margin]);
            let _paper=ui.push_style_color(StyleColor::ChildBg,[0.84,0.79,0.66,1.0]);
            let _ink=ui.push_style_color(StyleColor::Text,crate::scenery::ink_rgba(run));
            let _popup=ui.push_style_color(StyleColor::PopupBg,[0.91,0.86,0.72,1.0]);
            ui.child_window("route-content").flags(hudhook::imgui::WindowFlags::NO_NAV).scroll_bar(false).scrollable(false).size([size[0]-left-right-2.0*margin,size[1]-2.0*margin]).build(|| {
                let _disabled=ui.begin_disabled(!view.ready||view.busy);
                match &run.phase {
                    Phase::Reward { .. } => { ui.text("选择战利品以继续旅程。"); }
                    Phase::Dead => { ui.text("旅程结束"); crate::fancy_text::text_wrapped(ui,"角色死亡。重新启动游戏可开始新一局。"); }
                    Phase::Won => { ui.text("登顶 · 本局通关"); }
                    Phase::Map => {route(ui, run, view.ready && !view.busy,&view.ballots);party_legend(ui,&view.ballots,&view.latencies);},
                    Phase::Node { id } => {
                        match run.nodes[*id].kind {
                            NodeKind::Camp | NodeKind::Chest | NodeKind::Shop => { ui.text("使用房间内的原生交互继续。"); }
                            NodeKind::Event => {
                                ui.text("进入事件区域，开启宝匣或与守坛人交谈。");
                            }
                            _ => {}
                        }
                    }
                    _ => { crate::fancy_text::text_wrapped(ui,&view.status); }
                }
            });
            drop(_popup);drop(_ink);drop(_paper);
        });
}
/// 路线图右侧的信息栏：本幕与地区、旅程状态（层数、生命、专注、卢恩）、图例、地图工具、档案与构筑。
/// 暗色面板配金色细边，与旅程档案、选择职业等界面同一套样式；文字按设计字号缩放，不放大位图
fn sidebar(ui:&Ui,run:&Run,view:&View,vitals:&str,origin:[f32;2],extent:[f32;2],scale:f32) {
    use crate::archive_style::{self as style,GOLD,WHITE,MUTED};
    let (x0,w)=(origin[0],extent[0]);let at=|x:f32,y:f32|[x0+x*scale,origin[1]+y*scale];
    let text=|x:f32,y:f32,value:&str,color:[f32;4],px:f32,align:f32| {
        ui.set_window_font_scale(px/20.0*scale);let tw=ui.calc_text_size(value)[0];
        ui.set_cursor_screen_pos([x0+x*scale-tw*align,origin[1]+y*scale]);ui.text_colored(color,value);
    };
    let inner=w/scale;
    {
        let draw=ui.get_window_draw_list();
        draw.add_rect_filled_multicolor(origin,[x0+w,origin[1]+extent[1]],ImColor32::from_rgba(14,12,9,236),ImColor32::from_rgba(14,12,9,236),ImColor32::from_rgba(7,6,5,246),ImColor32::from_rgba(7,6,5,246));
        draw.add_line(origin,[x0,origin[1]+extent[1]],ImColor32::from_rgba(171,143,87,150)).thickness(1.2*scale).build();
    }
    // ── 本幕与地区 ──
    let theme=crate::scenery::theme(run);let (_,_,accent)=crate::scenery::palette(run);
    let accent=ImColor32::to_rgba_f32s(accent);
    text(20.0,16.0,&format!("第 {} 幕",run.act.max(1)),MUTED,14.0,0.0);
    {
        let font=crate::fancy_text::title_font().map(|f|ui.push_font(f));
        ui.set_window_font_scale(if font.is_some(){24.0/32.0}else{1.2}*scale);
        // 名字太长时缩小到放得下
        let tw=ui.calc_text_size(theme.name)[0];if tw>(inner-36.0)*scale {ui.set_window_font_scale(if font.is_some(){24.0/32.0}else{1.2}*scale*(inner-36.0)*scale/tw);}
        ui.set_cursor_screen_pos(at(20.0,38.0));ui.text_colored(accent,theme.name);
        drop(font);
    }
    text(20.0,74.0,theme.region,MUTED,14.0,0.0);
    crate::law_ui::ornament(&ui.get_window_draw_list(),at(inner*0.5,104.0),(inner*0.5-18.0)*scale,scale);
    // ── 旅程状态 ──
    let floors=run.floors().max(1);let floor=run.reached_layer.min(floors);
    let bar=|y:f32,fill:f32,color:[f32;4]| {
        let draw=ui.get_window_draw_list();let (a,b)=(at(20.0,y),at(inner-20.0,y+4.0));
        draw.add_rect(a,b,ImColor32::from_rgba(255,255,255,24)).filled(true).build();
        draw.add_rect(a,[a[0]+(b[0]-a[0])*fill.clamp(0.0,1.0),b[1]],style::rgba(color,0.95)).filled(true).build();
    };
    text(20.0,120.0,"层数",MUTED,14.0,0.0);text(inner-20.0,117.0,&format!("{floor} / {floors}"),WHITE,17.0,1.0);
    bar(144.0,floor as f32/floors as f32,GOLD);
    // 生命与专注：vitals 形如“生命 455/455    专注 82/82”
    let words:Vec<&str>=vitals.split_whitespace().collect();
    let mut y=160.0;
    for pair in words.chunks(2) {
        let [name,value]=pair else{continue};
        let fill=value.split_once('/').and_then(|(a,b)|Some(a.trim().parse::<f32>().ok()?/b.trim().parse::<f32>().ok()?.max(1.0))).unwrap_or(1.0);
        let color=if name.contains("生命"){[0.80,0.30,0.26,1.0]}else if name.contains("专注"){[0.36,0.52,0.86,1.0]}else{GOLD};
        text(20.0,y,name,MUTED,14.0,0.0);text(inner-20.0,y-2.0,value,WHITE,16.0,1.0);bar(y+23.0,fill,color);y+=38.0;
    }
    {let draw=ui.get_window_draw_list();style::coin(&draw,at(28.0,y+9.0),8.0*scale,scale);}
    text(42.0,y,"卢恩",MUTED,14.0,0.0);text(inner-20.0,y-2.0,&view.runes.to_string(),GOLD,16.0,1.0);
    let mut y=y+34.0;
    // 徘徊赐福（单人）：死亡时回到本层重打；实心菱形是剩下的
    if crate::runtime::solo() {
        text(20.0,y,"徘徊赐福",MUTED,14.0,0.0);
        let draw=ui.get_window_draw_list();
        for i in 0..erdtree_tower::RETRIES.max(run.retries) {
            let c=at(inner-26.0-i as f32*20.0,y+10.0);let r=6.5*scale;
            let points=vec![[c[0],c[1]-r],[c[0]+r,c[1]],[c[0],c[1]+r],[c[0]-r,c[1]]];
            if i<run.retries {draw.add_polyline(points,style::rgba(GOLD,1.0)).filled(true).build();}
            else {let mut closed=points.clone();closed.push(points[0]);draw.add_polyline(closed,style::rgba(MUTED,0.6)).thickness(1.2*scale).build();}
        }
        drop(draw);
        y+=30.0;
    }
    crate::law_ui::ornament(&ui.get_window_draw_list(),at(inner*0.5,y),(inner*0.5-18.0)*scale,scale);
    // ── 图例：两列；鼠标移到某一项上，地图上同类型的节点放大高亮（LEGEND_HOVER，路线图同一帧读取）──
    let y=y+14.0;
    text(20.0,y,"图例",MUTED,14.0,0.0);
    LEGEND_HOVER.with(|h|h.set(None));
    for (i,kind) in [NodeKind::Monster,NodeKind::Elite,NodeKind::Camp,NodeKind::Shop,NodeKind::Chest,NodeKind::Event,NodeKind::Boss].into_iter().enumerate() {
        let (cx,cy)=(34.0+(i%2) as f32*(inner*0.5-6.0),y+44.0+(i/2) as f32*40.0);
        let (row_min,row_max)=(at(cx-20.0,cy-18.0),at(cx+inner*0.5-30.0,cy+18.0));
        let hover=ui.is_mouse_hovering_rect(row_min,row_max);
        if hover {LEGEND_HOVER.with(|h|h.set(Some(kind)));}
        let draw=ui.get_window_draw_list();let p=at(cx,cy);
        if hover {
            draw.add_rect(row_min,row_max,style::rgba(GOLD,0.14)).filled(true).rounding(18.0*scale).build();
            draw.add_rect(row_min,row_max,style::rgba(GOLD,0.55)).rounding(18.0*scale).thickness(1.0*scale).build();
        }
        draw.add_circle(p,15.0*scale,ImColor32::from_rgb(203,183,136)).filled(true).build();
        draw.add_circle(p,15.0*scale,ImColor32::from_rgba(120,98,58,200)).thickness(1.0*scale).build();
        if !crate::map_art::icon_scaled(&draw,p,kind,1.0,if kind==NodeKind::Boss{0.36}else{0.56}*scale){glyph(&draw,p,kind,ImColor32::from_rgb(65,49,28));}
        drop(draw);
        text(cx+24.0,cy-9.0,if kind==NodeKind::Boss{"首领"}else{kind.label()},if hover{GOLD}else{WHITE},15.0,0.0);
    }
    let y=y+44.0+4.0*40.0-6.0;
    crate::law_ui::ornament(&ui.get_window_draw_list(),at(inner*0.5,y),(inner*0.5-18.0)*scale,scale);
    // ── 地图工具 ──
    crate::map_ink::toolbar(ui,erdtree_tower::map_drawing::Scope::of(run),at(20.0,y+16.0),[(inner-40.0)*scale,34.0*scale],scale,view.ready&&!view.busy&&run.phase==Phase::Map);
    // [HansFix] 堕落模式开关（第一幕开局）与整局堕落值
    // 放在地图工具的操作说明下面（说明在工具栏下方约 60 处），按钮和“选路 / 划线 / 擦除”同款
    let mut y=y+96.0;
    if crate::hansfix::wander()&&run.phase==Phase::Map&&run.act==1&&run.visited.is_empty() {
        let tone=if !view.ready||view.busy {style::Tone::Disabled} else if run.wander {style::Tone::Primary} else {style::Tone::Normal};
        if style::button(ui,"map-wander",if run.wander {"堕落：开"} else {"堕落：关"},at(20.0,y),at(inner-20.0,y+34.0),tone,0.9*scale) {runtime::submit(Action::ToggleWander);}
        y+=46.0;
    } else if run.wander {
        text(20.0,y,"堕落模式",[0.78,0.20,0.15,1.0],15.0,0.0);y+=26.0;
    }
    if run.wander {
        let n=run.descents();
        text(20.0,y,&format!("堕落值：{n}"),if n>0 {[0.78,0.20,0.15,1.0]} else {WHITE},15.0,0.0);
    }
    // ── 档案与构筑 ──
    let bottom=extent[1]/scale;
    let archive_key=crate::pad::Input::new(crate::pad::Pad::View,crate::options::label(erdtree_tower::options::Hotkey::Archive));
    if style::button_pad(ui,"map-archive","旅程档案",Some(archive_key),at(20.0,bottom-118.0),at(inner-20.0,bottom-72.0),style::Tone::Normal,scale)||ui.is_key_pressed_no_repeat(hudhook::imgui::Key::GamepadBack) {
        crate::run_screen::archive_entry();
    }
    let build_key=crate::pad::Input::new(crate::pad::Pad::Y,crate::options::label(erdtree_tower::options::Hotkey::Build));
    let open=crate::build_panel::open();
    if style::button_pad(ui,"map-build",if open{"收起构筑"}else{"构筑"},Some(build_key),at(20.0,bottom-62.0),at(inner-20.0,bottom-16.0),if open{style::Tone::Primary}else{style::Tone::Normal},scale)
        ||crate::options::pressed(ui,erdtree_tower::options::Hotkey::Build)||ui.is_key_pressed_no_repeat(hudhook::imgui::Key::GamepadFaceUp) {
        crate::build_panel::toggle();
    }
    ui.set_window_font_scale(scale);
}
pub(crate) fn reward_description(run: &Run, reward: &Reward) -> String {
    match reward {
        Reward::Blessing(kind) => super::blessings::description(
            *kind,
            run.blessings
                .iter()
                .find(|b| b.kind == *kind)
                .map_or(1, |b| (b.level + 1).min(3)),
        ),
        Reward::UpgradeWeapon => {
            "右手武器升至背包最高等效等级，再普通 +3 / 失色 +1；保留质变与战灰，满级封顶"
                .into()
        }
        Reward::Gold(amount) => format!("获得 {amount} 卢恩"),
        Reward::FlaskCapacity=>"红、蓝圣杯瓶上限各 +1；本局生效，不补充剩余次数".into(),
        Reward::MemorySlot=>"记忆栏位 +1；魔法与祷告共用，本局生效".into(),
        Reward::StarStream=>"彗星亚兹勒随视角左右瞄准，光束保持水平；本局生效".into(),
        Reward::Ember(ember)=>format!("余烬「{}」：{}；本局生效",ember.name(),ember.effect().replace('\n',"，")),
        Reward::TalismanBuff(id)=>format!("{}：直接生效，不占护符栏位，持续本局",erdtree_tower::rewards::lookup(0x20000000|id).map_or("护符效果",|i|i.name)),
        Reward::SacredTear=>"圣杯滴露 ×1：交给服务 NPC 提升红、蓝瓶恢复量".into(),
        Reward::UpgradeStone=>"强化石 ×1：在营地铁匠或商店强化一次武器（也可升阶护符、调灵骨灰）；可保留".into(),
        Reward::Torch(_)=>"火把：随机补正、词条与法术战技；举起 10 秒释放战技".into(),
        Reward::Item(id)=>erdtree_tower::rewards::lookup(*id).map_or("未知奖励".into(),|item|
            format!("{}：{}（领取实物，按原版要求装备或记忆）",item.kind.label(),item.name)),
    }
}

/// 投票标记的颜色，按队伍名单（Steam ID 排序）的序号取
const VOTER_COLORS:[[u8;3];8]=[[62,114,185],[173,67,68],[53,135,86],[154,102,179],[180,126,42],[52,139,142],[171,91,129],[99,103,114]];
/// 图例里一行的文字：序号、名字（没有 Steam 名字时写“队员 N”）、“你”、是否已选路线
fn legend_line(index:usize,name:Option<String>,local:bool,voted:bool)->String {
    format!("{}  {}{}  ·  {}",index+1,name.unwrap_or_else(||format!("队员 {}",index+1)),if local{"（你）"}else{""},if voted{"已选"}else{"未选"})
}
/// 联机时路线图右上角的队员图例：投票标记上的序号对应谁
fn latency_label(status:&erdtree_tower::party_latency::Status)->String {
    use erdtree_tower::party_latency::Status;
    match status {Status::Host=>"房主".into(),Status::Measured(ms)=>format!("{ms} ms"),Status::Waiting=>"检测中".into(),Status::Unavailable=>"暂无响应".into(),Status::Unsupported=>"暂不支持".into()}
}
fn party_legend(ui:&Ui,ballots:&[erdtree_tower::party_vote::Ballot],latencies:&[erdtree_tower::party_latency::Row]) {
    let players:Vec<_>=if latencies.is_empty(){ballots.iter().map(|b|b.player).collect()}else{latencies.iter().map(|r|r.player).collect()};
    if players.len()<2 {return;}
    let local=crate::seamless::local_player();
    let lines:Vec<_>=players.iter().enumerate().map(|(i,&player)|{
        let mut line=legend_line(i,crate::seamless::player_name(player),local==Some(player),ballots.iter().any(|b|b.player==player&&b.node.is_some()));
        if let Some(row)=latencies.iter().find(|r|r.player==player){line.push_str(&format!("  ·  {}",latency_label(&row.status)));}
        line
    }).collect();
    let header=if latencies.is_empty(){"队伍"}else{"队伍 · 至房主延迟（RTT）"};
    let row=ui.text_line_height()+6.0;
    let width=lines.iter().map(|l|ui.calc_text_size(l)[0]).fold(ui.calc_text_size(header)[0],f32::max)+46.0;
    let (pos,size)=(ui.window_pos(),ui.window_size());
    let (x,y)=(pos[0]+size[0]-width-10.0,pos[1]+10.0);
    let draw=ui.get_window_draw_list();
    draw.add_rect([x,y],[x+width,y+row*(lines.len()+1) as f32+10.0],ImColor32::from_rgba(233,222,196,230)).filled(true).rounding(5.0).build();
    draw.add_rect([x,y],[x+width,y+row*(lines.len()+1) as f32+10.0],ImColor32::from_rgb(143,95,22)).rounding(5.0).build();
    draw.add_text([x+12.0,y+5.0],ImColor32::from_rgb(64,54,38),header);
    for (i,line) in lines.iter().enumerate() {
        let cy=y+5.0+row*(i+1) as f32+row*0.5;let c=VOTER_COLORS[i%VOTER_COLORS.len()];
        draw.add_circle([x+16.0,cy],7.0,ImColor32::from_rgb(c[0],c[1],c[2])).filled(true).build();
        draw.add_text([x+30.0,cy-ui.text_line_height()*0.5],ImColor32::from_rgb(64,54,38),line);
    }
}
fn route(ui: &Ui, run: &Run, ready: bool,ballots:&[erdtree_tower::party_vote::Ballot]) {route_with(ui,run,ready,ballots,ready)}
/// `ready`: nodes can be entered and ink edited; `browse`: the map can still be dragged and scrolled.
fn route_with(ui: &Ui, run: &Run, ready: bool,ballots:&[erdtree_tower::party_vote::Ballot],browse:bool) {
    let selecting=crate::map_ink::selecting()||!ready;
    let available = run.available();
    let origin = ui.cursor_screen_pos();
    let content = ui.content_region_avail();
    let viewport = ui.window_size()[1] - 16.0;
    let (dragged, scroll) = SCROLL.with(|state| {
        let mut state=state.borrow_mut();
        if ui.time()-state.last_time > 0.15 { *state=MapScroll::default(); }
        state.last_time=ui.time();
        let mouse=ui.io().mouse_pos;
        let hovered=ui.is_window_hovered();
        let left=hudhook::imgui::MouseButton::Left;
        if !browse || !selecting || !ui.is_mouse_pos_valid(mouse) {state.anchor=None;state.dragging=false;}
        let blocked=state.gesture(mouse,browse&&selecting&&hovered&&ui.is_mouse_clicked(left),ui.is_mouse_down(left),ui.scroll_y(),ui.scroll_max_y());
        let wheel=if browse&&hovered&&state.anchor.is_none(){ui.io().mouse_wheel}else{0.0};
        let next=state.step(ui.scroll_y(),ui.scroll_max_y(),wheel,ui.io().delta_time,blocked);
        (blocked,next)
    });
    let last=run.floors()-1;
    // [HansFix] 堕落首领在地图下缘之外：高屏幕上也要能一屏看到它
    let reserve=if run.fallen_id().is_some() {230.0} else {0.0};
    let height = (viewport-reserve).max(170.0+last as f32*120.0);
    let row = (height - 170.0) / last as f32;
    let radius = 24.0;
    let span=content[0].min(1060.0);
    let mut reachable = vec![false; run.nodes.len()];
    for id in run.start_nodes() {reachable[id]=true;}
    for id in 0..run.nodes.len() {if reachable[id] {for &next in &run.nodes[id].next {reachable[next]=true;}}}
    let mut peers=vec![0usize;run.nodes.iter().map(|n|n.layer+1).max().unwrap_or(0)];let mut ranks=vec![0usize;run.nodes.len()];
    for (i,n) in run.nodes.iter().enumerate() {if reachable[i] {ranks[i]=peers[n.layer];peers[n.layer]+=1;}}
    // [HansFix] 堕落首领：画在起点那排下方、地图下缘之外
    let fallen=run.fallen_id();let red_camp=run.red_camp();
    let below=if fallen.is_some() {230.0} else {0.0};
    if let Some(f)=fallen {reachable[f]=true;}
    let mut points:Vec<[f32;2]>=(0..run.nodes.len()).map(|id| {
        let layer=run.nodes[id].layer;let count=peers[layer];let rank=ranks[id];
        let center=if count<=1 {0.5}else{0.13+rank as f32*0.74/(count-1) as f32};
        let jitter = ((run.seed.wrapping_add(id as u64 * 7919) % 101) as f32 / 100.0 - 0.5) * 0.065;
        let sway = ((layer as f32 * 1.7).sin()) * 0.045;
        [
            origin[0] + (content[0]-span)*0.5+span * (if count<=1 {0.5}else{center+jitter+sway}),
            origin[1] + 70.0 + (last - layer) as f32 * row,
        ]
    }).collect();
    if let Some(f)=fallen {points[f]=[origin[0]+content[0]*0.5,origin[1]+70.0+last as f32*row+190.0];}
    let point=|id:usize|points[id];
    let draw = ui.get_window_draw_list();
    let (paper_tint,ink,_)=crate::scenery::palette(run);
    let mut hovered=None;let mut pressed=None;
    let pad=crate::reward_ui::map_command();
    let pad_point=pad.and_then(|(point,_)|point);
    let confirm=pad.map_or_else(||ui.is_key_pressed_no_repeat(hudhook::imgui::Key::GamepadFaceDown),|(_,pressed)|pressed);
    let clip_min=ui.window_pos();let clip_max=[clip_min[0]+ui.window_size()[0],clip_min[1]+ui.window_size()[1]];
    for id in (0..run.nodes.len()).filter(|&id|reachable[id]) {
        let pos=point(id);
        ui.set_cursor_screen_pos([pos[0]-36.0,pos[1]-36.0]);
        let enabled=ready&&selecting&&available.contains(&id);
        let clicked=selecting&&ui.invisible_button(format!("node-{id}"),[72.0,72.0]);
        let pointed=if let Some(p)=pad_point {
            ui.is_item_visible()&&(0..2).all(|axis|p[axis]>=clip_min[axis]&&p[axis]<clip_max[axis]&&(p[axis]-pos[axis]).abs()<=36.0)
        }else{ui.is_window_hovered()&&ui.is_item_visible()&&ui.is_mouse_hovering_rect([pos[0]-36.0,pos[1]-36.0],[pos[0]+36.0,pos[1]+36.0])};
        if enabled&&!dragged&&(clicked||pointed&&confirm){enter_node(id);}
        if pointed&&selecting {
            hovered=Some(id);ui.set_mouse_cursor(Some(if enabled{hudhook::imgui::MouseCursor::Hand}else{hudhook::imgui::MouseCursor::Arrow}));
        }
        if ui.is_item_active()&&enabled {pressed=Some(id);}
    }
    thread_local! {static EMPHASIS:std::cell::RefCell<Vec<f32>>=const {std::cell::RefCell::new(Vec::new())};}
    // 图例悬停：同类型的节点放大（legend[id] 0→1），其余节点变淡（dim 0→1）
    thread_local! {static LEGEND_FOCUS:std::cell::RefCell<(Vec<f32>,f32)>=const {std::cell::RefCell::new((Vec::new(),0.0))};}
    let legend_kind=LEGEND_HOVER.with(|h|h.get());
    let (legend,dim)=LEGEND_FOCUS.with(|state| {
        let mut state=state.borrow_mut();let k=(ui.io().delta_time*12.0).min(1.0);
        state.0.resize(run.nodes.len(),0.0);
        for (id,value) in state.0.iter_mut().enumerate() {let target=if legend_kind==Some(run.nodes[id].kind){1.0}else{0.0};*value+=(target-*value)*k;}
        state.1+=(if legend_kind.is_some(){1.0}else{0.0}-state.1)*k;
        state.clone()
    });
    let emphasis=EMPHASIS.with(|values| {
        let mut values=values.borrow_mut();values.resize(run.nodes.len(),0.0);
        for (id,value) in values.iter_mut().enumerate(){let target=if hovered==Some(id){1.0}else{0.0};*value+=(target-*value)*(ui.io().delta_time*14.0).min(1.0);}
        values.clone()
    });
    #[cfg(test)]
    if let Some(&id)=available.first(){*TEST_TARGET.lock().unwrap()=(point(id),hovered==Some(id));}
    #[cfg(test)]
    if let Some(id)=available.first().and_then(|&id|run.nodes[id].next.first()).copied(){*TEST_FUTURE.lock().unwrap()=(point(id),hovered==Some(id));}
    crate::map_art::paper_tinted(&draw,[origin[0]-8.0,origin[1]-8.0],[origin[0]+content[0]+8.0,origin[1]+height+8.0],paper_tint);
    // [HansFix] 地图下缘之外留黑（堕落首领所在处）
    if below>0.0 {draw.add_rect([origin[0]-8.0,origin[1]+height+8.0],[origin[0]+content[0]+8.0,origin[1]+height+below+40.0],ImColor32::from_rgb(10,9,8)).filled(true).build();}
    // [HansFix] 堕落挑战中地图变暗（黑夜）
    if run.fallen_active() {draw.add_rect([origin[0]-8.0,origin[1]-8.0],[origin[0]+content[0]+8.0,origin[1]+height+8.0],ImColor32::from_rgba(12,18,46,105)).filled(true).build();}
    for n in 0..500 {
        let x=origin[0]+(n*71%997) as f32/997.0*content[0];let y=origin[1]+(n*137%1451) as f32/1451.0*height;
        draw.add_line([x,y],[x+3.0,y+1.0],ImColor32::from_rgba(109,87,48,25)).build();
    }
    draw.add_line([origin[0]+39.0,origin[1]+65.0],[origin[0]+39.0,origin[1]+height-80.0],ImColor32::from_rgba(98,81,50,65)).build();
    for floor in 0..run.floors() {
        let y = origin[1] + 60.0 + (last - floor) as f32 * row;
        draw.add_text(
            [origin[0] + 8.0, y - 8.0],
            ImColor32::from_rgba(98, 81, 50, 140),
            format!("{:02}", floor + 1),
        );
    }
    for (id, node) in run.nodes.iter().enumerate() {
        if !reachable[id] { continue; }
        for &next in &node.next {
            let preview=hovered==Some(id)||hovered==Some(next);
            let taken = run.visited.windows(2).any(|w| w == [id, next] || w == [next, id]);
            let a = point(id);
            let b = point(next);
            let curve = |t: f32| {
                let smooth = t * t * (3.0 - 2.0 * t);
                [
                    a[0] + (b[0] - a[0]) * smooth,
                    a[1] + (b[1] - a[1]) * t,
                ]
            };
            let color = if Some(id)==fallen {
                if run.fallen_active() {ImColor32::from_rgb(196,52,40)} else {ImColor32::from_rgba(150,150,150,170)}
            } else if preview {
                ImColor32::from_rgb(191,117,15)
            } else if taken {
                ImColor32::from_rgb(60, 94, 67)
            } else if run.visited.last()==Some(&id)&&available.contains(&next)||run.visited.last()==Some(&next)&&available.contains(&id) {
                // [HansFix] 堕落：通向向下关卡的连线用红色
                let target=if available.contains(&next){next}else{id};
                if run.descent_choice(target) {ImColor32::from_rgb(178,52,40)} else {ImColor32::from_rgb(179,126,32)}
            } else {
                ImColor32::from_rgba(78, 64, 39, 165)
            };
            let steps=(((b[0]-a[0]).abs()+(b[1]-a[1]).abs())/7.0).ceil().max(30.0) as usize;
            for segment in 0..steps {
                let p=curve(segment as f32/steps as f32);let q=curve((segment+1) as f32/steps as f32);
                if (p[0]-a[0]).hypot(p[1]-a[1])<radius+6.0||(q[0]-b[0]).hypot(q[1]-b[1])<radius+6.0 {continue;}
                if taken || segment % 3 != 0 {
                    draw.add_line(
                        p,
                        q,
                        color,
                    )
                    .thickness(if preview {3.8} else if taken { 3.0 } else { 1.5 })
                    .build();
                }
            }
        }
    }
    let descents=run.descent_path();
    for (id, node) in run.nodes.iter().enumerate() {
        if !reachable[id] { continue; }
        let pos = point(id);
        let enabled = ready && available.contains(&id);
        if Some(id)==fallen {
            // 堕落首领：未开启时灰白、不可点；堕落挑战中点亮为暗红
            let lit=run.fallen_active();
            if lit {for (r,a) in [(radius+26.0,45),(radius+19.0,90),(radius+13.0,150)] {draw.add_circle(pos,r,ImColor32::from_rgba(200,40,30,a)).thickness(4.0).build();}}
            if enabled {let pulse=(ui.time() as f32*2.0).sin()*2.5;draw.add_circle(pos,radius+30.0+pulse,ImColor32::from_rgba(226,58,40,160)).thickness(2.0).build();}
            // 图标是深色线稿：灰白底配灰色图标（未开启），红底配暗红图标（开启）
            draw.add_circle(pos,radius+16.0,if lit {ImColor32::from_rgb(238,150,132)} else {ImColor32::from_rgb(208,208,206)}).filled(true).build();
            draw.add_circle(pos,radius+16.0,if lit {ImColor32::from_rgb(170,30,24)} else {ImColor32::from_rgb(140,140,140)}).thickness(3.0).build();
            let tint=if lit {ImColor32::from_rgb(150,22,16)} else {ImColor32::from_rgb(105,105,105)};
            if !crate::map_art::boss_flipped(&draw,pos,0.82,tint) {glyph(&draw,pos,NodeKind::Boss,tint);}
            draw.add_text([pos[0]+radius+46.0,pos[1]-20.0],if lit {ImColor32::from_rgb(235,95,75)} else {ImColor32::from_rgb(190,190,190)},"堕落的首领");
            draw.add_text([pos[0]+radius+46.0,pos[1]+4.0],if lit {ImColor32::from_rgb(200,150,140)} else {ImColor32::from_rgb(140,140,140)},
                if lit {"只能向下 · 打到底部通关本幕"} else {"在红篝火选择走过的相邻关卡开启"});
            continue;
        }
        let focus=emphasis[id];let shown=legend[id];
        let zoom=1.0+0.18*focus+0.45*shown-if pressed==Some(id){0.09}else{0.0};
        // 图例点亮的类型：金色光圈 + 脉冲环
        if shown>0.01 {
            let pulse=(ui.time() as f32*3.0).sin()*0.5+0.5;
            draw.add_circle(pos,radius*zoom+4.0,ImColor32::from_rgba(255,226,150,(shown*150.0) as u8)).filled(true).build();
            draw.add_circle(pos,radius*zoom+6.0+pulse*6.0,ImColor32::from_rgba(196,128,28,(shown*(200.0-pulse*120.0)) as u8)).thickness(2.5).build();
        }
        // 图例点亮其他类型时，这个节点变淡
        let fade=1.0-0.6*dim*(1.0-shown);
        let visited = run.visited.contains(&id);
        // [HansFix] 堕落：向下的关卡（可选的与已走过的）标红，提示难度更高
        let descent=enabled&&run.descent_choice(id);
        let red_visited=visited&&descents.contains(&id);
        let color = if red_visited {
            ImColor32::from_rgb(150, 42, 34)
        } else if visited {
            ImColor32::from_rgb(50, 95, 61)
        } else if enabled {
            ink
        } else {
            ImColor32::from_rgb(139, 122, 88)
        };
        let paper = if descent {
            ImColor32::from_rgb(238, 150, 132)
        } else if enabled {
            ImColor32::from_rgb(232, 202, 127)
        } else {
            ImColor32::from_rgb(215, 202, 169)
        };
        if enabled {
            for ring in 0..4 {
                let (r,g,b)=if descent {(226,58,40)} else {(233,163,43)};
                draw.add_circle(pos,radius+5.0+ring as f32*4.0,ImColor32::from_rgba(r,g,b,(focus*(50.0-ring as f32*10.0)) as u8)).thickness(5.0).build();
            }
            let pulse = (ui.time() as f32 * 2.0).sin() * 2.0;
            draw.add_circle(
                pos,
                radius + 9.0 + pulse,
                if descent {ImColor32::from_rgba(170, 38, 28, 150)} else {ImColor32::from_rgba(149, 102, 26, 115)},
            )
            .thickness(1.5)
            .build();
        }
        if enabled { draw.add_circle(pos, radius*zoom, paper).filled(true).build(); }
        if enabled || run.visited.last()==Some(&id) {
            draw.add_circle(pos, radius*zoom, color).thickness(2.0+focus).build();
        }
        // [HansFix] 红篝火：红色图标加红圈
        if Some(id)==red_camp {
            for (r,a) in [(radius+6.0,120),(radius+12.0,60)] {draw.add_circle(pos,r,ImColor32::from_rgba(200,40,30,a)).thickness(3.0).build();}
            if !crate::map_art::icon_tinted(&draw,pos,node.kind,zoom,ImColor32::from_rgb(255,70,50)){glyph(&draw,pos,node.kind,ImColor32::from_rgb(190,40,30));}
        } else if !crate::map_art::icon_scaled(&draw,pos,node.kind,if enabled||visited{1.0}else{0.8}*fade,zoom){glyph(&draw, pos, node.kind, color);}
        // 节点下方的小牌子：当前位置（深绿底）、最终试炼（深墨底），不再把字直接压在纸和图标上
        let plate=|label:&str,dy:f32,fill:ImColor32| {
            let tw=ui.calc_text_size(label)[0];let (w,h)=(tw+18.0,ui.text_line_height()+6.0);let y=pos[1]+dy;
            draw.add_rect([pos[0]-w*0.5,y],[pos[0]+w*0.5,y+h],fill).filled(true).rounding(h*0.5).build();
            draw.add_rect([pos[0]-w*0.5,y],[pos[0]+w*0.5,y+h],ImColor32::from_rgba(240,226,190,150)).rounding(h*0.5).thickness(1.0).build();
            draw.add_text([pos[0]-tw*0.5,y+3.0],ImColor32::from_rgb(244,234,208),label);
        };
        if run.visited.last()==Some(&id) {plate("当前位置",radius+8.0,ImColor32::from_rgba(44,84,56,235));}
        if node.kind==NodeKind::Boss {plate("最终试炼",radius+22.0,ImColor32::from_rgba(52,38,22,235));}
        if visited {
            draw.add_circle([pos[0] + 18.0, pos[1] + 17.0], 4.0, color)
                .filled(true)
                .build();
        }
        let voters:Vec<_>=ballots.iter().enumerate().filter(|(_,b)|b.node==Some(id)).collect();
        for (offset,(index,_)) in voters.iter().enumerate() {
            let x=pos[0]+(offset as f32-(voters.len()-1) as f32*0.5)*23.0;
            let y=pos[1]-radius-17.0;
            let c=VOTER_COLORS[*index%VOTER_COLORS.len()];
            draw.add_circle([x,y],10.0,ImColor32::from_rgb(c[0],c[1],c[2])).filled(true).build();
            draw.add_text([x-5.0,y-10.0],ImColor32::WHITE,(index+1).to_string());
        }
        if hovered==Some(id) {
                use crate::fancy_text::{self,Effect,Layer,Style};
                let name=if node.kind==NodeKind::Boss {
                    crate::enemies::node_roster(run,id).first().map_or("首领",|enemy|enemy.name).to_owned()
                } else {node.kind.label().to_owned()};
                let affixes=erdtree_tower::boss_affixes::roll(run,id);
                let size=ui.current_font_size();
                let mut parts:Vec<_>=affixes.iter().map(|a|(a.name(),fancy_text::affix_style(*a))).collect();
                parts.push((&name,Style{color:[255,231,168],accent:[255,255,230],effect:if affixes.is_empty(){Effect::Plain}else{Effect::Shimmer},glow:!affixes.is_empty()}));
                let title_width=parts.iter().map(|(text,style)|fancy_text::measure(text,size)+if style.effect==Effect::Afterimage{size*0.4}else{0.0}).sum::<f32>()+fancy_text::measure(" · ",size)*(parts.len()-1) as f32;
                let width=title_width;
                let height=size+16.0;
                draw.add_rect([pos[0]-width*0.5-12.0,pos[1]+39.0],[pos[0]+width*0.5+12.0,pos[1]+39.0+height],ImColor32::from_rgb(45,34,21)).rounding(5.0).filled(true).build();
                fancy_text::draw_line(Layer::Window,pos[0],pos[1]+47.0,size,&parts,(" · ",[180,165,140]),ui.time() as f32);
                draw.add_circle(pos, radius + 5.0, ImColor32::from_rgb(143, 95, 22))
                    .thickness(2.0)
                    .build();
        }
    }
    drop(draw);
    // [HansFix] 画线区域覆盖整张路线图（含下缘外的堕落首领），比地图本身再宽一圈
    crate::map_ink::overlay(ui,erdtree_tower::map_drawing::Scope::of(run),[origin[0]-8.0,origin[1]-8.0],[content[0]+16.0,height+16.0+below],ready);
    ui.set_cursor_screen_pos(origin);
    ui.dummy([content[0], height+below]);
    ui.set_scroll_y(scroll);
    static LAST_ROUTE: AtomicU64 = AtomicU64::new(u64::MAX);
    let signature=run.seed.rotate_left(8)^run.visited.len() as u64
        ^u64::from(ui.io().display_size[0].to_bits()).rotate_left(29)^u64::from(ui.io().display_size[1].to_bits());
    // [HansFix7] When the read-only peek opens, centre it on where the player is.
    let focus=match run.phase {Phase::Combat{id}|Phase::Node{id}|Phase::Reward{id,..}=>Some(id),_=>run.visited.last().copied().or(available.first().copied())};
    if browse&&!ready&&PEEK_FOCUS.swap(false,Ordering::Relaxed)&&let Some(id)=focus {
        ui.set_scroll_y(((last - run.nodes[id].layer) as f32 * row - viewport * 0.5 + 70.0).max(0.0));
        SCROLL.with(|state| *state.borrow_mut()=MapScroll::default());
    } else if LAST_ROUTE.swap(signature, Ordering::Relaxed) != signature {
        if let Some(id) = available.first() {
            ui.set_scroll_y(((last - run.nodes[*id].layer) as f32 * row - viewport * 0.62 + 70.0).max(0.0));
            SCROLL.with(|state| *state.borrow_mut()=MapScroll::default());
        }
    }
}
fn glyph(draw: &hudhook::imgui::DrawListMut<'_>, p: [f32; 2], kind: NodeKind, color: ImColor32) {
    let q = |x: f32, y: f32| [p[0] + x, p[1] + y];
    let line = |a: [f32; 2], b: [f32; 2]| {
        draw.add_line(q(a[0], a[1]), q(b[0], b[1]), color)
            .thickness(2.5)
            .build();
    };
    match kind {
        NodeKind::Monster | NodeKind::Elite => {
            line([-11.0, 12.0], [11.0, -12.0]);
            line([11.0, 12.0], [-11.0, -12.0]);
            line([-12.0, 3.0], [-3.0, 12.0]);
            line([12.0, 3.0], [3.0, 12.0]);
            if kind == NodeKind::Elite {
                draw.add_circle(q(0.0, -3.0), 8.0, color)
                    .thickness(2.0)
                    .build();
            }
        }
        NodeKind::Camp => {
            line([-12.0, 13.0], [12.0, 7.0]);
            line([-12.0, 7.0], [12.0, 13.0]);
            for (a, b) in [
                ([-9.0, 4.0], [-7.0, -6.0]),
                ([-7.0, -6.0], [-2.0, -2.0]),
                ([-2.0, -2.0], [2.0, -14.0]),
                ([2.0, -14.0], [10.0, -2.0]),
                ([10.0, -2.0], [7.0, 5.0]),
                ([7.0, 5.0], [-9.0, 4.0]),
            ] {
                line(a, b);
            }
        }
        NodeKind::Chest => {
            draw.add_rect(q(-13.0, -6.0), q(13.0, 11.0), color)
                .rounding(2.0)
                .thickness(2.0)
                .build();
            line([-13.0, 0.0], [13.0, 0.0]);
            line([-13.0, -6.0], [-8.0, -12.0]);
            line([-8.0, -12.0], [8.0, -12.0]);
            line([8.0, -12.0], [13.0, -6.0]);
            draw.add_rect(q(-2.0, -2.0), q(2.0, 5.0), color)
                .filled(true)
                .build();
        }
        NodeKind::Shop => {
            draw.add_circle(q(0.0, 4.0), 11.0, color)
                .thickness(2.5)
                .build();
            line([-6.0, -6.0], [-9.0, -14.0]);
            line([-9.0, -14.0], [9.0, -14.0]);
            line([9.0, -14.0], [6.0, -6.0]);
            line([-7.0, -6.0], [7.0, -6.0]);
            line([0.0, -1.0], [0.0, 9.0]);
            line([-3.0, 3.0], [3.0, 3.0]);
        }
        NodeKind::Boss => {
            for (a, b) in [
                ([-14.0, -10.0], [-9.0, 10.0]),
                ([-9.0, 10.0], [9.0, 10.0]),
                ([9.0, 10.0], [14.0, -10.0]),
                ([14.0, -10.0], [5.0, -2.0]),
                ([5.0, -2.0], [0.0, -14.0]),
                ([0.0, -14.0], [-5.0, -2.0]),
                ([-5.0, -2.0], [-14.0, -10.0]),
            ] {
                line(a, b);
            }
            line([-8.0, 5.0], [8.0, 5.0]);
        }
        NodeKind::Event => {
            draw.add_text(q(-5.0, -11.0), color, "?");
        }
    }
}
