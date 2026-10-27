use hudhook::imgui::{Condition,DrawListMut,ImColor32,Key,TextureId,Ui,WindowFlags};
use erdtree_tower::law::{BRANCHES,GRADES,Law,NODES};

pub enum LawAction {Start(Law),Back,Archive}
pub struct LawScreen {pub law:Law,pub seed:Option<u64>,focus:usize,opened:Option<f64>,copied:Option<f64>}
impl LawScreen {pub fn new(law:Law)->Self {Self{law,seed:None,focus:0,opened:None,copied:None}}}

const GOLD:[f32;4]=[0.86,0.74,0.48,1.0];const MUTED:[f32;4]=[0.62,0.59,0.51,1.0];const WHITE:[f32;4]=[0.93,0.9,0.81,1.0];const RED:[f32;4]=[0.92,0.47,0.38,1.0];
const NOTES:[&str;4]=["强化敌方","束缚自身","削减收益","改写规则"];
const ACCENT:[(u8,u8,u8);4]=[(190,70,52),(196,150,72),(150,160,96),(128,104,170)];
const GRADE_COLORS:[(u8,u8,u8);4]=[(150,160,170),(200,160,90),(214,110,70),(225,60,60)];
const CARD:[f32;2]=[142.0,54.0];const TRUNK:f32=176.0;
fn rgba(c:(u8,u8,u8),a:u8)->ImColor32 {ImColor32::from_rgba(c.0,c.1,c.2,a)}
fn node_pos(id:usize)->[f32;2] {let n=&NODES[id];[210.0+f32::from(n.col-1)*168.0,136.0+f32::from(n.row)*66.0+n.branch as f32*16.0]}
fn moved(focus:usize,dx:i32,dy:i32)->usize {
    let (x,y)=(i32::from(NODES[focus].col),i32::from(NODES[focus].row));
    (0..NODES.len()).filter(|&i|{let (ix,iy)=(i32::from(NODES[i].col),i32::from(NODES[i].row));(dx==0||(ix-x)*dx>0)&&(dy==0||(iy-y)*dy>0)})
        .min_by_key(|&i|{let (ix,iy)=(i32::from(NODES[i].col),i32::from(NODES[i].row));if dx!=0 {(iy-y).abs()*10+(ix-x).abs()}else{(ix-x).abs()*10+(iy-y).abs()}}).unwrap_or(focus)
}
/// 带金色边框与四角装饰的暗色面板
pub(crate) fn panel(draw:&DrawListMut<'_>,min:[f32;2],max:[f32;2],s:f32) {
    draw.add_rect_filled_multicolor(min,max,ImColor32::from_rgba(16,13,9,228),ImColor32::from_rgba(16,13,9,228),ImColor32::from_rgba(8,7,5,236),ImColor32::from_rgba(8,7,5,236));
    draw.add_rect(min,max,ImColor32::from_rgba(171,143,87,120)).thickness(1.0*s).build();
    let gold=ImColor32::from_rgba(214,180,112,230);let l=18.0*s;
    for (x,y,dx,dy) in [(min[0],min[1],1.0,1.0),(max[0],min[1],-1.0,1.0),(min[0],max[1],1.0,-1.0),(max[0],max[1],-1.0,-1.0)] {
        draw.add_line([x,y],[x+l*dx,y],gold).thickness(2.0*s).build();draw.add_line([x,y],[x,y+l*dy],gold).thickness(2.0*s).build();
    }
}
/// 两端渐隐的金色装饰线，中间一颗菱形
pub(crate) fn ornament(draw:&DrawListMut<'_>,center:[f32;2],half:f32,s:f32) {
    let (x,y)=(center[0],center[1]);
    draw.add_rect_filled_multicolor([x-half,y-0.7*s],[x-8.0*s,y+0.7*s],ImColor32::from_rgba(214,180,112,0),ImColor32::from_rgba(214,180,112,220),ImColor32::from_rgba(214,180,112,220),ImColor32::from_rgba(214,180,112,0));
    draw.add_rect_filled_multicolor([x+8.0*s,y-0.7*s],[x+half,y+0.7*s],ImColor32::from_rgba(214,180,112,220),ImColor32::from_rgba(214,180,112,0),ImColor32::from_rgba(214,180,112,0),ImColor32::from_rgba(214,180,112,220));
    draw.add_polyline(vec![[x,y-5.0*s],[x+5.0*s,y],[x,y+5.0*s],[x-5.0*s,y]],ImColor32::from_rgba(214,180,112,240)).filled(true).build();
}
/// 苦难律法选择界面：左侧路线图，右侧已选列表，底部分数与开始按钮
pub fn draw(ui:&Ui,screen:&mut LawScreen,icon:&mut dyn FnMut(u32)->Option<TextureId>,record:Option<(u32,&str)>)->Option<LawAction> {
    let size=ui.io().display_size;let s=(size[0]/1600.0).min(size[1]/900.0);
    let (ox,oy)=((size[0]-1600.0*s)*0.5,(size[1]-900.0*s)*0.5);let p=|x:f32,y:f32|[ox+x*s,oy+y*s];
    let background=ui.get_background_draw_list();crate::oath_art::background(&background,size);
    let shade=|a:u8|ImColor32::from_rgba(0,0,0,a);
    background.add_rect_filled_multicolor([0.0,0.0],[size[0]*0.3,size[1]],shade(170),shade(0),shade(0),shade(170));
    background.add_rect_filled_multicolor([size[0]*0.7,0.0],size,shade(0),shade(170),shade(170),shade(0));
    background.add_rect_filled_multicolor([0.0,size[1]*0.72],size,shade(0),shade(0),shade(190),shade(190));
    drop(background);
    let opened=*screen.opened.get_or_insert(ui.time());let ready=ui.time()-opened>=0.35;
    let key=|k:Key|ready&&ui.is_key_pressed(k);
    if key(Key::GamepadDpadLeft){screen.focus=moved(screen.focus,-1,0);}
    if key(Key::GamepadDpadRight){screen.focus=moved(screen.focus,1,0);}
    if key(Key::GamepadDpadUp){screen.focus=moved(screen.focus,0,-1);}
    if key(Key::GamepadDpadDown){screen.focus=moved(screen.focus,0,1);}
    if key(Key::GamepadFaceDown){screen.law=screen.law.toggle(screen.focus);}
    if key(Key::GamepadFaceUp){screen.law=Law::default();}
    if ui.io().key_ctrl&&key(Key::V) {if let Some(law)=ui.clipboard_text().and_then(|text|Law::parse(&text)){screen.law=law;}}
    let mut action=if key(Key::Enter)||key(Key::GamepadFaceLeft){Some(LawAction::Start(screen.law))}else if key(Key::Escape)||key(Key::GamepadFaceRight){Some(LawAction::Back)}
        else if key(crate::options::key(erdtree_tower::options::Hotkey::Archive))||key(Key::GamepadBack){Some(LawAction::Archive)}else{None};
    let mut hovered=None;let mut copy=false;
    ui.window("苦难律法##law").position([0.0,0.0],Condition::Always).size(size,Condition::Always)
        .flags(WindowFlags::NO_DECORATION|WindowFlags::NO_BACKGROUND|WindowFlags::NO_MOVE|WindowFlags::NO_NAV|WindowFlags::NO_SCROLL_WITH_MOUSE).build(|| {
        let text=|x:f32,y:f32,value:&str,color:[f32;4],scale:f32,align:f32| {
            ui.set_window_font_scale(scale*s);ui.set_cursor_screen_pos([ox+x*s-ui.calc_text_size(value)[0]*align,oy+y*s]);ui.text_colored(color,value);
        };
        let button=|label:&str,x:f32,y:f32,w:f32,h:f32,scale:f32|->bool {
            let pos=p(x,y);ui.set_cursor_screen_pos(pos);let clicked=ui.invisible_button(label,[w*s,h*s]);
            crate::oath_art::button(&ui.get_window_draw_list(),pos,[pos[0]+w*s,pos[1]+h*s],if ui.is_item_hovered(){255}else{210});
            text(x+w*0.5,y+(h-20.0*scale)*0.5,label,GOLD,scale,0.5);clicked&&ready
        };
        let draw=ui.get_window_draw_list();
        ornament(&draw,p(800.0,92.0),230.0*s,s);
        panel(&draw,p(36.0,116.0),p(1046.0,778.0),s);
        panel(&draw,p(1066.0,116.0),p(1564.0,778.0),s);
        panel(&draw,p(1340.0,30.0),p(1564.0,90.0),s);
        drop(draw);
        if button("返回  [Esc / B]",36.0,34.0,196.0,52.0,0.9){action=Some(LawAction::Back);}
        if button(&format!("旅程档案  [{} / View]",crate::options::label(erdtree_tower::options::Hotkey::Archive)),246.0,34.0,236.0,52.0,0.85){action=Some(LawAction::Archive);}
        text(800.0,26.0,"苦 难 律 法",GOLD,2.0,0.5);
        let subtitle=screen.seed.map_or("接受的苦难越多，分数越高".to_string(),|seed|format!("以种子 {seed} 开局 · 接受的苦难越多，分数越高"));
        text(800.0,100.0,&subtitle,MUTED,0.8,0.5);
        match record {
            Some((best,class))=>{text(1356.0,40.0,&format!("{class} · 历史最高"),MUTED,0.8,0.0);text(1548.0,40.0,&best.to_string(),GOLD,1.45,1.0);},
            None=>text(1452.0,48.0,"尚无纪录",MUTED,0.85,0.5),
        }
        let draw=ui.get_window_draw_list();
        let gray=ImColor32::from_rgba(118,108,90,140);let lit=ImColor32::from_rgba(222,180,98,240);
        for b in 0..BRANCHES.len() {
            let ids:Vec<_>=(0..NODES.len()).filter(|&i|NODES[i].branch==b).collect();
            let top=ids.iter().map(|&i|node_pos(i)[1]).fold(f32::MAX,f32::min);let bottom=ids.iter().map(|&i|node_pos(i)[1]+CARD[1]).fold(f32::MIN,f32::max);
            if b>0 {draw.add_line(p(54.0,top-9.0),p(1028.0,top-9.0),ImColor32::from_rgba(171,143,87,45)).thickness(1.0*s).build();}
            draw.add_rect(p(56.0,top+4.0),p(60.0,bottom-4.0),rgba(ACCENT[b],210)).filled(true).build();
        }
        let first=node_pos(0)[1];let last=(0..NODES.len()).filter(|&i|NODES[i].parent.is_none()).map(|i|node_pos(i)[1]).fold(f32::MIN,f32::max)+CARD[1]*0.5;
        let lit_to=(0..NODES.len()).filter(|&i|NODES[i].parent.is_none()&&screen.law.has(i)).map(|i|node_pos(i)[1]+CARD[1]*0.5).fold(f32::NAN,f32::max);
        draw.add_line(p(TRUNK,first-10.0),p(TRUNK,last),gray).thickness(2.5*s).build();
        if lit_to.is_finite(){draw.add_line(p(TRUNK,first-10.0),p(TRUNK,lit_to),lit).thickness(3.5*s).build();}
        let root=p(TRUNK,first-14.0);draw.add_polyline(vec![[root[0],root[1]-7.0*s],[root[0]+7.0*s,root[1]],[root[0],root[1]+7.0*s],[root[0]-7.0*s,root[1]]],lit).filled(true).build();
        for (id,node) in NODES.iter().enumerate() {
            let [x,y]=node_pos(id);let cy=y+CARD[1]*0.5;let from=node.parent.map_or(TRUNK,|parent|node_pos(parent)[0]+CARD[0]);
            let on=screen.law.has(id);draw.add_line(p(from,cy),p(x,cy),if on{lit}else{gray}).thickness(if on{3.5}else{2.5}*s).build();
            if on {draw.add_circle(p(from,cy),3.5*s,lit).filled(true).build();}
        }
        drop(draw);
        for (b,name) in BRANCHES.iter().enumerate() {
            let ids:Vec<_>=(0..NODES.len()).filter(|&i|NODES[i].branch==b).collect();
            let mid=(ids.iter().map(|&i|node_pos(i)[1]).fold(f32::MAX,f32::min)+ids.iter().map(|&i|node_pos(i)[1]+CARD[1]).fold(f32::MIN,f32::max))*0.5;
            text(70.0,mid-22.0,name,GOLD,1.1,0.0);text(70.0,mid+4.0,NOTES[b],MUTED,0.72,0.0);
        }
        for (id,node) in NODES.iter().enumerate() {
            let [x,y]=node_pos(id);let (min,max)=(p(x,y),p(x+CARD[0],y+CARD[1]));
            ui.set_cursor_screen_pos(min);if ui.invisible_button(format!("##law-{id}"),[CARD[0]*s,CARD[1]*s])&&ready{screen.law=screen.law.toggle(id);screen.focus=id;}
            if ui.is_item_hovered(){hovered=Some(id);}
            let (chosen,open)=(screen.law.has(id),screen.law.available(id));let focus=screen.focus==id||hovered==Some(id);
            let draw=ui.get_window_draw_list();
            if chosen {
                draw.add_rect(p(x-3.0,y-3.0),p(x+CARD[0]+3.0,y+CARD[1]+3.0),ImColor32::from_rgba(222,170,90,50)).filled(true).rounding(6.0*s).build();
                draw.add_rect_filled_multicolor(min,max,ImColor32::from_rgba(122,38,26,245),ImColor32::from_rgba(58,20,14,245),ImColor32::from_rgba(40,14,10,245),ImColor32::from_rgba(96,30,20,245));
            } else {
                let a=if open{235}else{200};
                draw.add_rect_filled_multicolor(min,max,ImColor32::from_rgba(34,29,22,a),ImColor32::from_rgba(20,17,13,a),ImColor32::from_rgba(14,12,9,a),ImColor32::from_rgba(26,22,17,a));
            }
            draw.add_rect(p(x+5.0,y+5.0),p(x+49.0,y+49.0),ImColor32::from_rgba(0,0,0,150)).filled(true).rounding(3.0*s).build();
            if let Some(texture)=icon(node.icon) {
                let light=if chosen{255}else if open{200}else{70};
                draw.add_image(texture,p(x+6.0,y+6.0),p(x+48.0,y+48.0)).col(ImColor32::from_rgba(light,light,light,255)).build();
            }
            let border=if chosen{lit}else if focus{ImColor32::from_rgba(236,220,176,230)}else if open{ImColor32::from_rgba(150,126,84,200)}else{ImColor32::from_rgba(80,70,54,170)};
            draw.add_rect(min,max,border).rounding(4.0*s).thickness(if chosen||focus{2.2}else{1.0}*s).build();
            let tri=if chosen{ImColor32::from_rgba(255,236,200,255)}else if open{lit}else{gray};
            draw.add_triangle(p(x+56.0,y+46.0),p(x+66.0,y+46.0),p(x+61.0,y+36.5),tri).filled(true).build();
            if chosen {
                draw.add_circle(p(x+CARD[0]-11.0,y+11.0),8.0*s,lit).filled(true).build();
                draw.add_polyline(vec![p(x+CARD[0]-15.0,y+11.0),p(x+CARD[0]-12.0,y+14.5),p(x+CARD[0]-6.5,y+7.5)],ImColor32::from_rgba(30,18,10,255)).thickness(2.0*s).build();
            }
            drop(draw);
            let dim=|c:[f32;4]|if chosen||open{c}else{[c[0]*0.55,c[1]*0.55,c[2]*0.55,1.0]};
            text(x+55.0,y+6.0,node.name,dim(if chosen{WHITE}else{GOLD}),0.78,0.0);
            text(x+70.0,y+31.0,&node.points.to_string(),dim(if chosen{WHITE}else{GOLD}),0.8,0.0);
        }
        let selected=screen.law.selected();
        text(1088.0,132.0,"已选律法",GOLD,1.1,0.0);
        let draw=ui.get_window_draw_list();
        draw.add_rect(p(1482.0,132.0),p(1544.0,158.0),ImColor32::from_rgba(171,143,87,60)).filled(true).rounding(13.0*s).build();drop(draw);
        text(1513.0,135.0,&format!("{} 项",selected.len()),GOLD,0.8,0.5);
        ui.set_cursor_screen_pos(p(1080.0,172.0));
        ui.child_window("##law-list").size([476.0*s,594.0*s]).build(|| {
            let origin=ui.cursor_screen_pos();
            if selected.is_empty() {
                ui.set_window_font_scale(0.92*s);let line="尚未选择律法";let w=ui.calc_text_size(line)[0];
                ui.set_cursor_screen_pos([origin[0]+238.0*s-w*0.5,origin[1]+230.0*s]);ui.text_colored(MUTED,line);
                ui.set_window_font_scale(0.78*s);let hint="点击左侧节点，接受对应的限制";let w=ui.calc_text_size(hint)[0];
                ui.set_cursor_screen_pos([origin[0]+238.0*s-w*0.5,origin[1]+262.0*s]);ui.text_colored(MUTED,hint);
            }
            for (row,&id) in selected.iter().enumerate() {
                let node=&NODES[id];let (left,top)=(origin[0]+4.0*s,origin[1]+row as f32*62.0*s);let right=left+452.0*s;
                let draw=ui.get_window_draw_list();
                draw.add_rect_filled_multicolor([left,top],[right,top+56.0*s],ImColor32::from_rgba(40,32,22,200),ImColor32::from_rgba(20,16,11,140),ImColor32::from_rgba(20,16,11,140),ImColor32::from_rgba(40,32,22,200));
                draw.add_rect([left,top],[left+4.0*s,top+56.0*s],rgba(ACCENT[node.branch],230)).filled(true).build();
                draw.add_rect([left+16.0*s,top+12.0*s],[left+70.0*s,top+44.0*s],ImColor32::from_rgba(150,52,34,245)).filled(true).rounding(4.0*s).build();
                draw.add_triangle([left+24.0*s,top+37.0*s],[left+34.0*s,top+37.0*s],[left+29.0*s,top+19.0*s],ImColor32::from_rgba(255,236,200,255)).filled(true).build();
                drop(draw);
                ui.set_window_font_scale(0.95*s);ui.set_cursor_screen_pos([left+38.0*s,top+16.0*s]);ui.text_colored(WHITE,node.points.to_string());
                ui.set_cursor_screen_pos([left+84.0*s,top+5.0*s]);ui.text_colored(GOLD,node.name);
                ui.set_window_font_scale(0.84*s);ui.set_cursor_screen_pos([left+84.0*s,top+30.0*s]);ui.text_colored(WHITE,node.effect);
            }
            ui.set_cursor_screen_pos([origin[0],origin[1]+selected.len() as f32*62.0*s]);ui.dummy([1.0,1.0]);
        });
        if let Some(id)=hovered {
            let node=&NODES[id];let [x,y]=node_pos(id);
            let lines=[node.name.to_string(),node.effect.to_string(),if screen.law.has(id){"已选择 · 再次点击取消（其后的节点一并取消）".into()}else if screen.law.available(id){format!("点击接受 · +{} 分",node.points)}else{format!("需要先选择「{}」",NODES[node.parent.unwrap()].name)}];
            ui.set_window_font_scale(0.85*s);let w=lines.iter().map(|l|ui.calc_text_size(l)[0]).fold(0.0,f32::max)/s+28.0;
            let (tx,ty)=((x+12.0).min(1040.0-w),if y+CARD[1]+90.0>776.0 {y-86.0}else{y+CARD[1]+8.0});
            let draw=ui.get_window_draw_list();panel(&draw,p(tx,ty),p(tx+w,ty+80.0),s);drop(draw);
            for (i,line) in lines.iter().enumerate(){text(tx+14.0,ty+8.0+i as f32*22.0,line,[GOLD,WHITE,MUTED][i],0.85,0.0);}
        }
        let score=screen.law.score();let grade=GRADES.iter().rposition(|(min,_)|score>=*min);
        text(40.0,796.0,"律法码",MUTED,0.8,0.0);text(104.0,792.0,&screen.law.code(),GOLD,1.1,0.0);
        copy=button(if screen.copied.is_some_and(|t|ui.time()-t<1.5){"已复制"}else{"复制"},212.0,790.0,96.0,34.0,0.75);
        if button("清空  [Y]",40.0,836.0,170.0,44.0,0.85){screen.law=Law::default();}
        text(226.0,848.0,"Ctrl+V 粘贴律法码 · 取消节点会连带取消其后的节点",MUTED,0.72,0.0);
        text(700.0,790.0,"当前分数",MUTED,0.8,1.0);
        text(700.0,806.0,&score.to_string(),GOLD,2.7,1.0);
        let draw=ui.get_window_draw_list();
        let (label,color)=grade.map_or(("普通",(120,112,96)),|g|(GRADES[g].1,GRADE_COLORS[g]));
        draw.add_rect(p(722.0,804.0),p(800.0,836.0),rgba(color,70)).filled(true).rounding(16.0*s).build();
        draw.add_rect(p(722.0,804.0),p(800.0,836.0),rgba(color,220)).rounding(16.0*s).thickness(1.4*s).build();
        let (bx,by,bw)=(722.0,856.0,400.0);
        draw.add_rect(p(bx,by),p(bx+bw,by+6.0),ImColor32::from_rgba(56,50,40,230)).filled(true).rounding(3.0*s).build();
        draw.add_rect_filled_multicolor(p(bx,by),p(bx+bw*(score.min(420) as f32/420.0),by+6.0),ImColor32::from_rgba(150,110,60,255),lit,lit,ImColor32::from_rgba(150,110,60,255));
        drop(draw);
        text(761.0,808.0,label,grade.map_or(MUTED,|_|RED),0.95,0.5);
        let next=GRADES.iter().find(|(min,_)|score<*min);
        text(816.0,812.0,&next.map_or("已达最高评级".to_string(),|(min,name)|format!("距离「{name}」还差 {} 分",min-score)),MUTED,0.78,0.0);
        for (i,(min,name)) in GRADES.iter().enumerate() {
            let x=bx+bw*(*min as f32)/420.0;let draw=ui.get_window_draw_list();
            draw.add_line(p(x,by-3.0),p(x,by+9.0),rgba(GRADE_COLORS[i],230)).thickness(1.6*s).build();drop(draw);
            text(x,by+12.0,name,if grade.is_some_and(|g|g>=i){RED}else{MUTED},0.74,0.5);
        }
        if button("开始旅程  [Enter / X]",1180.0,796.0,384.0,78.0,1.15){action=Some(LawAction::Start(screen.law));}
    });
    if copy {ui.set_clipboard_text(screen.law.code());screen.copied=Some(ui.time());}
    action
}
