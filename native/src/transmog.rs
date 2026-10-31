use std::{ffi::{c_char,c_void,CString},path::PathBuf,sync::{Mutex,OnceLock,atomic::{AtomicUsize,Ordering}}};
use eldenring::cs::{EquipParamProtector,SoloParamRepository};
use fromsoftware_shared::FromStatic;
use erdtree_tower::transmog::Appearance;

#[link(name="kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(path:*const u16)->*mut c_void;
    fn GetProcAddress(module:*mut c_void,name:*const i8)->*mut c_void;
}
static SET:AtomicUsize=AtomicUsize::new(0);
static STATUS:Mutex<String>=Mutex::new(String::new());
static LOOK:Mutex<Option<((u32,i32),Appearance)>>=Mutex::new(None);
static APPLIED:Mutex<Option<Appearance>>=Mutex::new(None);
#[derive(Clone)]
pub struct Armor {pub id:i32,pub slot:usize,pub name:String,pub icon:u32}
static CATALOG:OnceLock<Vec<Armor>>=OnceLock::new();
pub fn catalog()->&'static [Armor] {CATALOG.get().map_or(&[],Vec::as_slice)}
pub fn ready()->bool {SET.load(Ordering::Acquire)!=0}
pub fn status()->String {if ready(){"外观独立于装备属性 · 同版队友可同步".into()}else{STATUS.lock().map(|s|s.clone()).unwrap_or_default()}}
pub fn initialize(module:Option<PathBuf>) {
    let result=(||->Result<(),String>{
        let root=module.and_then(|p|p.parent().map(|p|p.to_owned())).ok_or("无法定位幻化组件")?;
        let dll=root.join("tower_transmog.dll");
        use std::os::windows::ffi::OsStrExt;
        let path:Vec<u16>=dll.as_os_str().encode_wide().chain(Some(0)).collect();
        let handle=unsafe{LoadLibraryW(path.as_ptr())};if handle.is_null(){return Err(format!("幻化组件加载失败：{}",std::io::Error::last_os_error()));}
        let init=unsafe{GetProcAddress(handle,c"tower_transmog_init".as_ptr().cast())};
        let set=unsafe{GetProcAddress(handle,c"tower_transmog_set".as_ptr().cast())};
        if init.is_null()||set.is_null(){return Err("幻化组件接口版本不匹配".into());}
        let init:unsafe extern "C" fn(*const c_char)->bool=unsafe{std::mem::transmute(init)};
        let log=CString::new(super::LOG_PATH.with_file_name("tower-transmog.log").to_string_lossy().as_bytes()).map_err(|e|e.to_string())?;
        if !unsafe{init(log.as_ptr())}{return Err("幻化初始化失败，详见 tower-transmog.log".into());}
        SET.store(set as usize,Ordering::Release);Ok(())
    })();
    if let Err(error)=result{super::log(&error);if let Ok(mut s)=STATUS.lock(){*s=error;}}
    else{super::log("Integrated armor transmog engine ready");}
}
fn path(owner:(u32,i32))->PathBuf {super::hub::path(owner,"appearance")}
pub fn current()->Appearance {LOOK.lock().ok().and_then(|s|s.as_ref().map(|(_,look)|*look)).unwrap_or_default()}
fn category(id:i32)->Option<usize> {
    if let Some(slot)=erdtree_tower::transmog::BARE.iter().position(|&bare|bare==id){return Some(slot);}
    catalog().iter().find(|item|item.id==id).map(|item|item.slot)
}
pub fn choose(owner:(u32,i32),slot:usize,id:i32)->Result<(),String> {
    if !ready(){return Err("幻化组件尚未就绪".into());}
    if id>=0&&!erdtree_tower::dlc::item_allowed(0x10000000|id as u32){return Err("此防具需要启用 DLC 内容".into());}
    let mut state=LOOK.lock().map_err(|_|"幻化状态不可用")?;
    let Some((bound,look))=state.as_mut().filter(|(bound,_)|*bound==owner) else{return Err("角色尚未绑定".into());};
    let mut next=*look;next.set(slot,id,category).map_err(str::to_owned)?;
    let dest=path(*bound);let temp=dest.with_extension("appearance.tmp");
    std::fs::write(&temp,next.encode()).and_then(|_|std::fs::rename(&temp,&dest)).map_err(|e|format!("幻化保存失败：{e}"))?;
    *look=next;Ok(())
}
pub fn sync(owner:(u32,i32)) {
    let Ok(params)=(unsafe{SoloParamRepository::instance()}) else{return;};
    CATALOG.get_or_init(||erdtree_tower::rewards::CATALOG.iter().filter(|i|i.kind==erdtree_tower::rewards::ItemKind::Armor).filter_map(|item|{
        let row=params.get::<EquipParamProtector>(item.id&0x0fffffff)?;let slot=row.protector_category() as usize;
        (slot<4).then(||Armor{id:(item.id&0x0fffffff) as i32,slot,name:item.name.into(),icon:row.icon_id_m() as u32})
    }).collect());
    let Ok(mut state)=LOOK.lock() else{return;};
    if state.as_ref().is_none_or(|(old,_)|*old!=owner) {
        let mut look=std::fs::read_to_string(path(owner)).ok().and_then(|s|Appearance::decode(&s)).unwrap_or_default();
        for slot in 0..4 {let id=look.armor[slot];if id!=-1&&(category(id)!=Some(slot)||params.get::<EquipParamProtector>(id as u32).is_none()||!erdtree_tower::dlc::item_allowed(0x10000000|id as u32)){look.armor[slot]=-1;}}
        *state=Some((owner,look));if let Ok(mut last)=APPLIED.lock(){*last=None;}
    }
    let look=state.as_ref().unwrap().1;drop(state);
    let address=SET.load(Ordering::Acquire);if address==0{return;}
    let Ok(mut last)=APPLIED.lock() else{return;};if *last==Some(look){return;}
    let set:unsafe extern "C" fn(*const i32)->bool=unsafe{std::mem::transmute(address)};
    if unsafe{set(look.armor.as_ptr())}{*last=Some(look);super::log(format!("TRANSMOG applied owner={owner:?} armor={:?}",look.armor));}
}
