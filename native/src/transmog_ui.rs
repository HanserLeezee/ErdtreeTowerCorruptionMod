use hudhook::imgui::{Ui,TextureId,Key,ImColor32};
use erdtree_tower::transmog::{PARTS,BARE};
use crate::{transmog,run_screen::wrapped,law_ui::panel};
const WHITE:[f32;4]=[0.95,0.93,0.86,1.0];
const GOLD:[f32;4]=[0.94,0.82,0.56,1.0];
const DIM:[f32;4]=[0.72,0.70,0.65,1.0];
#[derive(Default)]
pub struct Page {part:usize,page:usize,selection:usize,search:String,notice:String}
fn apply(page:&mut Page,id:i32) {
    let result=crate::legacy::owner().ok_or("请先载入角色".to_string()).and_then(|owner|transmog::choose(owner,page.part,id));
    page.notice=match result {Ok(())=>"外观已保存，属性与词条不变".into(),Err(error)=>error};
}
#[allow(clippy::too_many_arguments)]
pub fn draw(ui:&Ui,page:&mut Page,icon:&mut dyn FnMut(u32)->Option<TextureId>,ready:bool,s:f32,ox:f32,oy:f32) {
    let p=|x:f32,y:f32|[ox+x*s,oy+y*s];
    let text=|x:f32,y:f32,value:&str,color:[f32;4],scale:f32|{ui.set_window_font_scale(scale*s);ui.set_cursor_screen_pos(p(x,y));ui.text_colored(color,value);};
    text(76.0,180.0,"防 具 幻 化",GOLD,1.4);
    text(310.0,189.0,"只改变外观，保留当前装备的防御、重量与词条",DIM,0.92);
    for (part,name) in PARTS.iter().enumerate() {
        let x=76.0+part as f32*250.0;ui.set_cursor_screen_pos(p(x,230.0));ui.set_window_font_scale(s);
        if ui.invisible_button(format!("transmog-part-{part}"),[230.0*s,42.0*s])&&ready {page.part=part;page.page=0;page.selection=0;}
        if page.part==part {ui.get_window_draw_list().add_rect(p(x,230.0),p(x+230.0,272.0),ImColor32::from_rgba(81,65,35,220)).filled(true).build();}
        text(x+18.0,240.0,name,if page.part==part{GOLD}else{DIM},1.1);
    }
    ui.set_cursor_screen_pos(p(76.0,285.0));ui.set_window_font_scale(s);ui.set_next_item_width(500.0*s);
    if ui.input_text("搜索防具##transmog",&mut page.search).build(){page.page=0;page.selection=0;}
    let typing=ui.is_item_active();
    let choices:Vec<_>=transmog::catalog().iter().filter(|a|a.slot==page.part&&a.name.contains(page.search.trim())&&erdtree_tower::dlc::item_allowed(0x10000000|a.id as u32)).collect();
    let pages=choices.len().div_ceil(12).max(1);page.page=page.page.min(pages-1);
    let key=|k|ready&&!typing&&ui.is_key_pressed(k);
    if key(Key::PageUp)||key(Key::GamepadL2){page.page=page.page.saturating_sub(1);page.selection=0;}
    if key(Key::PageDown)||key(Key::GamepadR2){page.page=(page.page+1).min(pages-1);page.selection=0;}
    let visible:Vec<_>=choices.iter().skip(page.page*12).take(12).copied().collect();
    if !visible.is_empty() {
        if key(Key::LeftArrow)||key(Key::GamepadDpadLeft){page.selection=page.selection.saturating_sub(1);}
        if key(Key::RightArrow)||key(Key::GamepadDpadRight){page.selection=(page.selection+1).min(visible.len()-1);}
        if key(Key::UpArrow)||key(Key::GamepadDpadUp){page.selection=page.selection.saturating_sub(4);}
        if key(Key::DownArrow)||key(Key::GamepadDpadDown){page.selection=(page.selection+4).min(visible.len()-1);}
        page.selection=page.selection.min(visible.len()-1);
        if key(Key::Enter)||key(Key::GamepadFaceDown){apply(page,visible[page.selection].id);}
    }
    let worn=transmog::current();
    for (i,item) in visible.iter().enumerate() {
        let (x,y)=(76.0+(i%4) as f32*250.0,335.0+(i/4) as f32*135.0);
        ui.set_cursor_screen_pos(p(x,y));ui.set_window_font_scale(s);
        if ui.invisible_button(format!("transmog-armor-{}",item.id),[232.0*s,120.0*s])&&ready{page.selection=i;apply(page,item.id);}
        let active=worn.armor[page.part]==item.id;let focus=page.selection==i||ui.is_item_hovered();
        let draw=ui.get_window_draw_list();
        draw.add_rect(p(x,y),p(x+232.0,y+120.0),ImColor32::from_rgba(23,23,20,240)).filled(true).rounding(4.0*s).build();
        draw.add_rect(p(x,y),p(x+232.0,y+120.0),if active||focus{ImColor32::from_rgba(226,194,132,230)}else{ImColor32::from_rgba(82,78,64,150)}).rounding(4.0*s).build();
        if let Some(texture)=icon(item.icon){draw.add_image(texture,p(x+8.0,y+10.0),p(x+88.0,y+90.0)).build();}drop(draw);
        wrapped(ui,p(x+94.0,y+18.0),ox+(x+221.0)*s,&item.name,if active{GOLD}else{WHITE},0.95*s);
        if active{text(x+94.0,y+90.0,"当前外观",GOLD,0.8);}
    }
    if visible.is_empty(){text(360.0,480.0,"没有匹配的防具",DIM,1.0);}
    for (label,x,next) in [("上一页",76.0,false),("下一页",850.0,true)] {
        ui.set_cursor_screen_pos(p(x,755.0));ui.set_window_font_scale(s);
        if ui.button_with_size(label,[180.0*s,38.0*s])&&ready {page.page=if next{(page.page+1).min(pages-1)}else{page.page.saturating_sub(1)};page.selection=0;}
    }
    text(460.0,764.0,&format!("{} / {} 页 · {} 件",page.page+1,pages,choices.len()),DIM,0.9);
    panel(&ui.get_window_draw_list(),p(1100.0,230.0),p(1530.0,800.0),s);
    text(1130.0,252.0,"当前外观",GOLD,1.25);
    for (part,name) in PARTS.iter().enumerate() {
        let id=worn.armor[part];let value=if id==-1{"跟随实际装备"}else if id==BARE[part]{"隐藏此部位"}else{transmog::catalog().iter().find(|a|a.id==id).map_or("未知防具",|a|a.name.as_str())};
        text(1130.0,310.0+part as f32*68.0,name,DIM,0.85);
        wrapped(ui,p(1215.0,310.0+part as f32*68.0),ox+1500.0*s,value,WHITE,0.9*s);
    }
    ui.set_window_font_scale(s);ui.set_cursor_screen_pos(p(1130.0,598.0));
    if ui.button_with_size("隐藏当前部位",[366.0*s,42.0*s])&&ready{apply(page,BARE[page.part]);}
    ui.set_cursor_screen_pos(p(1130.0,654.0));
    if ui.button_with_size("恢复当前部位",[366.0*s,42.0*s])&&ready{apply(page,-1);}
    let status=if page.notice.is_empty(){transmog::status()}else{page.notice.clone()};
    wrapped(ui,p(1130.0,727.0),ox+1500.0*s,&status,DIM,0.85*s);
}
