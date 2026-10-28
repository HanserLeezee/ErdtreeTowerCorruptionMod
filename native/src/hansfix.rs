//! [HansFix] Optional community patches, switched on by marker files next to this DLL.
//! Every HansFix patch ships the same DLL plus only its own marker, so patches never depend on
//! each other and can be combined. Without any marker the DLL behaves exactly like the official build.
//!   hansfix-wander.enabled               游龙模式（改变联机协议：队伍里所有人都要装同一补丁）
//!   hansfix-better-map-merchant.enabled  N 键只读地图、联机画线不随换层清空、每人独立商店（与官方版联机兼容）
//! The map key can be rebound with `key.map=<键>` in tower-options.ini.
use std::{path::{Path,PathBuf},sync::atomic::{AtomicBool,Ordering}};

pub const WANDER_MARKER:&str="hansfix-wander.enabled";
pub const MERCHANT_MARKER:&str="hansfix-better-map-merchant.enabled";
/// Protocol offset used when the wander patch is on, so mixed parties fail the handshake loudly.
pub const WANDER_PROTOCOL_OFFSET:u32=1000;

static WANDER:AtomicBool=AtomicBool::new(false);
static MERCHANT:AtomicBool=AtomicBool::new(false);
static STALE:std::sync::OnceLock<String>=std::sync::OnceLock::new();
/// Size of scene-assets/regulation.bin of the official release this DLL was built against (v98 and v99 share it).
/// A different size means the launcher updated the mod after this patch was applied: the patched
/// DLL would no longer match the game data, so every HansFix feature stays off and a warning is shown.
pub const BASE_RELEASE:&str="v99";
/// tower-install.json 里启动器记录的模组版本号；不一致说明启动器已经装了别的版本
pub const BASE_INSTALL_VERSION:u64=99;
pub const BASE_REGULATION_LEN:u64=3438688;
/// 游龙补丁带的 regulation.bin（v98/v99 + 向下强化与扭曲词条效果）
pub const HANSFIX_REGULATION_LEN:u64=3388928;
static MAP_KEY:std::sync::OnceLock<&'static str>=std::sync::OnceLock::new();

pub fn wander()->bool {WANDER.load(Ordering::Relaxed)}
pub fn merchant()->bool {MERCHANT.load(Ordering::Relaxed)}

fn map_key_from(text:&str)->Option<&'static str> {
    text.lines().filter_map(|l|l.trim().trim_start_matches('\u{feff}').split_once('='))
        .filter(|(k,_)|k.trim().eq_ignore_ascii_case("key.map"))
        .filter_map(|(_,v)|erdtree_tower::options::KEYS.iter().copied().find(|k|k.eq_ignore_ascii_case(v.trim()))).last()
}
pub fn map_label()->&'static str {MAP_KEY.get().copied().unwrap_or("N")}
pub fn map_key()->hudhook::imgui::Key {crate::options::key_named(map_label()).unwrap_or(hudhook::imgui::Key::N)}

pub fn set(wander_on:bool,merchant_on:bool) {
    WANDER.store(wander_on,Ordering::Relaxed);MERCHANT.store(merchant_on,Ordering::Relaxed);
    erdtree_tower::party_handshake::set_extension(if wander_on {WANDER_PROTOCOL_OFFSET} else {0});
}

/// Warning text when this patched DLL sits on top of a different official release.
pub fn stale()->Option<&'static str> {STALE.get().map(String::as_str)}
pub fn stale_in(dir:&Path)->Option<String> {
    if let Some(len)=std::fs::metadata(dir.join("scene-assets").join("regulation.bin")).ok().map(|m|m.len()).filter(|&len|len!=BASE_REGULATION_LEN&&len!=HANSFIX_REGULATION_LEN) {
        return Some(format!("regulation_len={len}"));
    }
    let version=std::fs::read_to_string(dir.join("tower-install.json")).ok().and_then(|t|install_version(&t));
    version.filter(|&v|v!=BASE_INSTALL_VERSION).map(|v|format!("install_version={v}"))
}
/// 只取顶层的 "version": 数字（文件很大，不必整份解析）
fn install_version(text:&str)->Option<u64> {
    let rest=&text[text.find("\"version\"")?+9..];
    let digits:String=rest.trim_start_matches(|c:char|c==':'||c.is_whitespace()).chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}
pub fn detect(dir:&Path)->(bool,bool) {(dir.join(WANDER_MARKER).is_file(),dir.join(MERCHANT_MARKER).is_file())}

