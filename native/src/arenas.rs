#[derive(Clone, Copy)]
pub struct Arena {
    pub block: i32,
    pub entry: u32,
    pub stone: u32,
    pub name: &'static str,
    pub center:[f32;3],
    pub yaw:f32,
    arrival:Option<[f32;3]>,
}
const LEGACY: [Arena; 3] = [
    Arena { block: 0x2d020000, entry: 45022600, stone: 45020100, name: "宁姆格福竞技场",center:[0.0;3],yaw:0.0,arrival:None },
    Arena { block: 0x2d010000, entry: 45012600, stone: 45010100, name: "盖利德竞技场",center:[0.0;3],yaw:0.0,arrival:None },
    Arena { block: 0x2d000000, entry: 45002600, stone: 45000100, name: "王城竞技场",center:[0.0;3],yaw:0.0,arrival:None },
];
static ARENAS:std::sync::LazyLock<Vec<Arena>>=std::sync::LazyLock::new(||include_str!("../../scene-assets/battlefields.txt").lines().map(|line| {
    let p:Vec<_>=line.split('|').collect();let map:Vec<u8>=p[1][1..].split('_').map(|v|v.parse().unwrap()).collect();
    let basis=p[2].parse::<u32>().unwrap();
    Arena{block:i32::from_be_bytes([map[0],map[1],map[2],map[3]]),entry:basis+2600,stone:basis+100,name:p[3],center:[p[4].parse().unwrap(),p[5].parse().unwrap(),p[6].parse().unwrap()],yaw:p[7].parse::<f32>().unwrap().to_radians(),arrival:Some(std::array::from_fn(|i|p[8+i].parse().unwrap()))}
}).collect());
impl Arena {
    pub fn combat_entry(self)->u32 {self.entry+351}
    pub fn attendant_position(self)->[f32;3] {self.arrival.unwrap_or_else(||self.position([28.35,0.35,-4.06]))}
    pub fn position(self,local:[f32;3])->[f32;3] {
        let (sin,cos)=self.yaw.sin_cos();
        [self.center[0]+local[0]*cos+local[2]*sin,self.center[1]+local[1],self.center[2]-local[0]*sin+local[2]*cos]
    }
    pub fn boss_entity(self,slot:usize)->i32 {self.stone as i32-100+850+slot as i32}
}
pub fn for_act(act: u8) -> Arena {
    LEGACY[usize::from(act.clamp(1,3)-1)]
}
impl Arena {
    /// [HansFix] 进场检查的“掉出地面”高度：竞技场原点附近为地面时是 -1.5；
    /// 其他场地按场地中心与落点高度往下 5 米（地下河、湖底书库等场地的地面本身就在 y = -100 … -1700）
    pub fn fall_floor(&self)->f32 {
        if self.center==[0.0;3]&&self.arrival.is_none() {return -1.5;}
        let ground=self.arrival.map_or(self.center[1],|a|a[1].min(self.center[1]));
        ground-5.0
    }
}
pub fn for_block(block: i32) -> Option<Arena> { ARENAS.iter().chain(LEGACY.iter()).chain(BUILT.iter().map(|(_,a)|a)).copied().find(|a| a.block == block) }

