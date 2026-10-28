//! 主题幕：每幕有三个可能的主题（地区），每局按这一幕的种子选一个（读档不变；联机时全队的幕种子一致，主题也一致）。
//! 主题只改“场景”——开幕标题卡、路线图配色、战斗场地与时间，不改敌人、奖励与数值。
//! 每幕还有一个 DLC 主题，只在 DLC 模式为“完整”时参与抽选（联机的无缝入口强制完整模式，全队一致）。
//! 战斗场地：Theme::arena 是 scene-assets/arena-sites.txt 的场地键；场地由 scene-builder arena-build 在 Windows 上生成后
//! 写进 scene-assets/arenas-built.txt，没生成的场地、联机（预置敌人只在三座竞技场里）都退回本幕的竞技场。
pub struct Theme {
    pub id:u8,
    pub act:u8,
    /// 主题名与地区（标题卡与路线图顶部）
    pub name:&'static str,
    pub region:&'static str,
    pub tagline:&'static str,
    /// 插画：scene-assets/theme-art/<art>.rgba（800 × 450），源图在 art-source/theme
    pub art:&'static str,
    /// 路线图配色：纸色（乘到纸张贴图上）、墨色、强调色
    pub paper:u32,pub ink:u32,pub accent:u32,
    /// 竞技场的时间（小时）
    pub hour:u8,
    /// 战斗场地（arena-sites.txt 的键，空表示用本幕竞技场）与是否需要 DLC
    pub arena:&'static str,
    pub dlc:bool,
}
#[allow(clippy::too_many_arguments)]
const fn t(id:u8,act:u8,name:&'static str,region:&'static str,tagline:&'static str,art:&'static str,paper:u32,ink:u32,accent:u32,hour:u8,arena:&'static str,dlc:bool)->Theme {
    Theme{id,act,name,region,tagline,art,paper,ink,accent,hour,arena,dlc}
}
/// 编号只能追加；同一幕的三个主题连续排列
pub const THEMES:[Theme;12]=[
    t(1,1,"风暴之门","宁姆格福 · 史东薇尔","暴风雨笼罩着破碎的城门","loading1",0xE8DCC0,0x403626,0xB0822E,18,"anticipation_scion",false),
    t(2,1,"啜泣半岛","啜泣半岛 · 摩恩城","雨从未停过，城堡还在燃烧","weeping",0xD4DCD0,0x2E3A36,0x6E8C78,16,"",false),
    t(3,1,"地下河","希芙拉河 · 安瑟尔河","伪造的星空下，祖灵仍在徘徊","river",0xCCD0E6,0x262A44,0x6A78C8,0,"siofra_ancestor",false),
    t(4,2,"月下学院","利耶尼亚 · 雷亚卢卡利亚","满月之下，辉石的光从未熄灭","loading2",0xD0D8EC,0x23283E,0x5C8CD8,0,"raya_library",false),
    t(5,2,"腐败之地","盖利德","猩红的天空下，大地在腐烂","caelid",0xEAD0C4,0x4A1E18,0xC0402C,17,"",false),
    t(6,2,"永恒之城","诺克伦 · 诺克史黛拉","坠落的星光照着沉睡的城","eternal",0xD6D8E4,0x22263A,0x9CA6D0,22,"nokstella_hall",false),
    t(7,3,"黄金王城","亚坛高原 · 王城罗德尔","黄金树的光，照不到它的根","loading3",0xF0E2BC,0x46361A,0xD0A040,12,"leyndell_tree",false),
    t(8,3,"巨人山顶","巨人山顶","风雪的尽头，火焰仍在燃烧","mountaintops",0xE2E8EE,0x2A3440,0xD07A3A,9,"",false),
    t(9,3,"圣树与龙之王朝","米凯拉圣树 · 崩塌的法姆亚兹拉","在云端之上，神明也会腐朽","haligtree",0xF2E6D2,0x483A2A,0xC89A6A,7,"farum_maliketh",false),
    // DLC 主题（幽影地）：只在 DLC 完整模式下参与抽选
    t(10,1,"幽影之塔","幽影地 · 贝鲁拉特高塔聚落","塔顶的神兽，仍在为谁起舞","shadowtower",0xE6DCCC,0x3A2E26,0xB8603A,15,"belurat_lion",true),
    t(11,2,"石棺深渊","幽影地 · 石棺巨坑","巨坑之底，腐败的骑士在等待","fissure",0xD8D0DC,0x2C2434,0x8C6AB0,1,"fissure_knight",true),
    t(12,3,"神门之前","幽影地 · 恩尼尔伊利姆","花海尽头，神门已经打开","divinegate",0xF2E8D8,0x483826,0xE0A850,10,"enir_ilim_gate",true),
];
/// 这一幕的主题（按这一幕的种子从本幕的主题里选；DLC 主题只在 DLC 完整模式下参与）
pub fn for_act(seed:u64,act:u8)->&'static Theme {for_act_with(seed,act,crate::dlc::mode()==crate::dlc::Mode::Full)}
pub fn for_act_with(seed:u64,act:u8,dlc:bool)->&'static Theme {
    let options:Vec<&Theme>=THEMES.iter().filter(|t|t.act==act.clamp(1,3)&&(dlc||!t.dlc)).collect();
    options[(crate::mix(seed^0x7468656d65^u64::from(act)) % options.len() as u64) as usize]
}
pub fn for_run(run:&crate::Run)->&'static Theme {for_act(run.seed,run.act)}
pub fn lookup(id:u8)->Option<&'static Theme> {THEMES.iter().find(|t|t.id==id)}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_act_has_three_base_themes_and_one_dlc_theme() {
        for act in 1..=3u8 {
            let themes:Vec<_>=THEMES.iter().filter(|t|t.act==act).collect();
            assert_eq!((themes.iter().filter(|t|!t.dlc).count(),themes.iter().filter(|t|t.dlc).count()),(3,1));
            assert!(themes.iter().all(|t|t.hour<24));
            assert!(themes.iter().filter(|t|t.dlc).all(|t|!t.arena.is_empty()));
        }
        let sites=include_str!("../scene-assets/arena-sites.txt");
        for theme in THEMES.iter().filter(|t|!t.arena.is_empty()) {
            assert!(sites.lines().any(|l|l.split('|').next()==Some(theme.arena)),"{} has no arena site",theme.name);
        }
        for (i,a) in THEMES.iter().enumerate() {assert!(THEMES[..i].iter().all(|b|b.id!=a.id&&b.name!=a.name&&b.art!=a.art));}
    }
    #[test]
    fn the_choice_follows_the_act_seed_and_covers_all_themes() {
        let run=crate::Run::new(42,crate::Class::Hero);
        assert_eq!(for_run(&run).act,1);assert_eq!(for_run(&run).id,for_act(run.seed,1).id);
        for act in 1..=3u8 {
            let base:std::collections::BTreeSet<_>=(0..300u64).map(|s|for_act_with(s,act,false).id).collect();
            assert_eq!(base.len(),3,"act {act} reaches every base theme");
            assert!((0..300u64).all(|s|!for_act_with(s,act,false).dlc&&for_act_with(s,act,true).act==act));
            let full:std::collections::BTreeSet<_>=(0..400u64).map(|s|for_act_with(s,act,true).id).collect();
            assert_eq!(full.len(),4,"with the DLC the act {act} theme is also reachable");
        }
    }
}
