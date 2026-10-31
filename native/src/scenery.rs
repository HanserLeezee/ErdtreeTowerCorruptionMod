// 主题幕的“场景”部分（src/themes.rs）：开幕标题卡、路线图配色、竞技场的时间。不改敌人与数值。
// 插画按需加载：scene-assets/theme-art/<art>.rgba（800 × 450），只保留当前主题这一张。
use std::sync::Mutex;
use std::time::Instant;
use hudhook::{RenderContext,imgui::{ImColor32,TextureId,Ui}};
use erdtree_tower::{Phase,Run,themes::Theme};

const ART_SIZE:(u32,u32)=(800,450);
/// 想要的插画、已经载入的插画与贴图
static ART:Mutex<(Option<&'static str>,Option<(&'static str,Option<TextureId>)>)>=Mutex::new((None,None));
/// 开幕标题卡：正在显示的主题与开始时间、上一次显示过的 (幕种子, 幕)
static CARD:Mutex<(Option<(&'static Theme,Instant)>,Option<(u64,u8)>)>=Mutex::new((None,None));
const CARD_SECONDS:f32=4.5;

pub fn theme(run:&Run)->&'static Theme {erdtree_tower::themes::for_run(run)}

/// before_render：上传当前主题的插画（换主题时换掉）
pub fn load(renderer:&mut dyn RenderContext) {
    let Ok(mut art)=ART.lock() else{return};
    let Some(wanted)=art.0 else{return};
    if art.1.is_some_and(|(name,_)|name==wanted) {return;}
    let path=crate::LOG_PATH.with_file_name("scene-assets").join("theme-art").join(format!("{wanted}.rgba"));
    let texture=match std::fs::read(&path) {
        Ok(data) if data.len()==(ART_SIZE.0*ART_SIZE.1*4) as usize=>renderer.load_texture(&data,ART_SIZE.0,ART_SIZE.1).map_err(|e|crate::log(format!("Theme art upload failed: {wanted}: {e:?}"))).ok(),
        _=>{crate::log(format!("Theme art missing: {}",path.display()));None},
    };
    art.1=Some((wanted,texture));
}
fn texture(name:&'static str)->Option<TextureId> {
    let Ok(mut art)=ART.lock() else{return None};
    art.0=Some(name);
    art.1.filter(|(loaded,_)|*loaded==name).and_then(|(_,t)|t)
}

/// 每帧：进入新的一幕（路线图、还没走任何节点）时显示一次标题卡
pub fn observe(run:&Run) {
    if run.phase!=Phase::Map||!run.visited.is_empty() {return;}
    let Ok(mut card)=CARD.lock() else{return};
    let key=(run.seed,run.act);
    if card.1==Some(key) {return;}
    card.1=Some(key);
    let theme=theme(run);texture(theme.art);
    card.0=Some((theme,Instant::now()));
    crate::log(format!("Act {} theme: {} ({})",run.act,theme.name,theme.region));
}
fn rgb(c:u32,a:f32)->ImColor32 {ImColor32::from_rgba((c>>16) as u8,(c>>8) as u8,c as u8,(a.clamp(0.0,1.0)*255.0) as u8)}
/// 开幕标题卡：画在最前层，不拦截输入；淡入 0.6 秒、停留、最后 0.9 秒淡出
pub fn draw_card(ui:&Ui) {
    let Some((theme,started))=CARD.lock().ok().and_then(|c|c.0) else{return};
    let t=started.elapsed().as_secs_f32();
    if t>CARD_SECONDS {if let Ok(mut c)=CARD.lock(){c.0=None;}return;}
    let alpha=(t/0.6).min(1.0).min((CARD_SECONDS-t)/0.9);
    let size=ui.io().display_size;let s=(size[1]/1080.0).max(0.6);
    let list=ui.get_foreground_draw_list();
    list.add_rect([0.0,0.0],size,rgb(0x060505,0.86*alpha)).filled(true).build();
    // 插画：铺满屏幕（按比例裁切），轻微推近
    if let Some(id)=texture(theme.art) {
        let zoom=1.0+0.04*(t/CARD_SECONDS);
        let ratio=size[0]/size[1];let native=ART_SIZE.0 as f32/ART_SIZE.1 as f32;
        let (w,h)=if ratio>native {(size[0]*zoom,size[0]*zoom/native)}else{(size[1]*zoom*native,size[1]*zoom)};
        let min=[(size[0]-w)*0.5,(size[1]-h)*0.5];
        list.add_image(id,min,[min[0]+w,min[1]+h]).col(ImColor32::from_rgba(255,255,255,(alpha*235.0) as u8)).build();
    }
    // 下方压暗，放文字
    let band=size[1]*0.5;
    list.add_rect_filled_multicolor([0.0,band],[size[0],size[1]*0.66],rgb(0x050404,0.0),rgb(0x050404,0.0),rgb(0x050404,0.8*alpha),rgb(0x050404,0.8*alpha));
    list.add_rect([0.0,size[1]*0.66],size,rgb(0x050404,0.88*alpha)).filled(true).build();
    drop(list);
    use crate::fancy_text::{Effect,Layer,Style,draw,measure};
    let center=|text:&str,y:f32,k:f32,style:&Style| {let w=measure(text,k*s);draw(Layer::Foreground,[(size[0]-w)*0.5,y],k*s,text,style,t,alpha);};
    let (accent,paper)=(theme.accent,theme.paper);
    let tint=|c:u32|[(c>>16) as u8,(c>>8) as u8,c as u8];
    let act=["第一幕","第二幕","第三幕"][usize::from(theme.act.clamp(1,3)-1)];
    center(act,size[1]*0.70,30.0,&Style{color:tint(paper),accent:tint(paper),effect:Effect::Plain,glow:false});
    // 主题名用浅色字、主题强调色做流光与光晕：强调色直接写字会和同色系的插画混在一起
    center(theme.name,size[1]*0.70+46.0*s,72.0,&Style{color:tint(paper),accent:tint(accent),effect:Effect::Shimmer,glow:true});
    center(theme.region,size[1]*0.70+136.0*s,26.0,&Style{color:tint(paper),accent:tint(paper),effect:Effect::Plain,glow:false});
    center(theme.tagline,size[1]*0.70+176.0*s,22.0,&Style{color:[180,170,150],accent:[180,170,150],effect:Effect::Plain,glow:false});
}

/// 路线图配色：纸色（乘到纸张贴图上）、墨色、强调色
pub fn palette(run:&Run)->(ImColor32,ImColor32,ImColor32) {let t=theme(run);(rgb(t.paper,1.0),rgb(t.ink,1.0),rgb(t.accent,1.0))}
pub fn ink_rgba(run:&Run)->[f32;4] {let c=theme(run).ink;[((c>>16)&255) as f32/255.0,((c>>8)&255) as f32/255.0,(c&255) as f32/255.0,1.0]}

#[cfg(test)]
mod tests {
    #[test]
    fn every_theme_has_its_art_file_and_a_readable_palette() {
        for theme in erdtree_tower::themes::THEMES.iter() {
            let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../scene-assets/theme-art").join(format!("{}.rgba",theme.art));
            assert_eq!(std::fs::metadata(&path).map(|m|m.len()).unwrap_or(0),u64::from(super::ART_SIZE.0*super::ART_SIZE.1*4),"{}",theme.name);
            let luma=|c:u32|((c>>16)&255)*3+((c>>8)&255)*6+(c&255);
            assert!(luma(theme.paper)>luma(theme.ink)*3,"{}: ink must stand out on paper",theme.name);
        }
    }
}
