// 动态文字特效：把文字拆成逐个字形来画，每个字单独计算颜色与位置，做出流光、渐变、呼吸发光、波浪、抖动、残影等“着色器风格”的效果。
// ImGui 的渲染管线是固定的，不能插入自定义着色器；这里只用绘制列表叠加多层字形实现，开销是每个字多画十来次。
use hudhook::imgui::sys;

/// 界面字表（scene-builder/title_font.py 生成的 scene-assets/fonts/ui-glyphs.txt）转成 ImGui 的字形范围：
/// 正文黑体与标题宋体都只加载这些字，字体贴图比加载全部汉字小得多。缺文件时返回 None，由调用方退回原来的范围
pub fn ui_glyph_ranges()->Option<&'static [u32]> {
    static RANGES:std::sync::LazyLock<Option<Vec<u32>>>=std::sync::LazyLock::new(|| {
        let text=std::fs::read_to_string(crate::LOG_PATH.with_file_name("scene-assets").join("fonts").join("ui-glyphs.txt")).ok()?;
        Some(ranges(&text))
    });
    RANGES.as_deref()
}
/// 字符集合并成连续区间 [起, 止, 起, 止, …, 0]；总是包含可见 ASCII
fn ranges(text:&str)->Vec<u32> {
    let mut codes:Vec<u32>=text.chars().map(u32::from).filter(|c|*c>=0x80).chain(0x20..0x7f).collect();codes.sort_unstable();codes.dedup();
    let mut out=Vec::new();
    for code in codes {
        match out.last_mut() {Some(end) if *end+1==code=>*end=code,_=>{out.push(code);out.push(code);}}
    }
    out.push(0);out
}
/// 标题字体文件
pub fn title_font_path()->std::path::PathBuf {crate::LOG_PATH.with_file_name("scene-assets").join("fonts").join("TowerTitleSerif-Bold.otf")}
/// 标题字体的 ImGui 句柄：启动时加载后登记在这里，各界面画标题时推入。
/// 它指向字体图集里的字体，图集在整个渲染期间不会释放，且只在渲染线程上使用
#[derive(Clone,Copy)]
pub struct TitleFont(pub hudhook::imgui::FontId);
unsafe impl Send for TitleFont {}
unsafe impl Sync for TitleFont {}
static TITLE_FONT:std::sync::Mutex<Option<TitleFont>>=std::sync::Mutex::new(None);
pub fn set_title_font(font:TitleFont) {if let Ok(mut slot)=TITLE_FONT.lock(){*slot=Some(font);}}
pub fn title_font()->Option<hudhook::imgui::FontId> {TITLE_FONT.lock().ok().and_then(|f|*f).map(|f|f.0)}
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum Effect {
    /// 不加动态效果，只有描边（与特效文字并排时保持一致的外观）
    Plain,
    /// 一道高光从左到右扫过
    Shimmer,
    /// 颜色在主色与辅色之间沿文字流动
    Gradient,
    /// 整体亮度呼吸起伏
    Pulse,
    /// 每个字上下波动
    Wave,
    /// 每个字轻微不规则抖动
    Jitter,
    /// 身后拖着两道残影
    Afterimage,
    /// 每个字明暗不定地闪烁
    Flicker,
}
#[derive(Clone,Copy,Debug,PartialEq)]
pub struct Style {pub color:[u8;3],pub accent:[u8;3],pub effect:Effect,pub glow:bool}
/// 画到哪一层：前景（全屏 HUD）或当前窗口
#[derive(Clone,Copy)]
pub enum Layer {Foreground,Window}
fn list(layer:Layer)->*mut sys::ImDrawList {unsafe {match layer {Layer::Foreground=>sys::igGetForegroundDrawList(),Layer::Window=>sys::igGetWindowDrawList()}}}
fn rgba(c:[f32;3],a:f32)->u32 {
    let ch=|v:f32|(v.clamp(0.0,1.0)*255.0) as u32;
    ch(c[0])|ch(c[1])<<8|ch(c[2])<<16|ch(a)<<24
}
fn mix(a:[u8;3],b:[u8;3],t:f32)->[f32;3] {
    let t=t.clamp(0.0,1.0);[0,1,2].map(|i|(f32::from(a[i])*(1.0-t)+f32::from(b[i])*t)/255.0)
}
fn scale(c:[f32;3],k:f32)->[f32;3] {c.map(|v|v*k)}
/// 0..1 的确定性随机数
fn noise(i:u32,k:u32)->f32 {
    let mut x=i.wrapping_mul(0x9E37_79B1)^k.wrapping_mul(0x85EB_CA77);x^=x>>15;x=x.wrapping_mul(0x2C1B_3C6D);x^=x>>12;(x&0xffff) as f32/65535.0
}
/// 按当前字体、指定字号量文字宽度
pub fn measure(text:&str,size:f32)->f32 {
    if text.is_empty() {return 0.0;}
    let mut out=sys::ImVec2{x:0.0,y:0.0};
    unsafe {
        let font=sys::igGetFont();let range=text.as_bytes().as_ptr_range();
        sys::ImFont_CalcTextSizeA(&mut out,font,size,f32::MAX,0.0,range.start.cast(),range.end.cast(),std::ptr::null_mut());
    }
    out.x
}
fn glyph(list:*mut sys::ImDrawList,size:f32,pos:[f32;2],color:u32,text:&str) {
    let range=text.as_bytes().as_ptr_range();
    unsafe {sys::ImDrawList_AddText_FontPtr(list,sys::igGetFont(),size,sys::ImVec2{x:pos[0],y:pos[1]},color,range.start.cast(),range.end.cast(),0.0,std::ptr::null());}
}
/// 从 pos（左上角）起画一段带特效的文字，返回画完后的宽度
pub fn draw(layer:Layer,pos:[f32;2],size:f32,text:&str,style:&Style,time:f32,alpha:f32)->f32 {
    let list=list(layer);
    let total=measure(text,size).max(1.0);
    let mut x=pos[0];
    let mut buffer=[0u8;4];
    for (i,ch) in text.chars().enumerate() {
        let s:&str=ch.encode_utf8(&mut buffer);
        let width=measure(s,size);
        let (i_f,u)=(i as f32,(x-pos[0]+width*0.5)/total);
        // 颜色
        let mut color=mix(style.color,style.color,0.0);let mut light=1.0f32;
        match style.effect {
            Effect::Gradient=>color=mix(style.color,style.accent,0.5+0.5*(time*1.6-u*5.0).sin()),
            Effect::Shimmer=>{let band=(time*0.7).rem_euclid(1.6)-0.3;let hit=(1.0-(u-band).abs()*4.0).max(0.0);color=mix(style.color,style.accent,hit);light=1.0+0.35*hit;},
            Effect::Pulse=>light=0.78+0.22*(time*2.6).sin(),
            Effect::Flicker=>{light=0.62+0.38*((time*11.0+i_f*1.7).sin()*(time*6.3+i_f*2.9).sin()).abs();color=mix(style.color,style.accent,1.0-light);},
            _=>{},
        }
        // 位置
        let (mut dx,mut dy)=(0.0,0.0);
        match style.effect {
            Effect::Wave=>dy=(time*3.2+i_f*0.7).sin()*size*0.1,
            Effect::Jitter=>{let step=(time*18.0) as u32;dx=(noise(step,i as u32*2)-0.5)*size*0.09;dy=(noise(step,i as u32*2+1)-0.5)*size*0.09;},
            _=>{},
        }
        let at=[x+dx,pos[1]+dy];
        // 外发光：辅色在四周叠一圈半透明字形，亮度随呼吸起伏
        if style.glow {
            let breath=0.55+0.45*(time*2.2+i_f*0.3).sin();
            // 两圈：外圈大而淡，内圈小而亮
            for (r,a) in [(size*0.13,0.14),(size*0.07,0.26)] {
                for k in 0..8 {
                    let angle=k as f32*std::f32::consts::FRAC_PI_4;
                    glyph(list,size,[at[0]+angle.cos()*r,at[1]+angle.sin()*r],rgba(mix(style.accent,style.accent,0.0),a*breath*alpha),s);
                }
            }
        }
        // 残影：身后两道淡色拖影，随时间伸缩
        if style.effect==Effect::Afterimage {
            let stretch=0.6+0.4*(time*5.0).sin().abs();
            for (k,a) in [(3.0,0.14),(2.0,0.24),(1.0,0.4)] {glyph(list,size,[at[0]-size*0.12*k*stretch,at[1]],rgba(mix(style.accent,style.accent,0.0),a*alpha),s);}
        }
        // 深色描边，保证在任何背景上都清楚
        let outline=rgba([0.02,0.02,0.02],0.85*alpha);
        for (ox,oy) in [(-1.5,0.0),(1.5,0.0),(0.0,-1.5),(0.0,1.5),(1.5,1.5)] {glyph(list,size,[at[0]+ox,at[1]+oy],outline,s);}
        glyph(list,size,at,rgba(scale(color,light),alpha),s);
        x+=width;
    }
    x-pos[0]
}
/// 品质前缀的特效：普通素色，稀有蓝色流光，史诗紫色呼吸发光，传说金色流光发光
fn prefix_style(prefix:&str,color:[u8;3])->Option<Style> {
    let light=color.map(|c|(u16::from(c)+(255-u16::from(c))*3/5) as u8);
    match prefix {
        "普通·"=>Some(Style{color,accent:color,effect:Effect::Plain,glow:false}),
        "稀有·"=>Some(Style{color,accent:light,effect:Effect::Shimmer,glow:false}),
        "史诗·"=>Some(Style{color,accent:light,effect:Effect::Pulse,glow:true}),
        "传说·"=>Some(Style{color,accent:[255,244,200],effect:Effect::Shimmer,glow:true}),
        _=>None,
    }
}
/// 补正属性的颜色：力气赤红、灵巧翠绿、智力蔚蓝、信仰圣金、感应绯紫
fn attribute(word:&str)->Option<[u8;3]> {
    match word {"力气"=>Some([228,108,70]),"灵巧"=>Some([104,204,146]),"智力"=>Some([98,152,242]),"信仰"=>Some([238,206,108]),"感应"=>Some([204,88,176]),_=>None}
}
/// 防具类型的样式：物理钢灰流光、魔法蓝色呼吸、锈蚀锈红波动、艾尔登金色流光发光
fn armor(word:&str)->Option<Style> {
    Some(match word {
        "物理"=>Style{color:[170,178,190],accent:[236,240,246],effect:Effect::Shimmer,glow:false},
        "魔法"=>Style{color:[112,152,238],accent:[196,216,255],effect:Effect::Pulse,glow:false},
        "锈蚀"=>Style{color:[198,120,72],accent:[238,170,110],effect:Effect::Wave,glow:false},
        "艾尔登"=>Style{color:[232,196,108],accent:[255,244,200],effect:Effect::Shimmer,glow:true},
        _=>return None,
    })
}
fn lighter(c:[u8;3])->[u8;3] {c.map(|v|(u16::from(v)+(255-u16::from(v))*3/5) as u8)}
/// 名字中间的补正组合或防具类型；认不出来时返回 false，由调用方按普通文字画
fn middle(layer:Layer,x:&mut f32,y:f32,size:f32,word:&str,time:f32,alpha:f32)->bool {
    if let Some(style)=armor(word) {*x+=draw(layer,[*x,y],size,word,&style,time,alpha);return true;}
    if word=="全属性" {
        // 五色轮转：每个字的颜色随时间在五种属性色之间流动
        let colors=["力气","灵巧","智力","信仰","感应"].map(|w|attribute(w).unwrap());
        let mut buffer=[0u8;4];
        for (i,ch) in word.chars().enumerate() {
            let phase=time*1.2+i as f32*0.9;let k=phase.floor() as usize;
            let (a,b)=(colors[k%5],colors[(k+1)%5]);
            let t=phase.fract();let mix=[0,1,2].map(|c|(f32::from(a[c])*(1.0-t)+f32::from(b[c])*t) as u8);
            *x+=draw(layer,[*x,y],size,ch.encode_utf8(&mut buffer),&Style{color:mix,accent:lighter(mix),effect:Effect::Plain,glow:true},time,alpha);
        }
        return true;
    }
    let words:Vec<&str>=word.split('/').collect();
    if words.iter().any(|w|attribute(w).is_none()) {return false;}
    let slash=Style{color:[150,142,126],accent:[150,142,126],effect:Effect::Plain,glow:false};
    for (i,w) in words.into_iter().enumerate() {
        if i>0 {*x+=draw(layer,[*x,y],size,"/",&slash,time,alpha);}
        let color=attribute(w).unwrap();
        *x+=draw(layer,[*x,y],size,w,&Style{color,accent:lighter(color),effect:Effect::Gradient,glow:false},time,alpha);
    }
    true
}
/// 物品名，如“传说·灵巧/信仰·亵渎圣剑”“史诗·物理·恶兆之子铠甲”：
/// 品质前缀按品质带特效（传说金色流光发光，史诗紫色呼吸发光，稀有蓝色流光，普通素色）；
/// 中间的补正组合每个属性各用自己的颜色（全属性五色轮转），防具类型各有样式；
/// 最后的装备名用品质色，传说品质也是金色流光发光。x 是按 align 对齐的位置（0 左对齐、0.5 居中），返回整段宽度
pub fn quality_name(layer:Layer,x:f32,y:f32,size:f32,text:&str,color:[f32;4],tier:usize,align:f32,time:f32)->f32 {
    let rgb=[0,1,2].map(|i|(color[i].clamp(0.0,1.0)*255.0) as u8);let alpha=color[3];
    let total=measure(text,size);let mut x=x-total*align;
    let legendary=Style{color:rgb,accent:[255,244,200],effect:Effect::Shimmer,glow:true};
    let name_style=if tier>=3 {legendary}else{Style{color:rgb,accent:rgb,effect:Effect::Plain,glow:false}};
    let prefix=["普通·","稀有·","史诗·","传说·"].into_iter().find(|p|text.starts_with(p));
    let Some(prefix)=prefix else {
        draw(layer,[x,y],size,text,&name_style,time,alpha);return total;
    };
    x+=draw(layer,[x,y],size,prefix,&prefix_style(prefix,rgb).unwrap_or(name_style),time,alpha);
    let rest=&text[prefix.len()..];
    if let Some((word,name))=rest.split_once('·') {
        let start=x;
        if middle(layer,&mut x,y,size,word,time,alpha) {
            x+=draw(layer,[x,y],size,"·",&Style{effect:Effect::Plain,glow:false,..name_style},time,alpha);
            draw(layer,[x,y],size,name,&name_style,time,alpha);
            return total;
        }
        x=start;
    }
    draw(layer,[x,y],size,rest,&name_style,time,alpha);
    total
}
/// 一行由多段组成的特效文字（段之间插分隔符），整行居中画在 center_x
pub fn draw_line(layer:Layer,center_x:f32,y:f32,size:f32,parts:&[(&str,Style)],separator:(&str,[u8;3]),time:f32) {
    if parts.is_empty() {return;}
    let sep_width=measure(separator.0,size);
    let total:f32=parts.iter().map(|(t,s)|measure(t,size)+if s.effect==Effect::Afterimage{size*0.4}else{0.0}).sum::<f32>()+sep_width*(parts.len()-1) as f32;
    let mut x=center_x-total*0.5;
    let plain=Style{color:separator.1,accent:separator.1,effect:Effect::Pulse,glow:false};
    for (index,(text,style)) in parts.iter().enumerate() {
        if index>0 {x+=draw(layer,[x,y],size,separator.0,&Style{effect:Effect::Shimmer,..plain},0.0,1.0);}
        // 残影拖在字的左边，给它留出空位，不压到前面的分隔符
        if style.effect==Effect::Afterimage {x+=size*0.4;}
        x+=draw(layer,[x,y],size,text,style,time,1.0);
    }
}