/// DllMain: read markers next to the DLL. Must run before anything talks to the party.
pub fn init(dll:Option<PathBuf>) {
    let dir=dll.as_deref().and_then(Path::parent).map(PathBuf::from);
    let (mut wander_on,mut merchant_on)=dir.as_deref().map(detect).unwrap_or_default();
    if let Some(reason)=dir.as_deref().and_then(stale_in) {
        crate::log(format!("HANSFIX_STALE base={BASE_RELEASE} {reason}"));
        let _=STALE.set(format!("HansFix 补丁是为爬塔 {BASE_RELEASE} 制作的，与当前模组版本不匹配：请在启动器「设置 → 更新 → 校验并修复文件」恢复官方文件，再安装对应新版本的补丁。"));
        wander_on=false;merchant_on=false;
    }
    set(wander_on,merchant_on);
    if let Some(key)=dir.as_deref().and_then(|d|std::fs::read_to_string(d.join(erdtree_tower::options::FILE)).ok()).and_then(|t|map_key_from(&t)) {let _=MAP_KEY.set(key);}
    crate::log(format!("HANSFIX wander={wander_on} better_map_merchant={merchant_on} map_key={} protocol={} link={}",map_label(),
        erdtree_tower::party_handshake::protocol(),erdtree_tower::party_link::wire_version()));
}

/// [HansFix] 游龙·向下：红色战斗关卡里给每个刷出的敌人施加一次强化（血量 / 攻击倍率）；联机时只由房主施加
#[derive(Default)]
pub struct Descent {key:Option<(u64,u8,usize)>,applied:Vec<eldenring::cs::FieldInsHandle>,missing:bool}
impl Descent {
    pub fn tick(&mut self,world:&mut eldenring::cs::WorldChrMan,key:(u64,u8,usize),level:Option<u32>,handles:&[eldenring::cs::FieldInsHandle],authority:bool) {
        use eldenring::cs::{ChrInsExt,SoloParamRepository,SpEffectParam};
        use fromsoftware_shared::FromStatic;
        if self.key!=Some(key){*self=Self{key:Some(key),..Default::default()};}
        let Some(level)=level.filter(|_|authority&&wander()) else{return};
        let effect=erdtree_tower::descent::enemy_effect(level);
        if !unsafe {SoloParamRepository::instance()}.is_ok_and(|p|p.get::<SpEffectParam>(effect).is_some()) {
            if !self.missing{crate::log(format!("HANSFIX_DESCENT missing effect {effect}; reinstall the wander patch regulation.bin"));self.missing=true;}
            return;
        }
        for handle in handles {
            if self.applied.contains(handle){continue;}
            let Some(chr)=world.chr_ins_by_handle_mut(handle) else{continue};
            if chr.modules.data.hp<=0||chr.chr_flags1c5.death_flag(){continue;}
            chr.apply_speffect(effect as i32,false);self.applied.push(*handle);
            if self.applied.len()==1 {crate::log(format!("HANSFIX_DESCENT level={level} effect={effect} hp_x={:.3} attack_x={:.3}",erdtree_tower::descent::hp_rate(level),erdtree_tower::descent::attack_rate(level)));}
        }
    }
}

