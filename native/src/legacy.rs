// 职业传承（局外养成，不加数值）：每个角色存档一份 .legacy 文件，记录各职业的熟练度经验与选用的开局护符。
// 整局结束时按结算记录加经验；开局时把当前职业选用的护符代替祖灵角发给玩家。
use std::sync::Mutex;
use erdtree_tower::{journal::Record,legacy::{Gain,Legacy}};
/// 当前角色存档（运行时识别角色后设置），旅程档案的“传承”页据此读写
static OWNER:Mutex<Option<(u32,i32)>>=Mutex::new(None);
pub fn set_owner(owner:(u32,i32)) {if let Ok(mut current)=OWNER.lock(){*current=Some(owner);}}
pub fn owner()->Option<(u32,i32)> {OWNER.lock().ok().and_then(|o|*o)}
fn path(owner:(u32,i32))->std::path::PathBuf {super::hub::path(owner,"legacy")}
pub fn load(owner:(u32,i32))->Legacy {std::fs::read_to_string(path(owner)).map(|t|Legacy::decode(&t)).unwrap_or_default()}
pub fn save(owner:(u32,i32),legacy:&Legacy)->Result<(),String> {
    let path=path(owner);let tmp=path.with_extension("legacy.tmp");
    std::fs::write(&tmp,legacy.encode()).and_then(|_|std::fs::rename(&tmp,&path)).map_err(|e|format!("职业传承写入失败：{e}"))
}
/// 整局结束：给这一局的职业加熟练度经验
pub fn award(owner:(u32,i32),record:&Record)->Option<Gain> {
    let mut legacy=load(owner);
    let gain=legacy.award(record)?;
    match save(owner,&legacy) {
        Ok(())=>{super::log(format!("TOWER_LEGACY class={:?} xp=+{} level={}->{}",gain.class,gain.xp,gain.before,gain.after));Some(gain)},
        Err(error)=>{super::log(error);None},
    }
}
/// 开局：当前职业选用的开局护符（未选或未解锁时为祖灵角）
pub fn talisman(owner:(u32,i32),class:erdtree_tower::Class)->u32 {load(owner).get(class).chosen(class)}