/// 屏幕上方词条行的颜色与动态效果：与 Boss 身上光效的颜色一致
pub fn affix_style(affix:erdtree_tower::boss_affixes::Affix)->Style {
    use erdtree_tower::boss_affixes::Affix;
    let (color,accent,effect,glow)=match affix {
        Affix::Stalwart=>([196,182,150],[244,226,178],Effect::Pulse,false),
        Affix::Bulwark=>([140,172,204],[228,240,255],Effect::Shimmer,false),
        Affix::Frenzied=>([232,72,52],[255,170,90],Effect::Jitter,true),
        Affix::Regenerating=>([112,204,124],[204,255,196],Effect::Pulse,true),
        Affix::Rot=>([206,82,62],[244,146,96],Effect::Wave,false),
        Affix::Envious=>([160,96,214],[60,24,90],Effect::Flicker,true),
        Affix::Proud=>([244,238,224],[255,214,140],Effect::Shimmer,true),
        Affix::Golden=>([242,200,92],[255,248,210],Effect::Shimmer,true),
        Affix::Reborn=>([244,150,62],[255,224,128],Effect::Gradient,true),
        Affix::Leeching=>([204,32,54],[255,96,116],Effect::Pulse,true),
        Affix::Thorned=>([172,42,42],[232,96,84],Effect::Jitter,false),
        Affix::Swift=>([92,222,204],[204,255,246],Effect::Afterimage,false),
    };
    Style{color,accent,effect,glow}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn colors_pack_as_imgui_abgr_and_noise_stays_in_range() {
        assert_eq!(rgba([1.0,0.0,0.0],1.0),0xff0000ff);
        assert_eq!(rgba([0.0,0.0,1.0],0.5)>>24,127);
        assert!((0..1000).all(|i|(0.0..=1.0).contains(&noise(i,7))));
        assert_eq!(mix([0,0,0],[255,255,255],0.5),[0.5019608,0.5019608,0.5019608]);
        assert_eq!(ranges("文乙丁丙"),vec![0x20,0x7e,0x4e01,0x4e01,0x4e19,0x4e19,0x4e59,0x4e59,0x6587,0x6587,0]);
    }
}
