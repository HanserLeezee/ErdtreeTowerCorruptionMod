use std::{collections::VecDeque,sync::{Mutex,atomic::{AtomicBool,Ordering}}};
use erdtree_tower::{Class,Run};
#[cfg(test)]
use erdtree_tower::Phase;
use hudhook::imgui::{Ui,Key,Condition,StyleColor,TextureId};
use erdtree_tower::law::Law;
use erdtree_tower::{journal::{Outcome,Record},legacy::Gain};

static SCREEN:Mutex<VecDeque<Screen>>=Mutex::new(VecDeque::new());
static PENDING:Mutex<Option<(Record,Option<Gain>)>>=Mutex::new(None);
static SEED:Mutex<Option<u64>>=Mutex::new(None);
static LAW_START:Mutex<Option<(Option<u64>,Law,Class)>>=Mutex::new(None);
/// 客机开局：选完职业后等待房主的苦难律法同步。
static CLASS_START:Mutex<Option<Class>>=Mutex::new(None);
static ACTIVE:AtomicBool=AtomicBool::new(false);
enum Screen {Summary{record:Record,gain:Option<Gain>,from_archive:bool,focus:usize,opened:Option<f64>},Archive(Box<crate::archive_ui::Archive>),Notice{message:String,opened:Option<f64>},TrialRewards{items:Vec<u32>,opened:Option<f64>},Seed{input:String,error:String,digit:u8,opened:Option<f64>},Law{screen:crate::law_ui::LawScreen,seed:Option<u64>,record:Option<(u32,String)>,class:Class},
    /// 选择职业；law 为 false 时（客机）选完直接开局
    Class{screen:crate::class_ui::ClassScreen,seed:Option<u64>,law:bool}}