/// 主题幕的战斗场地：scene-builder arena-build（Windows）把原版地图改造成战场后写出 scene-assets/arenas-built.txt，每行
/// 键|地图|名称|中心 x y z|朝向（度）|场边引导者 x y z。改造时复用宁姆格福竞技场（m45_02）的进场设施与编号，
/// 所以入口点、召唤石等编号与第一幕竞技场相同（两张地图不会同时加载）。文件缺失或缺某块场地时退回竞技场
const SITE_BASIS:u32=45020000;
static BUILT:std::sync::LazyLock<Vec<(&'static str,Arena)>>=std::sync::LazyLock::new(|| {
    let path=crate::LOG_PATH.with_file_name("scene-assets").join("arenas-built.txt");
    let Ok(text)=std::fs::read_to_string(&path) else {return Vec::new();};
    let rows:Vec<_>=text.lines().filter(|l|!l.trim().is_empty()&&!l.starts_with('#')).filter_map(parse_built).collect();
    crate::log(format!("Built arenas: {}",rows.iter().map(|(k,_)|*k).collect::<Vec<_>>().join(",")));
    rows
});
fn parse_built(line:&str)->Option<(&'static str,Arena)> {
    let p:Vec<&str>=line.split('|').collect();
    if p.len()<10 {return None;}
    let map:Vec<u8>=p[1].strip_prefix('m')?.split('_').map(|v|v.parse().ok()).collect::<Option<_>>()?;
    if map.len()!=4 {return None;}
    let f=|i:usize|p.get(i).and_then(|v|v.parse::<f32>().ok());
    let leak=|v:&str|->&'static str {Box::leak(v.to_owned().into_boxed_str())};
    Some((leak(p[0]),Arena{block:i32::from_be_bytes([map[0],map[1],map[2],map[3]]),entry:SITE_BASIS+2600,stone:SITE_BASIS+100,name:leak(p[2]),
        center:[f(3)?,f(4)?,f(5)?],yaw:f(6)?.to_radians(),arrival:Some([f(7)?,f(8)?,f(9)?])}))
}
/// [HansFix r19] 官方当前版本所有主题临时共用的单人战斗场地
pub const TEMPORARY_SITE:&str="farum_dragonlord";
/// 调试菜单“下一战指定场地”（单人）：场地键，空为按主题
static OVERRIDE:std::sync::Mutex<Option<&'static str>>=std::sync::Mutex::new(None);
pub fn set_override(key:Option<&'static str>) {if let Ok(mut o)=OVERRIDE.lock(){*o=key;}crate::log(format!("Arena override: {key:?}"));}
pub fn override_key()->Option<&'static str> {OVERRIDE.lock().ok().and_then(|o|*o)}
/// v89 的战场（battlefields.txt，已随资源发布）：arena-sites.txt 里地图写 battlefield、锚点写战场序号的场地直接用它们。
/// 第 1 处史东薇尔庭院已被跑酷试炼改写，不再引用
static PREBUILT:std::sync::LazyLock<Vec<(&'static str,Arena)>>=std::sync::LazyLock::new(||include_str!("../../scene-assets/arena-sites.txt").lines()
    .filter_map(|l|{let p:Vec<&str>=l.split('|').collect();(p.len()>=5&&p[1]=="battlefield").then(||(p[0],ARENAS[p[2].parse::<usize>().unwrap()-1]))}).collect());
fn sites()->impl Iterator<Item=&'static (&'static str,Arena)> {PREBUILT.iter().chain(BUILT.iter())}
/// 可用的场地键与名称（调试菜单用）
pub fn built()->Vec<(&'static str,&'static str)> {sites().map(|(k,a)|(*k,a.name)).collect()}
/// 由 arena-build 生成的场地：复用宁姆格福竞技场的编号与场边服务
pub fn is_built(block:i32)->bool {BUILT.iter().any(|(_,a)|a.block==block)}
/// 这一局这一幕的战斗场地：单人按主题（或调试指定）选已生成的场地；DLC 场地要求 DLC 完整模式；联机与未生成时用本幕竞技场
pub fn for_run(run:&erdtree_tower::Run,party:bool)->Arena {
    if party {return for_act(run.act);}
    let theme=erdtree_tower::themes::for_run(run);
    // [HansFix r19] 与官方 v101 发布版一致：作者调整各主题场地期间，单人战斗暂时都用龙王圆形平台（已生成时）；调试指定仍优先
    if override_key().is_none() {
        if let Some((_,a))=sites().find(|(k,_)|*k==TEMPORARY_SITE) {return *a;}
    }
    let key=override_key().unwrap_or(theme.arena);
    if key.is_empty()||(theme.dlc&&erdtree_tower::dlc::mode()!=erdtree_tower::dlc::Mode::Full) {return for_act(run.act);}
    sites().find(|(k,_)|*k==key).map_or_else(||for_act(run.act),|(_,a)|*a)
}

