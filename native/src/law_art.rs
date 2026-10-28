// 律法路线的专属图标（原创生成，不用原版图标）：scene-assets/law-art/icons.rgba，5 × 4 格、每格 160 像素，
// 第 N 格对应 src/law.rs 的 Node::art（scene-builder/law_icons.py 生成）。
use std::sync::atomic::{AtomicUsize,Ordering};
use hudhook::{RenderContext,imgui::{DrawListMut,ImColor32,TextureId}};
static ATLAS:AtomicUsize=AtomicUsize::new(usize::MAX);
const CELL:u32=160;const COLUMNS:u32=5;const ROWS:u32=4;
pub fn initialize(renderer:&mut dyn RenderContext) {
    ATLAS.store(usize::MAX,Ordering::Relaxed);
    let (width,height)=(CELL*COLUMNS,CELL*ROWS);
    match std::fs::read(crate::LOG_PATH.with_file_name("scene-assets").join("law-art").join("icons.rgba")) {
        Ok(data) if data.len()==(width*height*4) as usize=>match renderer.load_texture(&data,width,height) {
            Ok(id)=>ATLAS.store(id.id(),Ordering::Relaxed),
            Err(e)=>crate::log(format!("Law icons upload failed: {e:?}")),
        },
        _=>crate::log("Law icons missing: scene-assets/law-art/icons.rgba"),
    }
}
/// 第 index 格在图标集里的 UV
fn uv(index:u8)->([f32;2],[f32;2]) {
    let (x,y)=(f32::from(index)%COLUMNS as f32,(u32::from(index)/COLUMNS) as f32);
    ([x/COLUMNS as f32,y/ROWS as f32],[(x+1.0)/COLUMNS as f32,(y+1.0)/ROWS as f32])
}
/// 画律法图标；图标集没加载时返回 false（调用方不画图标）
pub fn draw(draw:&DrawListMut<'_>,index:u8,min:[f32;2],max:[f32;2],light:u8)->bool {
    let Some(id)=Some(ATLAS.load(Ordering::Relaxed)).filter(|&id|id!=usize::MAX).map(TextureId::new) else{return false;};
    let (a,b)=uv(index);
    draw.add_image(id,min,max).uv_min(a).uv_max(b).col(ImColor32::from_rgba(light,light,light,255)).build();true
}
#[cfg(test)]
mod tests {
    #[test]
    fn cells_cover_the_grid_without_overlap() {
        assert_eq!(super::uv(0),([0.0,0.0],[0.2,0.25]));
        assert_eq!(super::uv(16),([0.2,0.75],[0.4,1.0]));
        assert!(erdtree_tower::law::NODES.iter().all(|n|u32::from(n.art)<super::COLUMNS*super::ROWS));
    }
}
