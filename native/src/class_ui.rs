// 选择职业：大赐福“开始新旅程”后先选职业，再进入苦难律法。六张职业卡并排，方向键或鼠标选择，Enter / A 确定，Esc / B 返回。
use hudhook::imgui::{Condition,DrawListMut,ImColor32,Key,Ui,WindowFlags};
use crate::law_ui::{ornament,panel};
use crate::run_screen::wrapped;

pub enum ClassAction {Pick(usize),Back}
/// 一个职业的卡面：名字、定位、借用出身、机制名与说明、起手、擅长
pub struct Role {pub name:&'static str,pub role:&'static str,pub base:&'static str,pub mechanic:(&'static str,&'static str),pub kit:&'static str,pub bias:&'static str,pub accent:(u8,u8,u8)}
/// 顺序与 erdtree_tower::Class::PLAYABLE 一致
pub const ROLES:[Role;6]=[
    Role{name:"守誓骑士",role:"坦克 · 格挡反击",base:"流浪骑士",mechanic:("誓守","防御反击伤害 +30%，防御时精力消耗 -30%"),kit:"长剑、斧枪、鸢形盾",bias:"直剑、大剑、斧枪、中盾与大盾",accent:(196,160,96)},
    Role{name:"狂战士",role:"重武器 · 残血爆发",base:"勇者",mechanic:("狂血","生命低于一半时造成的伤害 +20%；打赢一场战斗回复 8% 生命"),kit:"战斧、巨斧、大皮盾",bias:"斧、大斧、槌、大槌、特大武器",accent:(200,84,64)},
    Role{name:"影刃",role:"敏捷 · 致命一击",base:"武士",mechanic:("影步","致命一击伤害 +30%，精力恢复速度提高"),kit:"打刀、长弓、红棘木盾",bias:"刀、匕首、曲剑、弓",accent:(150,172,190)},
    Role{name:"星辉术士",role:"远程法术",base:"观星者",mechanic:("星涌","进入战斗时专注回复一半；魔法专注消耗 -15%"),kit:"辉石杖、短剑、辉石魔砾、辉石彗星",bias:"法杖与魔法",accent:(110,140,220)},
    Role{name:"祷誓者",role:"祷告 · 治疗 · 辅助",base:"预言家",mechanic:("圣誓","祷告专注消耗 -15%；打赢一场战斗回复 25% 专注"),kit:"短矛、指头圣印记、治疗、招来火焰",bias:"圣印记、祷告与矛",accent:(226,196,120)},
    Role{name:"召唤师",role:"骨灰为核心",base:"密使",mechanic:("群灵","战斗胜利时召唤物还活着，这次召唤不消耗骨灰；每页奖励都有骨灰，召唤流余烬必定可出"),kit:"阔剑、鸢形盾、圣印记、离群野狼与骸骨民兵的骨灰",bias:"骨灰、轻武器与祷告",accent:(140,196,170)},
];
pub struct ClassScreen {pub focus:usize,opened:Option<f64>}
impl ClassScreen {pub fn new(focus:usize)->Self {Self{focus:focus.min(ROLES.len()-1),opened:None}}}
pub fn draw(ui:&Ui,screen:&mut ClassScreen)->Option<ClassAction> {
    let size=ui.io().display_size;let s=(size[0]/1600.0).min(size[1]/900.0);
    let (ox,oy)=((size[0]-1600.0*s)*0.5,(size[1]-900.0*s)*0.5);let p=|x:f32,y:f32|[ox+x*s,oy+y*s];
    let background=ui.get_background_draw_list();crate::oath_art::background(&background,size);
    let shade=|a:u8|ImColor32::from_rgba(0,0,0,a);
    background.add_rect_filled_multicolor([0.0,0.0],size,shade(80),shade(80),shade(170),shade(170));drop(background);
    let opened=*screen.opened.get_or_insert(ui.time());let ready=ui.time()-opened>=0.3;
    let pressed=|keys:&[Key]|ready&&keys.iter().any(|k|ui.is_key_pressed(*k));
    if pressed(&[Key::LeftArrow,Key::A,Key::GamepadDpadLeft]) {screen.focus=(screen.focus+ROLES.len()-1)%ROLES.len();}
    if pressed(&[Key::RightArrow,Key::D,Key::GamepadDpadRight]) {screen.focus=(screen.focus+1)%ROLES.len();}
    let mut action=if pressed(&[Key::Enter,Key::GamepadFaceDown]) {Some(ClassAction::Pick(screen.focus))}else if pressed(&[Key::Escape,Key::GamepadFaceRight]) {Some(ClassAction::Back)}else{None};
    let moved=ui.io().mouse_delta!=[0.0,0.0];let time=ui.time() as f32;
    ui.window("选择职业##classes").position([0.0,0.0],Condition::Always).size(size,Condition::Always)
        .flags(WindowFlags::NO_DECORATION|WindowFlags::NO_BACKGROUND|WindowFlags::NO_MOVE|WindowFlags::NO_NAV|WindowFlags::NO_SCROLL_WITH_MOUSE).build(|| {
        let text=|x:f32,y:f32,value:&str,color:[f32;4],scale:f32,align:f32| {
            ui.set_window_font_scale(scale*s);ui.set_cursor_screen_pos([ox+x*s-ui.calc_text_size(value)[0]*align,oy+y*s]);ui.text_colored(color,value);
        };
        let (gold,white,muted,boon)=([0.86,0.74,0.48,1.0],[0.93,0.9,0.81,1.0],[0.62,0.59,0.51,1.0],[0.86,0.8,0.64,1.0]);
        text(800.0,30.0,"选 择 职 业",gold,1.9,0.5);
        text(800.0,92.0,"每个职业有一项专属机制、起手装备与擅长的奖励；职业传承按职业分别计算",muted,0.86,0.5);
        let draw=ui.get_window_draw_list();ornament(&draw,p(800.0,128.0),420.0*s,s);drop(draw);
        for (i,role) in ROLES.iter().enumerate() {
            let focused=screen.focus==i;let lift=if focused{14.0}else{0.0};
            let (x,y,w,h)=(52.0+i as f32*250.0,160.0-lift,236.0,650.0);
            let (min,max)=(p(x,y),p(x+w,y+h));
            ui.set_cursor_screen_pos(min);ui.set_window_font_scale(s);
            let clicked=ui.invisible_button(format!("role-{i}"),[w*s,h*s]);
            if ui.is_item_hovered()&&moved {screen.focus=i;}
            if clicked&&ready {action=Some(ClassAction::Pick(i));}
            let (r,g,b)=role.accent;let accent=[f32::from(r)/255.0,f32::from(g)/255.0,f32::from(b)/255.0,1.0];
            let draw=ui.get_window_draw_list();
            panel(&draw,min,max,s);
            // 选中的卡：外框随“呼吸”明暗变化
            if focused {let breath=(110.0+80.0*(time*2.2).sin()) as u8;draw.add_rect([min[0]-3.0*s,min[1]-3.0*s],[max[0]+3.0*s,max[1]+3.0*s],ImColor32::from_rgba(r,g,b,breath)).rounding(3.0*s).thickness(2.0*s).build();}
            drop(draw);
            // 职业插画（贴图未加载时退回职业色的菱形徽记）
            let draw=ui.get_window_draw_list();
            let (art_min,art_max)=(p(x+8.0,y+8.0),p(x+w-8.0,y+218.0));
            // 选中时插画缓慢推近
            let zoom=if focused {1.0+0.035*(0.5-0.5*(time*0.6).cos())}else{1.0};
            if crate::oath_art::ready() {crate::oath_art::class_art(&draw,i,art_min,art_max,if focused{255}else{185},zoom);}
            else {
                let c=p(x+w*0.5,y+110.0);let hr=40.0*s;
                let diamond=vec![[c[0],c[1]-hr],[c[0]+hr,c[1]],[c[0],c[1]+hr],[c[0]-hr,c[1]]];
                draw.add_polyline(diamond.clone(),ImColor32::from_rgba(r/4,g/4,b/4,255)).filled(true).build();
                let mut outline=diamond.clone();outline.push(diamond[0]);
                draw.add_polyline(outline,ImColor32::from_rgba(r,g,b,255)).thickness(2.0*s).build();
            }
            effects(&draw,i,art_min,art_max,focused,time,s);
            draw.add_rect(art_min,art_max,ImColor32::from_rgba(r,g,b,if focused{150}else{60})).thickness(s).build();
            drop(draw);
            text(x+w*0.5,y+228.0,role.name,if focused{gold}else{white},1.25,0.5);
            text(x+w*0.5,y+264.0,role.role,accent,0.82,0.5);
            let draw=ui.get_window_draw_list();ornament(&draw,p(x+w*0.5,y+298.0),90.0*s,s);drop(draw);
            let right=ox+(x+w-18.0)*s;
            text(x+18.0,y+314.0,&format!("机制 · {}",role.mechanic.0),gold,0.92,0.0);
            let next=wrapped(ui,p(x+18.0,y+344.0),right,role.mechanic.1,boon,0.84*s);
            let mut yy=(next-oy)/s+16.0;
            text(x+18.0,yy,"起手",gold,0.86,0.0);yy+=26.0;
            yy=(wrapped(ui,p(x+18.0,yy),right,role.kit,white,0.82*s)-oy)/s+14.0;
            text(x+18.0,yy,"擅长",gold,0.86,0.0);yy+=26.0;
            wrapped(ui,p(x+18.0,yy),right,role.bias,white,0.82*s);
            text(x+w*0.5,y+h-34.0,&format!("属性借用 {}",role.base),muted,0.76,0.5);
        }
        text(800.0,840.0,"左右键或鼠标选择 · Enter / A 确定 · Esc / B 返回",muted,0.84,0.5);
    });
    action
}

/// 0..1 的确定性随机数：同一颗粒子每帧取到相同的值
fn rand(i:u32,k:u32)->f32 {
    let mut x=i.wrapping_mul(0x9E37_79B1)^k.wrapping_mul(0x85EB_CA77);
    x^=x>>15;x=x.wrapping_mul(0x2C1B_3C6D);x^=x>>12;
    (x&0xffff) as f32/65535.0
}
/// 带一圈柔光的粒子
fn mote(draw:&DrawListMut<'_>,c:[f32;2],radius:f32,(r,g,b):(u8,u8,u8),alpha:f32) {
    let a=|k:f32|(alpha.clamp(0.0,1.0)*k*255.0) as u8;
    draw.add_circle(c,radius*2.6,ImColor32::from_rgba(r,g,b,a(0.18))).filled(true).build();
    draw.add_circle(c,radius,ImColor32::from_rgba(r,g,b,a(0.95))).filled(true).build();
}
/// 卡面插画上的动态效果：每个职业一种粒子；选中时粒子更多更亮，并有一道光带扫过
fn effects(draw:&DrawListMut<'_>,index:usize,min:[f32;2],max:[f32;2],focused:bool,time:f32,s:f32) {
    let (w,h)=(max[0]-min[0],max[1]-min[1]);
    let count=if focused {22}else{7};let dim=if focused {1.0}else{0.45};
    draw.with_clip_rect_intersect(min,max,|| {
        for i in 0..count {
            let (a,b,c,d)=(rand(i,index as u32*7+1),rand(i,index as u32*7+2),rand(i,index as u32*7+3),rand(i,index as u32*7+4));
            match index {
                // 狂战士：翻飞的红色余烬，忽明忽暗
                1=>{
                    let life=(time*(0.35+0.4*a)+b).fract();
                    let x=min[0]+w*c+(time*4.0+i as f32).sin()*6.0*s;let y=max[1]-life*h*1.1;
                    let flicker=0.6+0.4*(time*13.0+i as f32*1.7).sin();
                    mote(draw,[x,y],(1.4+1.6*d)*s,(255,96,52),(std::f32::consts::PI*life).sin()*flicker*dim);
                },
                // 影刃：横向飘过的雾
                2=>{
                    let life=(time*(0.05+0.05*a)+b).fract();
                    let x=min[0]-40.0*s+(w+80.0*s)*life;let y=min[1]+h*(0.45+0.5*c);
                    draw.add_circle([x,y],(18.0+16.0*d)*s,ImColor32::from_rgba(150,160,176,((std::f32::consts::PI*life).sin()*0.12*dim*255.0) as u8)).filled(true).build();
                },
                // 星辉术士：闪烁的星光，较亮的带十字光芒
                3=>{
                    let (x,y)=(min[0]+w*a,min[1]+h*0.75*b);
                    let twinkle=(0.5+0.5*(time*(2.0+3.0*c)+d*6.283).sin())*dim;
                    mote(draw,[x,y],(0.9+1.3*d)*s,(150,186,255),twinkle);
                    if d>0.7 {
                        let l=(5.0+5.0*twinkle)*s;let color=ImColor32::from_rgba(190,214,255,(twinkle*200.0) as u8);
                        draw.add_line([x-l,y],[x+l,y],color).thickness(s).build();draw.add_line([x,y-l],[x,y+l],color).thickness(s).build();
                    }
                },
                // 守誓骑士、祷誓者、召唤师：缓缓上升的光点
                _=>{
                    let (speed,color,size,sway,spread)=match index {
                        0=>(0.18,(255,208,120),1.0,3.0,1.0),
                        4=>(0.14,(255,232,168),1.4,4.0,0.45),
                        _=>(0.12,(120,240,210),2.0,10.0,1.0),
                    };
                    let life=(time*speed*(0.6+0.8*a)+b).fract();
                    let x=min[0]+w*((1.0-spread)*0.5+spread*c)+(time*1.5+i as f32).sin()*sway*s;let y=max[1]-life*h*1.05;
                    mote(draw,[x,y],(size+1.2*d)*s,color,(std::f32::consts::PI*life).sin()*dim);
                },
            }
        }
        // 选中时一道光带每 4.5 秒从左到右扫过插画
        if focused {
            let progress=(time%4.5)/1.6;
            if progress<1.0 {
                let center=min[0]-60.0*s+(w+120.0*s)*progress;let half=26.0*s;
                let (clear,bright)=(ImColor32::from_rgba(255,244,220,0),ImColor32::from_rgba(255,244,220,46));
                draw.add_rect_filled_multicolor([center-half,min[1]],[center,max[1]],clear,bright,bright,clear);
                draw.add_rect_filled_multicolor([center,min[1]],[center+half,max[1]],bright,clear,clear,bright);
            }
        }
    });
}