fn show(next:Screen) {
    let Ok(mut screens)=SCREEN.lock() else{return;};
    if screens.iter().any(|s|match (s,&next) {(Screen::Notice{message:a,..},Screen::Notice{message:b,..})=>a==b,(Screen::Seed{..},Screen::Seed{..})|(Screen::Law{..},Screen::Law{..})|(Screen::Class{..},Screen::Class{..})=>true,_=>false}) {return;}
    screens.push_back(next);ACTIVE.store(true,Ordering::Relaxed);
}
pub fn party_notice(message:String) {show(Screen::Notice{message,opened:None});}
pub fn trial_rewards(before:&Run,after:&Run) {
    let items=after.elite_buffs.iter().copied().filter(|id|!before.elite_buffs.contains(id)).collect();
    show(Screen::TrialRewards{items,opened:None});
}
pub fn active()->bool {ACTIVE.load(Ordering::Relaxed)}
pub fn trial_rewards_active()->bool {SCREEN.lock().is_ok_and(|s|s.iter().any(|s|matches!(s,Screen::TrialRewards{..})))}
#[cfg(test)]
pub fn summary(run:&Run) {summary_with((Record::new(run,vec![],vec![],0),None));}
/// 整局结束：回到大赐福后显示这条结算记录（附带这一局的职业熟练度变化）
pub fn summary_with(summary:(Record,Option<Gain>)) {if let Ok(mut pending)=PENDING.lock(){*pending=Some(summary);}}
/// 打开旅程档案（往期旅程与图鉴）；其他界面打开时盖在它们上面，关闭后回到原界面
pub fn archive_entry() {
    open_archive(false);
}
pub fn progression_entry() {
    open_archive(true);
}
fn open_archive(progression:bool) {
    let Ok(mut screens)=SCREEN.lock() else{return;};
    if screens.iter().any(|s|matches!(s,Screen::Archive(_))) {return;}
    let records=crate::run_log::records();
    let archive=if progression {crate::archive_ui::Archive::progression(records)}else{crate::archive_ui::Archive::new(records)};
    screens.push_front(Screen::Archive(Box::new(archive)));ACTIVE.store(true,Ordering::Relaxed);
}
pub fn hub_ready() {
    if let Ok(mut screen)=SCREEN.lock() {
        if !screen.is_empty(){return;}
        if let Some((record,gain))=PENDING.lock().ok().and_then(|mut pending|pending.take()) {
            screen.push_back(Screen::Summary{record,gain,from_archive:false,focus:0,opened:None});ACTIVE.store(true,Ordering::Relaxed);
        }
    }
}
pub fn seed_entry() {show(Screen::Seed{input:String::new(),error:String::new(),digit:0,opened:None});}
pub fn take_seed()->Option<u64> {SEED.lock().ok()?.take()}
/// 开局：先选职业（默认选中 current），单人或房主再立苦难律法；seed 为空时随机开局
pub fn class_entry(seed:Option<u64>,law:bool,current:Class) {
    let focus=Class::PLAYABLE.iter().position(|c|*c==current).unwrap_or(0);
    show(Screen::Class{screen:crate::class_ui::ClassScreen::new(focus),seed,law});
}
fn law_screen(seed:Option<u64>,class:Class)->Screen {
    let mut screen=crate::law_ui::LawScreen::new(Law::default());screen.seed=seed;
    let record=crate::legacy::owner().and_then(|owner|crate::law::best(owner,class)).map(|best|(best,crate::map_ui::class_name(class).to_owned()));
    Screen::Law{screen,seed,record,class}
}
pub fn take_law_start()->Option<(Option<u64>,Law,Class)> {LAW_START.lock().ok()?.take()}
pub fn take_class_start()->Option<Class> {CLASS_START.lock().ok()?.take()}
fn parse_seed(text:&str)->Result<u64,&'static str> {
    let text=text.trim();
    if text.is_empty()||!text.bytes().all(|c|c.is_ascii_digit()){return Err("请输入数字种子");}
    text.parse().map_err(|_|"种子超出范围")
}
pub fn draw(ui:&Ui,icon:&mut dyn FnMut(u32)->Option<TextureId>)->bool {
    let Ok(mut screen)=SCREEN.lock() else{return false;};
    let Some(current)=screen.front_mut() else{return false;};
    let mut next=None;
    let done=match current {
        Screen::Law{screen:law,seed,record,class}=>match crate::law_ui::draw(ui,law,record.as_ref().map(|(best,class)|(*best,class.as_str()))) {
            Some(crate::law_ui::LawAction::Start(law))=>{if let Ok(mut start)=LAW_START.lock(){*start=Some((*seed,law,*class));}true},
            // 返回：回到选择职业
            Some(crate::law_ui::LawAction::Back)=>{next=Some(Screen::Class{screen:crate::class_ui::ClassScreen::new(Class::PLAYABLE.iter().position(|c|c==class).unwrap_or(0)),seed:*seed,law:true});true},
            Some(crate::law_ui::LawAction::Archive)=>{next=Some(Screen::Archive(Box::new(crate::archive_ui::Archive::new(crate::run_log::records()))));false},
            None=>false,
        },
        Screen::Class{screen:classes,seed,law}=>match crate::class_ui::draw(ui,classes) {
            Some(crate::class_ui::ClassAction::Pick(index))=>{
                let class=Class::PLAYABLE[index.min(Class::PLAYABLE.len()-1)];
                if *law {next=Some(law_screen(*seed,class));}
                else if let Ok(mut start)=CLASS_START.lock() {*start=Some(class);}
                true
            },
            Some(crate::class_ui::ClassAction::Back)=>true,
            None=>false,
        },
        Screen::Archive(archive)=>match crate::archive_ui::draw(ui,archive,icon) {
            Some(crate::archive_ui::ArchiveAction::Close)=>true,
            Some(crate::archive_ui::ArchiveAction::Open(record))=>{next=Some(Screen::Summary{record,gain:None,from_archive:true,focus:0,opened:None});false},
            None=>false,
        },
        _=>render(ui,current,icon),
    };
    if done {screen.pop_front();}
    if let Some(next)=next {screen.push_front(next);}
    ACTIVE.store(!screen.is_empty(),Ordering::Relaxed);
    true
}
fn render(ui:&Ui,current:&mut Screen,icon:&mut dyn FnMut(u32)->Option<TextureId>)->bool {
    if let Screen::Summary{record,gain,from_archive,focus,opened}=current {let opened=*opened.get_or_insert(ui.time());return summary_page(ui,record,*gain,focus,*from_archive,icon,ui.time()-opened>=0.35);}
    if let Screen::Seed{input,error,digit,opened}=current {let opened=*opened.get_or_insert(ui.time());return seed_page(ui,input,error,digit,ui.time()-opened>=0.35);}
    let size=ui.io().display_size;
    crate::oath_art::background(&ui.get_background_draw_list(),size);
    let mut close=false;
    let _background=ui.push_style_color(StyleColor::WindowBg,[0.035,0.035,0.025,0.95]);
    let _text=ui.push_style_color(StyleColor::Text,[0.85,0.76,0.52,1.0]);
    ui.window("旅程##run-screen").position([size[0]*0.5,size[1]*0.5],Condition::Always).position_pivot([0.5,0.5])
        .size([620.0f32.min(size[0]*0.9),510.0f32.min(size[1]*0.9)],Condition::Always).title_bar(false).resizable(false).movable(false).build(|| {
        ui.dummy([0.0,15.0]);
        match current {
            Screen::Notice{message,opened}=>{
                let opened=*opened.get_or_insert(ui.time());
                ui.text("暂时无法开始联机旅程");ui.separator();ui.spacing();crate::fancy_text::text_wrapped(ui,message);
                ui.spacing();ui.text("详细原因已记录到 tower-preview.log。");ui.spacing();
                if (ui.button("确定 [Enter / A]")||ui.is_key_pressed(Key::Enter)||ui.is_key_pressed(Key::GamepadFaceDown)||ui.is_key_pressed(Key::Escape)||ui.is_key_pressed(Key::GamepadFaceRight))&&ui.time()-opened>=0.35 {close=true;}
            },
            Screen::TrialRewards{items,opened}=>{
                let opened=*opened.get_or_insert(ui.time());
                ui.text("男生女生向前冲 · 挑战完成");ui.separator();ui.spacing();
                ui.text("已获得以下护符赐福");ui.spacing();
                for &id in items.iter() {
                    let (name,info)=crate::reward_ui::talisman_description(id);
                    ui.text_colored([0.9,0.78,0.5,1.0],name);crate::fancy_text::text_wrapped(ui,info);ui.spacing();ui.separator();ui.spacing();
                }
                if items.is_empty(){crate::fancy_text::text_wrapped(ui,"已拥有全部可获得的护符赐福，本次没有新增赐福。");}
                crate::fancy_text::text_wrapped(ui,"本局持续生效，不占护符栏位，无需装备。");ui.spacing();
                let ready=ui.time()-opened>=0.35;
                if (ui.button("确定，继续选路  [Enter / A]")||ui.is_key_pressed(Key::Enter)||ui.is_key_pressed(Key::GamepadFaceDown))&&ready {close=true;}
            },
            Screen::Summary{..}|Screen::Seed{..}|Screen::Law{..}|Screen::Class{..}|Screen::Archive(_)=>unreachable!(),
        }
    });
    close
}
#[cfg(test)]
pub fn test_frames(context:&mut hudhook::imgui::Context) {
    let mut run=Run::new(u64::MAX,erdtree_tower::Class::Samurai);run.phase=Phase::Dead;
    summary(&run);
    assert!(!active(),"Death must only queue the result, not capture input");
    assert!(!draw(context.frame(),&mut |_|None),"Death and loading must remain unobstructed");context.render();
    hub_ready();
    assert!(active());
    assert!(PENDING.lock().unwrap().is_none());
    SCREEN.lock().unwrap().clear();ACTIVE.store(false,Ordering::Relaxed);
    hub_ready();assert!(!active(),"The result must only open once");
    let before=run.clone();let mut after=before.clone();
    after.elite_buffs=vec![1000,1150];trial_rewards(&before,&after);
    assert!(trial_rewards_active());
    assert!(matches!(SCREEN.lock().unwrap().front(),Some(Screen::TrialRewards{items,..}) if items==&vec![1000,1150]));
    party_notice("队伍已解散".into());party_notice("队伍已解散".into());
    {let screens=SCREEN.lock().unwrap();assert!(matches!(screens.front(),Some(Screen::TrialRewards{..}))&&screens.len()==2,"A notice must queue behind unacknowledged rewards");}
    SCREEN.lock().unwrap().clear();ACTIVE.store(false,Ordering::Relaxed);
    for size in [[1280.0,720.0],[1920.0,1080.0],[3440.0,1440.0]] {
        context.io_mut().display_size=size;
        for mut screen in [Screen::Notice{message:"爬塔协议版本不匹配：本机 23，对方 22。\n请所有玩家更新同一版爬塔补丁。".into(),opened:None},Screen::Summary{record:Record::new(&run,vec![erdtree_tower::journal::Fight{act:1,layer:9,kind:"精英".into(),won:false,enemies:vec!["黑骑士".into()]}],vec![],0),gain:Some(Gain{class:erdtree_tower::Class::Samurai,xp:320,before:4,after:5}),from_archive:false,focus:0,opened:None},Screen::TrialRewards{items:vec![1000,1150],opened:None},Screen::TrialRewards{items:vec![],opened:None},Screen::Seed{input:u64::MAX.to_string(),error:String::new(),digit:9,opened:None}] {
            render(context.frame(),&mut screen,&mut |_|None);
            let data=context.render();assert!(data.total_vtx_count>0);
            if matches!(screen,Screen::Summary{..}) {if let Ok(path)=std::env::var("ERDTREE_UI_PREVIEW") {
                use std::io::Write;
                let mut f=std::fs::File::create(std::path::Path::new(&path).join(format!("summary-{}-{}.draw",size[0],size[1]))).unwrap();
                writeln!(f,"{} {}",size[0],size[1]).unwrap();
                for list in data.draw_lists(){for command in list.commands(){if let hudhook::imgui::DrawCmd::Elements{count,cmd_params:p}=command {
                    writeln!(f,"C {} {} {} {} {}",p.texture_id.id(),p.clip_rect[0],p.clip_rect[1],p.clip_rect[2],p.clip_rect[3]).unwrap();
                    for triangle in list.idx_buffer()[p.idx_offset..p.idx_offset+count].chunks_exact(3) {
                        write!(f,"T").unwrap();for index in triangle {let v=list.vtx_buffer()[*index as usize+p.vtx_offset];write!(f," {} {} {} {} {} {} {} {}",v.pos[0],v.pos[1],v.uv[0],v.uv[1],v.col[0],v.col[1],v.col[2],v.col[3]).unwrap();}writeln!(f).unwrap();
                    }
                }}}
            }}
        }
    }
}
const SEED_KEYS:[&str;12]=["1","2","3","4","5","退格","6","7","8","9","0","清空"];
const SEED_DIGITS:usize=20;
fn seed_key(input:&mut String,error:&mut String,key:&str) {
    match key {"退格"=>{input.pop();},"清空"=>input.clear(),digit=>if input.len()<SEED_DIGITS {input.push_str(digit);}}
    error.clear();
}
fn seed_page(ui:&Ui,input:&mut String,error:&mut String,selected:&mut u8,ready:bool)->bool {
    let size=ui.io().display_size;
    let s=(size[0]/1600.0).min(size[1]/900.0);
    let mid=size[0]*0.5;let top=(size[1]-690.0*s)*0.5;
    let gold=[0.84,0.72,0.46,1.0];let muted=[0.64,0.61,0.52,1.0];let white=[0.92,0.88,0.77,1.0];let red=[0.9,0.46,0.38,1.0];
    crate::oath_art::background(&ui.get_background_draw_list(),size);
    let key=|k:Key|ready&&ui.is_key_pressed(k);
    let digits=[(Key::Alpha0,Key::Keypad0),(Key::Alpha1,Key::Keypad1),(Key::Alpha2,Key::Keypad2),(Key::Alpha3,Key::Keypad3),(Key::Alpha4,Key::Keypad4),
        (Key::Alpha5,Key::Keypad5),(Key::Alpha6,Key::Keypad6),(Key::Alpha7,Key::Keypad7),(Key::Alpha8,Key::Keypad8),(Key::Alpha9,Key::Keypad9)];
    for (n,(a,b)) in digits.into_iter().enumerate() {if key(a)||key(b){seed_key(input,error,&n.to_string());}}
    if key(Key::Backspace)||key(Key::GamepadFaceUp){seed_key(input,error,"退格");}
    if key(Key::Delete){seed_key(input,error,"清空");}
    if ui.io().key_ctrl&&key(Key::V) {
        for c in ui.clipboard_text().unwrap_or_default().chars().filter(char::is_ascii_digit) {seed_key(input,error,&c.to_string());}
    }
    if ui.is_key_pressed(Key::GamepadDpadLeft){*selected=(*selected+11)%12;}
    if ui.is_key_pressed(Key::GamepadDpadRight){*selected=(*selected+1)%12;}
    if ui.is_key_pressed(Key::GamepadDpadUp)||ui.is_key_pressed(Key::GamepadDpadDown){*selected=(*selected+6)%12;}
    *selected%=12;
    if key(Key::GamepadFaceDown){seed_key(input,error,SEED_KEYS[usize::from(*selected)]);}
    let mut start=key(Key::Enter)||key(Key::KeypadEnter)||key(Key::GamepadFaceLeft);
    let mut close=key(Key::Escape)||key(Key::GamepadFaceRight);
    let mut pressed=None;
    ui.window("以种子开始##seed").position([0.0,0.0],Condition::Always).size(size,Condition::Always)
        .flags(hudhook::imgui::WindowFlags::NO_DECORATION|hudhook::imgui::WindowFlags::NO_BACKGROUND|hudhook::imgui::WindowFlags::NO_MOVE|hudhook::imgui::WindowFlags::NO_NAV).build(|| {
        let text=|x:f32,y:f32,value:&str,color:[f32;4],scale:f32| {
            ui.set_window_font_scale(scale*s);ui.set_cursor_pos([x-ui.calc_text_size(value)[0]*0.5,top+y*s]);ui.text_colored(color,value);
        };
        text(mid,12.0,"开始新旅程",muted,1.0);
        text(mid,57.0,"以 种 子 开 启 旅 程",gold,2.1);
        text(mid,130.0,"相同种子生成相同的路线与敌人，奖励仍独立随机",muted,1.0);
        let draw=ui.get_window_draw_list();
        for y in [194.0,540.0] {draw.add_line([mid-410.0*s,top+y*s],[mid+410.0*s,top+y*s],hudhook::imgui::ImColor32::from_rgba(171,143,87,100)).build();}
        draw.add_rect([mid-394.0*s,top+222.0*s],[mid+394.0*s,top+300.0*s],hudhook::imgui::ImColor32::from_rgba(10,9,7,210)).filled(true).rounding(6.0*s).build();
        draw.add_rect([mid-394.0*s,top+222.0*s],[mid+394.0*s,top+300.0*s],hudhook::imgui::ImColor32::from_rgba(171,143,87,170)).rounding(6.0*s).thickness(1.5*s).build();
        drop(draw);
        if input.is_empty() {text(mid,247.0,"输入 0 – 18446744073709551615 之间的整数",muted,1.0);}
        else {text(mid,240.0,&format!("{input}{}",if (ui.time()*2.0) as i64%2==0 {"|"}else{" "}),white,1.6);}
        if error.is_empty() {text(mid,318.0,&format!("{} / {SEED_DIGITS} 位  ·  可直接用键盘输入，Ctrl+V 粘贴",input.len()),muted,0.9);}
        else {text(mid,318.0,error,red,1.0);}
        let (w,h,gap)=(118.0,60.0,16.0);
        for (i,label) in SEED_KEYS.iter().enumerate() {
            let x=mid+(-394.0+(i%6) as f32*(w+gap))*s;let y=370.0+(i/6) as f32*(h+gap);
            let pos=[x,top+y*s];let extent=[w*s,h*s];
            ui.set_cursor_screen_pos(pos);if ui.invisible_button(format!("##seed-key-{i}"),extent)&&ready{pressed=Some(i);}
            let focus=usize::from(*selected)==i;let draw=ui.get_window_draw_list();
            crate::oath_art::button(&draw,pos,[pos[0]+extent[0],pos[1]+extent[1]],if ui.is_item_hovered()||focus{255}else{190});
            if focus {draw.add_rect([pos[0]-3.0*s,pos[1]-3.0*s],[pos[0]+extent[0]+3.0*s,pos[1]+extent[1]+3.0*s],hudhook::imgui::ImColor32::from_rgba(214,184,117,230)).rounding(5.0*s).thickness(2.0*s).build();}
            drop(draw);
            let label_scale=if label.len()>1 {0.95}else{1.25};
            ui.set_window_font_scale(label_scale*s);
            ui.set_cursor_pos([x+extent[0]*0.5-ui.calc_text_size(label)[0]*0.5,pos[1]+(extent[1]-ui.text_line_height())*0.5]);
            ui.text_colored(if focus{white}else{gold},label);
        }
        text(mid,556.0,"手柄：方向键选择  ·  A 按下  ·  Y 退格  ·  X 开始  ·  B 取消",muted,0.85);
        ui.set_window_font_scale(s);
        let button=|label:&str,x:f32| {
            let pos=[x,top+602.0*s];let extent=[320.0*s,65.0*s];
            ui.set_cursor_screen_pos(pos);let clicked=ui.invisible_button(label,extent);
            crate::oath_art::button(&ui.get_window_draw_list(),pos,[pos[0]+extent[0],pos[1]+extent[1]],if ui.is_item_hovered(){255}else{215});
            text(x+extent[0]*0.5,622.0,label,gold,1.0);clicked
        };
        start|=button("开始旅程  [Enter / X]",mid-340.0*s)&&ready;
        close|=button("取消  [Esc / B]",mid+20.0*s)&&ready;
    });
    if let Some(i)=pressed {seed_key(input,error,SEED_KEYS[i]);}
    if start&&!close {
        match parse_seed(input) {Ok(seed)=>{if let Ok(mut pending)=SEED.lock(){*pending=Some(seed);return true;}},Err(message)=>*error=message.into()}
    }
    close
}
const ROMAN:[&str;4]=["","I","II","III"];
const ACT_NAMES:[&str;3]=["第一幕","第二幕","第三幕"];
/// 从 (x,y) 起写一段在 right 处折行的文字，返回下一行的 y（都是屏幕坐标）
/// 按中文规则换行（见 fancy_text::wrap_lines），逐行画出
pub(crate) fn wrapped(ui:&Ui,pos:[f32;2],right:f32,value:&str,color:[f32;4],scale:f32)->f32 {
    ui.set_window_font_scale(scale);
    let lines=crate::fancy_text::wrap_lines(value,(right-pos[0]).max(1.0),|t|ui.calc_text_size(t)[0]);
    let step=ui.text_line_height_with_spacing();let mut y=pos[1];let mut bottom=pos[1];
    for line in lines {ui.set_cursor_screen_pos([pos[0],y]);ui.text_colored(color,&line);bottom=ui.item_rect_max()[1];y+=step;}
    bottom
}
/// 结算页：标题与终局说明，下方三块面板（旅程 · 誓约 · 余烬），底部种子与按钮。
/// from_archive 时从旅程档案打开，第二个按钮改为返回档案
fn summary_page(ui:&Ui,record:&Record,gain:Option<Gain>,focus:&mut usize,from_archive:bool,icon:&mut dyn FnMut(u32)->Option<TextureId>,ready:bool)->bool {
    use hudhook::imgui::{ImColor32,WindowFlags};
    let size=ui.io().display_size;let s=(size[0]/1600.0).min(size[1]/900.0);
    let (ox,oy)=((size[0]-1600.0*s)*0.5,(size[1]-900.0*s)*0.5);let p=|x:f32,y:f32|[ox+x*s,oy+y*s];
    let gold=[0.86,0.74,0.48,1.0];let muted=[0.62,0.59,0.51,1.0];let white=[0.93,0.9,0.81,1.0];let red=[0.92,0.47,0.38,1.0];let dim=[0.45,0.43,0.38,1.0];let boon=[0.86,0.8,0.64,1.0];
    let background=ui.get_background_draw_list();crate::oath_art::background(&background,size);
    let shade=|a:u8|ImColor32::from_rgba(0,0,0,a);
    background.add_rect_filled_multicolor([0.0,0.0],size,shade(60),shade(60),shade(160),shade(160));
    drop(background);
    let review=erdtree_tower::journal::review(record);
    let (won,dead)=(record.result==Outcome::Won,record.result==Outcome::Dead);
    // 余烬焦点：方向键左右切换，鼠标移动时跟随悬停
    let count=record.embers.len();
    if count>0&&ready {
        let pressed=|keys:&[Key]|keys.iter().any(|k|ui.is_key_pressed(*k));
        if pressed(&[Key::LeftArrow,Key::GamepadDpadLeft]) {*focus=(*focus+count-1)%count;}
        if pressed(&[Key::RightArrow,Key::GamepadDpadRight]) {*focus=(*focus+1)%count;}
    }
    *focus=(*focus).min(count.saturating_sub(1));
    let mut close=false;
    ui.window("旅程结算##summary").position([0.0,0.0],Condition::Always).size(size,Condition::Always)
        .flags(WindowFlags::NO_DECORATION|WindowFlags::NO_BACKGROUND|WindowFlags::NO_MOVE|WindowFlags::NO_NAV|WindowFlags::NO_SCROLL_WITH_MOUSE).build(|| {
        let text=|x:f32,y:f32,value:&str,color:[f32;4],scale:f32,align:f32| {
            ui.set_window_font_scale(scale*s);ui.set_cursor_screen_pos([ox+x*s-ui.calc_text_size(value)[0]*align,oy+y*s]);ui.text_colored(color,value);
        };
        // 标题与终局说明
        text(800.0,30.0,crate::map_ui::class_name(record.class),muted,0.95,0.5);
        text(800.0,58.0,if won{"登 顶 成 功"}else if dead{"旅 程 终 结"}else{"暂 离 结 算"},gold,2.2,0.5);
        let mut subtitle=format!("第 {} 幕  ·  第 {} 层",record.act,record.layer);
        if record.debug {subtitle.push_str("  ·  调试旅程，不计入纪录");}
        text(800.0,124.0,&subtitle,white,1.1,0.5);
        let draw=ui.get_window_draw_list();crate::law_ui::ornament(&draw,p(800.0,166.0),280.0*s,s);drop(draw);
        let verdict=match &review.fatal {
            Some(fatal)=>format!("倒在 {} 面前  ·  第 {} 幕第 {} 层{}",fatal.enemies.join("、"),fatal.act,fatal.layer,match fatal.kind.as_str() {"精英"=>" · 精英战","Boss"=>" · Boss 战",_=>""}),
            None if dead=>format!("倒在第 {} 幕第 {} 层",record.act,record.layer.max(1)),
            None if won=>"三幕全部通关".into(),
            None=>"旅程尚未结束，回到大赐福即可继续".into(),
        };
        text(800.0,182.0,&verdict,if dead{red}else{muted},0.95,0.5);
        const TOP:f32=228.0;const BOTTOM:f32=690.0;const WIDTH:f32=420.0;
        let draw=ui.get_window_draw_list();
        for x in [110.0,590.0,1070.0] {crate::law_ui::panel(&draw,p(x,TOP),p(x+WIDTH,BOTTOM),s);crate::law_ui::ornament(&draw,p(x+WIDTH*0.5,TOP+58.0),150.0*s,s);}
        drop(draw);
        for (x,title) in [(110.0,"旅  程"),(590.0,"誓  约"),(1070.0,"余  烬")] {text(x+WIDTH*0.5,TOP+18.0,title,gold,1.15,0.5);}
        let line=|x0:f32,x1:f32,y:f32| {ui.get_window_draw_list().add_line(p(x0,y),p(x1,y),ImColor32::from_rgba(171,143,87,70)).thickness(s).build();};
        // 旅程：总数、三幕战绩、圣杯与律法
        let x=110.0;
        for (i,(label,value)) in [("节点",record.nodes as usize),("战斗",record.battles as usize),("物品",record.items)].into_iter().enumerate() {
            let cx=x+75.0+i as f32*135.0;text(cx,TOP+76.0,&value.to_string(),white,1.8,0.5);text(cx,TOP+124.0,label,muted,0.92,0.5);
        }
        line(x+30.0,x+WIDTH-30.0,TOP+162.0);
        for (i,act) in review.acts.iter().enumerate() {
            let number=i as u8+1;let y=TOP+178.0+i as f32*64.0;
            let passed=won||number<record.act;let current=!won&&number==record.act;
            let draw=ui.get_window_draw_list();let center=p(x+52.0,y+24.0);
            let (fill,border)=if passed {(ImColor32::from_rgba(150,118,62,255),ImColor32::from_rgba(228,196,130,255))}
                else if current {(ImColor32::from_rgba(30,24,16,255),if dead{ImColor32::from_rgba(214,96,74,255)}else{ImColor32::from_rgba(214,180,112,255)})}
                else {(ImColor32::from_rgba(20,18,14,255),ImColor32::from_rgba(90,80,62,200))};
            draw.add_circle(center,19.0*s,fill).filled(true).build();draw.add_circle(center,19.0*s,border).thickness(1.6*s).build();drop(draw);
            ui.set_window_font_scale(0.95*s);let numeral=ROMAN[usize::from(number)];let w=ui.calc_text_size(numeral);
            ui.set_cursor_screen_pos([center[0]-w[0]*0.5,center[1]-w[1]*0.5]);ui.text_colored(if passed{[0.12,0.09,0.05,1.0]}else if current{white}else{dim},numeral);
            let (status,color)=if !act.bosses.is_empty() {(format!("击败 {}",act.bosses.join("、")),white)}
                else if current&&dead {(format!("倒在第 {} 层",review.fatal.as_ref().map_or(record.layer.max(1),|f|f.layer)),red)}
                else if passed {("已通过".into(),white)} else if current {("进行中".into(),gold)} else {("未抵达".into(),dim)};
            text(x+88.0,y+1.0,ACT_NAMES[i],if passed||current{gold}else{dim},0.86,0.0);
            let detail=if act.battles>0 {format!("{status}  ·  战斗 {} 场{}",act.battles,if act.elites>0{format!("，精英 {}",act.elites)}else{String::new()})}else{status};
            wrapped(ui,p(x+88.0,y+25.0),ox+(x+WIDTH-24.0)*s,&detail,color,0.95*s);
        }
        line(x+30.0,x+WIDTH-30.0,BOTTOM-92.0);
        text(x+30.0,BOTTOM-76.0,&format!("圣杯  红 +{} / 蓝 +{}",record.flask,record.flask),gold,0.88,0.0);
        text(x+WIDTH-30.0,BOTTOM-76.0,&format!("记忆栏位 +{}",record.memory),gold,0.88,1.0);
        if record.law.is_empty() {text(x+30.0,BOTTOM-44.0,"未立苦难律法",dim,0.88,0.0);}
        else {
            text(x+30.0,BOTTOM-44.0,&format!("苦难律法 {} 分 · {}",record.law.score(),record.law.grade().unwrap_or("普通")),if won{white}else{muted},0.88,0.0);
            text(x+WIDTH-30.0,BOTTOM-44.0,&record.law.code(),muted,0.88,1.0);
        }
        // 誓约：等级徽记、增益与代价
        let x=590.0;
        if record.oaths.is_empty() {text(x+WIDTH*0.5,(TOP+BOTTOM)*0.5,"本局没有立下誓约",dim,0.95,0.5);}
        let row=(372.0/record.oaths.len().max(1) as f32).min(124.0);
        for (i,oath) in record.oaths.iter().enumerate() {
            let y=TOP+86.0+i as f32*row;
            let (boon_text,bane_text)=if oath.level==0 {(oath.kind.boon().to_owned(),oath.kind.bane())}else{(oath.kind.boon_at(oath.level),oath.kind.bane_at())};
            oath_badge(ui,p(x+48.0,y+20.0),19.0*s,ROMAN[usize::from(oath.level.min(3))],true,s);
            text(x+80.0,y+4.0,oath.kind.name(),white,1.05,0.0);
            let next=wrapped(ui,p(x+80.0,y+36.0),ox+(x+WIDTH-24.0)*s,&boon_text.replace('\n',"，"),boon,0.92*s);
            wrapped(ui,[ox+(x+80.0)*s,next+3.0*s],ox+(x+WIDTH-24.0)*s,&format!("代价：{bane_text}"),red,0.92*s);
        }
        // 余烬：徽记网格，下方显示选中余烬的效果（方向键或鼠标切换）
        let x=1070.0;
        if count==0 {text(x+WIDTH*0.5,(TOP+BOTTOM)*0.5,"本局没有获得余烬",dim,0.95,0.5);}
        let (columns,side)=match count {0..=6=>(3,80.0),7..=8=>(4,64.0),_=>(4,56.0)};
        let cell=(WIDTH-40.0)/columns as f32;let row=side+36.0;
        for (i,ember) in record.embers.iter().enumerate() {
            let cx=x+20.0+cell*((i%columns) as f32+0.5);let y=TOP+84.0+(i/columns) as f32*row;
            let (min,max)=(p(cx-side*0.5,y),p(cx+side*0.5,y+side));
            let selected=i==*focus;
            let draw=ui.get_window_draw_list();
            if selected {draw.add_circle(p(cx,y+side*0.5),(side*0.5+5.0)*s,ImColor32::from_rgba(236,196,120,230)).thickness(2.0*s).build();}
            match crate::reward_ui::ember_icon(*ember).and_then(&mut *icon) {
                Some(texture)=>{let light=if selected{255}else{205};draw.add_image(texture,min,max).col(ImColor32::from_rgba(light,light,light,255)).build();},
                None=>{draw.add_circle(p(cx,y+side*0.5),side*0.45*s,ImColor32::from_rgba(120,60,30,255)).filled(true).build();},
            }
            drop(draw);
            let name=ember.name();ui.set_window_font_scale(0.88*s);
            let fit=(cell*s-6.0*s)/ui.calc_text_size(name)[0];
            text(cx,y+side+6.0,name,if selected{gold}else{white},0.88*fit.min(1.0),0.5);
            let moved=ui.io().mouse_delta!=[0.0,0.0];
            if moved&&ui.is_mouse_hovering_rect(min,max) {*focus=i;}
        }
        if let Some(ember)=record.embers.get(*focus) {
            line(x+30.0,x+WIDTH-30.0,BOTTOM-104.0);
            text(x+30.0,BOTTOM-94.0,ember.name(),gold,0.9,0.0);
            if count>1 {text(x+WIDTH-30.0,BOTTOM-94.0,"左右键切换",dim,0.78,1.0);}
            wrapped(ui,p(x+30.0,BOTTOM-66.0),ox+(x+WIDTH-26.0)*s,&ember.effect().replace('\n',"，"),boon,0.86*s);
        }
        // 种子与按钮
        let label=if record.whole_seed{"本局种子"}else{"本幕种子"};
        text(792.0,710.0,label,muted,0.95,1.0);text(808.0,706.0,&record.seed.to_string(),white,1.15,0.0);
        // 职业熟练度（局外养成）
        if let Some(gain)=gain {
            let class=crate::map_ui::class_name(gain.class);
            let unlocked:Vec<_>=gain.unlocked().into_iter().map(|id|crate::reward_ui::talisman_description(id).0).collect();
            let line=if !unlocked.is_empty() {format!("{class}熟练度 +{}  ·  升到 Lv {}，解锁开局护符「{}」（按 {} 在旅程档案的传承页更换）",gain.xp,gain.after,unlocked.join("」「"),crate::options::label(erdtree_tower::options::Hotkey::Archive))}
                else if gain.after>gain.before {format!("{class}熟练度 +{}  ·  升到 Lv {}",gain.xp,gain.after)}
                else {format!("{class}熟练度 +{}  ·  Lv {}",gain.xp,gain.after)};
            text(800.0,742.0,&line,if unlocked.is_empty(){boon}else{gold},0.92,0.5);
        }
        let button=|label:&str,x:f32| {
            let pos=p(x,784.0);let extent=[320.0*s,65.0*s];
            ui.set_cursor_screen_pos(pos);ui.set_window_font_scale(s);let clicked=ui.invisible_button(label,extent);
            crate::oath_art::button(&ui.get_window_draw_list(),pos,[pos[0]+extent[0],pos[1]+extent[1]],if ui.is_item_hovered(){255}else{215});
            text(x+160.0,804.0,label,gold,1.0,0.5);clicked
        };
        if button("复制种子  [Y]",460.0)||ui.is_key_pressed_no_repeat(Key::GamepadFaceUp){ui.set_clipboard_text(record.seed.to_string());}
        close=if from_archive {
            button("返回档案  [Esc / B]",820.0)||[Key::Escape,Key::GamepadFaceRight,Key::Enter,Key::GamepadFaceDown].into_iter().any(|k|ui.is_key_pressed_no_repeat(k))
        } else {
            button("返回大赐福  [A]",820.0)||ui.is_key_pressed_no_repeat(Key::Enter)||ui.is_key_pressed_no_repeat(Key::GamepadFaceDown)
        };
    });
    close&&ready
}
/// 誓约等级徽记：菱形底，罗马数字写等级；lit 为 false 时是未解锁的灰色
pub(crate) fn oath_badge(ui:&Ui,c:[f32;2],h:f32,numeral:&str,lit:bool,s:f32) {
    use hudhook::imgui::ImColor32;
    let draw=ui.get_window_draw_list();
    let diamond=vec![[c[0],c[1]-h],[c[0]+h,c[1]],[c[0],c[1]+h],[c[0]-h,c[1]]];
    draw.add_polyline(diamond.clone(),if lit{ImColor32::from_rgba(74,50,28,255)}else{ImColor32::from_rgba(28,25,20,255)}).filled(true).build();
    let mut outline=diamond.clone();outline.push(diamond[0]);
    draw.add_polyline(outline,if lit{ImColor32::from_rgba(214,180,112,255)}else{ImColor32::from_rgba(90,80,62,200)}).thickness(1.6*s).build();drop(draw);
    if !numeral.is_empty() {
        ui.set_window_font_scale(0.85*h/19.0);let w=ui.calc_text_size(numeral);
        ui.set_cursor_screen_pos([c[0]-w[0]*0.5,c[1]-w[1]*0.5]);ui.text_colored(if lit{[0.93,0.9,0.81,1.0]}else{[0.45,0.43,0.38,1.0]},numeral);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seed_input_accepts_full_u64_without_float_precision_loss() {
        assert_eq!(parse_seed(" 18446744073709551615 "),Ok(u64::MAX));
        assert_eq!(parse_seed("0"),Ok(0));
        for text in ["","-1","1.5","1e3","18446744073709551616","abc"]{assert!(parse_seed(text).is_err());}
    }
}
