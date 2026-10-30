use std::sync::atomic::{AtomicUsize,Ordering};
use hudhook::{RenderContext,imgui::{DrawListMut,ImColor32,TextureId}};
static TEXTURES:[AtomicUsize;5]=[AtomicUsize::new(usize::MAX),AtomicUsize::new(usize::MAX),AtomicUsize::new(usize::MAX),AtomicUsize::new(usize::MAX),AtomicUsize::new(usize::MAX)];
fn texture(index:usize)->Option<TextureId> {Some(TEXTURES[index].load(Ordering::Relaxed)).filter(|&id|id!=usize::MAX).map(TextureId::new)}
/// scene-assets 下的贴图（不含扩展名）与尺寸；职业插画放在 class-art，其余在 oath-art
/// [HansFix] oath-art/cards2：新誓约封面（磐石、吊命、贪婪、灵辉、锻铁、疫血），同样 3 列 × 2 行
pub const ASSETS:[(&str,u32,u32);5]=[("oath-art/cards",1254,1254),("oath-art/background",1672,941),("oath-art/button",2172,724),("class-art/classes",1536,1024),("oath-art/cards2",1680,1680)];
pub fn initialize(renderer:&mut dyn RenderContext) {
    for (index,(name,w,h)) in ASSETS.into_iter().enumerate() {
        TEXTURES[index].store(usize::MAX,Ordering::Relaxed);
        let path=crate::LOG_PATH.with_file_name("scene-assets").join(format!("{name}.rgba"));
        match std::fs::read(path) {
            Ok(data) if data.len()==w as usize*h as usize*4=>match renderer.load_texture(&data,w,h) {
                Ok(id)=>TEXTURES[index].store(id.id(),Ordering::Relaxed),
                Err(e)=>crate::log(format!("Oath texture upload failed: {name}: {e:?}")),
            },
            _ if index==4=>{},
            _=>crate::log(format!("Oath texture missing or invalid: {name}")),
        }
    }
}
/// 新封面图集（4）只随新誓约补丁发布，缺少时退回原图集，不影响誓约界面
pub fn ready()->bool {(0..4).all(|i|texture(i).is_some())}
fn image(draw:&DrawListMut<'_>,index:usize,min:[f32;2],max:[f32;2],uv:([f32;2],[f32;2]),light:u8) {
    let Some(id)=texture(index) else{return;};
    draw.add_image(id,min,max).uv_min(uv.0).uv_max(uv.1).col(ImColor32::from_rgba(light,light,light,255)).build();
}
pub fn background(draw:&DrawListMut<'_>,size:[f32;2]) {
    let ratio=size[0]/size[1];let native=1672.0/941.0;
    let uv=if ratio>native {let d=(1.0-native/ratio)*0.5;([0.0,d],[1.0,1.0-d])}else{let d=(1.0-ratio/native)*0.5;([d,0.0],[1.0-d,1.0])};
    image(draw,1,[0.0,0.0],size,uv,255);
}
/// 卡面数量：原图集 6 张 + 新封面 6 张
pub const CARDS:usize=12;
/// 新封面的插画区更高，文字要放得更低
pub fn tall_art(kind:usize)->bool {kind%CARDS>=6}
pub fn card(draw:&DrawListMut<'_>,kind:usize,min:[f32;2],max:[f32;2],light:u8) {
    let kind=kind%CARDS;
    if kind>=6&&texture(4).is_none() {return card(draw,kind%6,min,max,light);}
    if kind>=6 {
        let kind=kind-6;let col=(kind%3) as f32;let row=(kind/3) as f32;
        image(draw,4,min,max,([(col+0.002)/3.0,(row+0.002)/2.0],[(col+0.998)/3.0,(row+0.998)/2.0]),light);
        return;
    }
    let col=(kind%3) as f32;let row=(kind/3) as f32;
    image(draw,0,min,max,([(col+0.005)/3.0,(row+0.003)/2.0],[(col+0.995)/3.0,(row+0.997)/2.0]),light);
}
/// 职业插画：3 列 × 2 行图集中的第 index 幅，按目标区域的宽高比从顶部裁切（人物在画面上部）；zoom 大于 1 时向中心推近
pub fn class_art(draw:&DrawListMut<'_>,index:usize,min:[f32;2],max:[f32;2],light:u8,zoom:f32) {
    let (col,row)=((index%3) as f32,((index/3)%2) as f32);
    let aspect=(max[0]-min[0])/(max[1]-min[1]).max(1.0);
    // 每格是正方形：宽于方形时裁掉上下，否则裁掉左右
    let (w,h)=if aspect>=1.0 {(1.0,1.0/aspect)}else{(aspect,1.0)};
    let (w,h)=(w/zoom.max(1.0),h/zoom.max(1.0));
    let (u,v)=((1.0-w)*0.5,(1.0-h)*0.3);
    image(draw,3,min,max,([(col+u)/3.0,(row+v)/2.0],[(col+u+w)/3.0,(row+v+h)/2.0]),light);
}
pub fn button(draw:&DrawListMut<'_>,min:[f32;2],max:[f32;2],light:u8) {
    image(draw,2,min,max,([0.0,0.20],[1.0,0.80]),light);
}
#[cfg(test)]
pub fn preview(){for (i,t) in TEXTURES.iter().enumerate(){t.store(if i==4{200010}else{200000+i},Ordering::Relaxed);}}
#[test]
fn generated_texture_dimensions_are_valid() {
    for (name,w,h) in ASSETS {
        let bytes=std::fs::read(format!("../scene-assets/{name}.rgba")).unwrap();
        assert_eq!(bytes.len(),w as usize*h as usize*4);
    }
}
