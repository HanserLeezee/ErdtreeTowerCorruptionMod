use eldenring::cs::{ChrInsExt,FieldInsHandle,SoloParamRepository,SpEffectParam,WorldChrMan};
use fromsoftware_shared::FromStatic;
pub const GROWTH_FIRST:i32=9640000;
fn ready(step:u8)->bool {unsafe {SoloParamRepository::instance()}.is_ok_and(|p|p.get::<SpEffectParam>((GROWTH_FIRST+i32::from(step)) as u32).is_some())}
/// 新旅程按本幕进度给每个刷出的敌人施加一次递增效果（生命与攻击）。
/// 联机时房主与成员都在本机给同步过来的敌人挂上同一个效果：这个效果不随网络同步，只有房主挂时，
/// 成员那边同一只敌人的最大生命更低，成员一刀把本机副本打到 0、房主那边还活着，副本卡在原地不倒、成员也拿不到卢恩。
/// 已经挂着的不重复挂
#[derive(Default)]
pub struct Growth {key:Option<(u64,u8,usize)>,applied:Vec<FieldInsHandle>,missing:bool}
impl Growth {
    pub fn tick(&mut self,world:&mut WorldChrMan,key:(u64,u8,usize),step:u8,handles:&[FieldInsHandle]) {
        if self.key!=Some(key){*self=Self{key:Some(key),..Default::default()};}
        if step==0{return;}
        if !ready(step){if !self.missing{super::log(format!("TOWER_GROWTH missing effect {}",GROWTH_FIRST+i32::from(step)));self.missing=true;}return;}
        for handle in handles {
            if self.applied.contains(handle){continue;}
            let Some(chr)=world.chr_ins_by_handle_mut(handle) else{continue};
            if chr.modules.data.hp<=0||chr.chr_flags1c5.death_flag(){continue;}
            let id=GROWTH_FIRST+i32::from(step);
            if !chr.special_effect.entries().any(|e|e.param_id==id) {chr.apply_speffect(id,false);}
            self.applied.push(*handle);
        }
    }
}