#[cfg(test)]
mod tests {
    #[test]
    fn prebuilt_sites_use_the_v89_battlefields_with_matching_act_numbers() {
        let sites:Vec<_>=super::PREBUILT.iter().collect();
        assert!(sites.iter().all(|(_,a)|a.block!=0x0a000000),"Stormveil now belongs to the parkour trial");
        for theme in erdtree_tower::themes::THEMES.iter() {
            if let Some((_,arena))=sites.iter().find(|(k,_)|*k==theme.arena) {assert_eq!(arena.stone,super::for_act(theme.act).stone,"{}",theme.name);}
        }
        assert_eq!(sites.len(),2);
    }
    #[test]
    fn underground_sites_are_not_mistaken_for_falling_through_the_floor() {
        let (_,siofra)=super::parse_built("siofra_ancestor|m12_08_00_00|希芙拉河·祖灵洞窟|1546.102|-1736.05|1053.286|-12.443|1558.954|-1735.7|1054.25|1|0|1").unwrap();
        assert!(-1735.7>siofra.fall_floor()&&siofra.fall_floor()>-1745.0);
        assert_eq!(super::for_act(1).fall_floor(),-1.5);
    }
    #[test]
    fn built_site_rows_parse_and_reuse_the_first_arena_numbers() {
        let (key,arena)=super::parse_built("leyndell_morgott|m11_00_00_00|王城·恶兆王的战场|12.5|60|-300|90|20|60.5|-310").unwrap();
        assert_eq!((key,arena.block,arena.entry,arena.stone),("leyndell_morgott",0x0b000000,45022600,45020100));
        assert_eq!(arena.combat_entry(),45022951);assert_eq!(arena.attendant_position(),[20.0,60.5,-310.0]);
        assert!((arena.yaw-std::f32::consts::FRAC_PI_2).abs()<1e-5);
        assert!(super::parse_built("broken|m11_00|x").is_none());
        assert!(super::parse_built("x|m11_00_00_00|名|a|0|0|0|0|0|0").is_none());
        let run=erdtree_tower::Run::new(1,erdtree_tower::Class::Hero);
        assert_eq!(super::for_run(&run,true).block,super::for_act(1).block,"multiplayer keeps the colosseum");
    }
    #[test]
    fn combat_arrival_uses_exterior_gate_region_without_moving_services() {
        for act in 1..=3 {
            let arena=super::for_act(act);
            assert_eq!(arena.combat_entry(),arena.entry+351);
            assert_eq!(arena.attendant_position(),[28.35,0.35,-4.06]);
        }
    }
    #[test]
    fn attendants_use_verified_entry_height_instead_of_flat_arena_offsets() {
        for (index,line) in include_str!("../../scene-assets/battlefields.txt").lines().enumerate() {
            let fields:Vec<_>=line.split('|').collect();
            let expected:[f32;3]=std::array::from_fn(|i|fields[8+i].parse().unwrap());
            let arena=super::ARENAS[index];
            assert_eq!(arena.attendant_position(),expected);
            assert_ne!(arena.attendant_position(),arena.position([-16.0,0.1,10.0]));
        }
        let legacy=super::for_block(0x2d020000).unwrap();
        assert_eq!(legacy.attendant_position(),[28.35,0.35,-4.06]);
    }
    #[test]
    fn acts_have_distinct_matching_arena_resources() {
        let arenas = [super::for_act(1),super::for_act(2),super::for_act(3)];
        assert_eq!(arenas[0].block,0x2d020000);
        assert_eq!(arenas[0].entry,45022600);
        assert_eq!(arenas[0].attendant_position(),[28.35,0.35,-4.06]);
        for (i, a) in arenas.iter().enumerate() {
            assert_eq!(super::for_act(i as u8 + 1).block, a.block);
            assert_eq!(super::for_block(a.block).unwrap().stone, a.stone);
            assert_eq!(a.entry - a.stone, 2500);
            assert_eq!(a.position([0.0;3]),a.center);
            assert_eq!(a.boss_entity(0),a.entry as i32-1750);
            assert_eq!(a.block,[0x2d020000,0x2d010000,0x2d000000][i]);
            let front=a.position([0.0,0.0,10.0]);let back=a.position([0.0,0.0,-10.0]);
            assert!((front[0]+back[0]-2.0*a.center[0]).abs()<0.001);
            assert!((front[2]+back[2]-2.0*a.center[2]).abs()<0.001);
            assert!(arenas[..i].iter().all(|b| b.block != a.block));
        }
        assert!(super::for_block(0x0b0a0000).is_none());
    }
}