/// [HansFix] 扭曲装备：把这件装备（武器含各质变行）常驻效果里被扭曲的词条换成 ×2 / ×-1 的版本。
/// 参数只在本机内存里改；旅程变化时先全部还原再按当前旅程重新套用，回到大赐福或换局即恢复原样。
#[derive(Default)]
struct Curses {signature:Option<u64>,original:std::collections::HashMap<(bool,u32),[i32;3]>,missing:bool}
static CURSES:std::sync::Mutex<Option<Curses>>=std::sync::Mutex::new(None);
fn signature(run:Option<&erdtree_tower::Run>)->u64 {
    use std::hash::{Hash,Hasher};
    let mut h=std::collections::hash_map::DefaultHasher::new();
    run.map(|r|(r.seed,&r.cursed)).filter(|_|wander()).map(|(seed,c)|(seed,c.iter().map(|c|(c.item,c.swaps.clone())).collect::<Vec<_>>())).hash(&mut h);h.finish()
}
pub fn curses_tick(run:Option<&erdtree_tower::Run>) {
    use eldenring::cs::{EquipParamProtector,EquipParamWeapon,SoloParamRepository,SpEffectParam};
    use fromsoftware_shared::FromStatic;
    let Ok(mut guard)=CURSES.try_lock() else{return};
    let state=guard.get_or_insert_with(Curses::default);
    let sig=signature(run);
    if state.signature==Some(sig) {return;}
    let Ok(params)=(unsafe {SoloParamRepository::instance_mut()}) else{return};
    // 还原
    for (&(weapon,row),values) in &state.original {
        if weapon {if let Some(p)=params.get_mut::<EquipParamWeapon>(row){p.set_resident_sp_effect_id(values[0]);p.set_resident_sp_effect_id1(values[1]);p.set_resident_sp_effect_id2(values[2]);}}
        else if let Some(p)=params.get_mut::<EquipParamProtector>(row){p.set_resident_sp_effect_id(values[0]);p.set_resident_sp_effect_id2(values[1]);p.set_resident_sp_effect_id3(values[2]);}
    }
    state.original.clear();state.signature=Some(sig);
    crate::hansfix_text::restore();
    let Some(run)=run.filter(|_|wander()) else{return};
    let (mut count,mut texts)=(0,0);
    for cursed in &run.cursed {
        if cursed.swaps.iter().any(|&(_,to)|params.get::<SpEffectParam>(to).is_none()) {
            if !state.missing {crate::log(format!("HANSFIX_CURSE missing effects for {}; reinstall the wander patch regulation.bin",cursed.item));state.missing=true;}
            continue;
        }
        let swap=|value:i32|cursed.swaps.iter().find(|&&(from,_)|from as i32==value).map_or(value,|&(_,to)|to as i32);
        let weapon=cursed.item>>28==0;let first=cursed.item&0x0fff_ffff;
        // 武器：同一件词条武器的各质变行都在 id..id+9999；防具只有一行
        for row in if weapon {first..first+10000} else {first..first+1} {
            if weapon {
                let Some(p)=params.get_mut::<EquipParamWeapon>(row) else{continue};
                let values=[p.resident_sp_effect_id(),p.resident_sp_effect_id1(),p.resident_sp_effect_id2()];
                state.original.insert((true,row),values);
                p.set_resident_sp_effect_id(swap(values[0]));p.set_resident_sp_effect_id1(swap(values[1]));p.set_resident_sp_effect_id2(swap(values[2]));
            } else {
                let Some(p)=params.get_mut::<EquipParamProtector>(row) else{continue};
                let values=[p.resident_sp_effect_id(),p.resident_sp_effect_id2(),p.resident_sp_effect_id3()];
                state.original.insert((false,row),values);
                p.set_resident_sp_effect_id(swap(values[0]));p.set_resident_sp_effect_id2(swap(values[1]));p.set_resident_sp_effect_id3(swap(values[2]));
            }
            count+=1;
        }
        texts+=crate::hansfix_text::apply(cursed);
    }
    crate::log(format!("HANSFIX_CURSE items={} rows={count} texts={texts}",run.cursed.len()));
}

#[cfg(test)]
mod tests {
    #[test]
    fn markers_switch_features_independently_and_only_wander_changes_the_protocol() {
        let dir=std::env::temp_dir().join(format!("hansfix-test-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&dir);std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(super::detect(&dir),(false,false));
        std::fs::write(dir.join(super::MERCHANT_MARKER),"").unwrap();
        assert_eq!(super::detect(&dir),(false,true));
        super::set(false,true);assert_eq!(erdtree_tower::party_handshake::protocol(),erdtree_tower::party_handshake::VERSION);
        std::fs::write(dir.join(super::WANDER_MARKER),"").unwrap();
        assert_eq!(super::detect(&dir),(true,true));
        super::set(true,false);assert_eq!(erdtree_tower::party_handshake::protocol(),erdtree_tower::party_handshake::VERSION+1000);
        assert_eq!(erdtree_tower::party_link::wire_version(),erdtree_tower::party_link::VERSION+1000);
        super::set(false,false);assert_eq!(erdtree_tower::party_handshake::protocol(),erdtree_tower::party_handshake::VERSION);
        assert_eq!(super::stale_in(&dir),None,"no scene-assets: nothing to compare");
        std::fs::create_dir_all(dir.join("scene-assets")).unwrap();
        std::fs::write(dir.join("scene-assets/regulation.bin"),[0u8;16]).unwrap();
        assert_eq!(super::stale_in(&dir).as_deref(),Some("regulation_len=16"));
        std::fs::remove_file(dir.join("scene-assets/regulation.bin")).unwrap();
        std::fs::write(dir.join("tower-install.json"),r#"{"version":99,"displayVersion":"1.1.1","files":[{"version":3}]}"#).unwrap();
        assert_eq!(super::stale_in(&dir),None);
        std::fs::write(dir.join("tower-install.json"),r#"{"version": 100,"files":[]}"#).unwrap();
        assert_eq!(super::stale_in(&dir).as_deref(),Some("install_version=100"));
        let _=std::fs::remove_dir_all(&dir);
        assert_eq!(super::map_key_from("key.map = m\nkey.build=C"),Some("M"));
        assert_eq!(super::map_key_from("key.map=bogus"),None);
    }
}
