use std::{ffi::c_void,sync::{Mutex,atomic::{AtomicUsize,AtomicBool,Ordering}}};
use eldenring::cs::{ChrIns,FieldInsHandle,WorldChrMan};
use hudhook::mh::{MhHook,MH_EnableHook};

#[derive(Clone,Copy,PartialEq)]
struct Binding {world:usize,address:usize,handle:FieldInsHandle,entity:u32}
static BINDING:Mutex<Option<Binding>>=Mutex::new(None);
static ORIGINAL:AtomicUsize=AtomicUsize::new(0);
static REPORTED:AtomicBool=AtomicBool::new(false);
type Lookup=unsafe extern "C" fn(*const WorldChrMan,*const u32)->*mut ChrIns;

fn eligible(entity:u32,model:i32,networked:bool)->bool {
    !networked&&model==4730&&matches!(entity,45000850..=45000852|45010850..=45010852|45020850..=45020852)
}
fn same_actor(binding:Binding,address:usize,entity:u32,model:i32,hp:i32,dead:bool,disabled:bool)->bool {
    binding.address==address&&binding.entity==entity&&model==4730&&hp>0&&!dead&&!disabled
}
pub fn sync(world:&WorldChrMan,chr:Option<&ChrIns>,networked:bool) {
    let next=chr.filter(|c|eligible(c.event_entity_id,c.npc_id,networked)&&c.modules.data.hp>0&&!c.chr_flags1c5.death_flag())
        .map(|c|Binding{world:world as *const _ as usize,address:c as *const _ as usize,handle:c.field_ins_handle,entity:c.event_entity_id});
    if let Ok(mut binding)=BINDING.lock(){if *binding!=next{*binding=next;REPORTED.store(false,Ordering::Relaxed);}}
}
unsafe extern "C" fn lookup(world:*const WorldChrMan,entity:*const u32)->*mut ChrIns {
    let original:Lookup=unsafe {std::mem::transmute(ORIGINAL.load(Ordering::Acquire))};
    let found=unsafe {original(world,entity)};
    if !found.is_null()||world.is_null()||entity.is_null(){return found;}
    let binding=BINDING.try_lock().ok().and_then(|b|*b);
    let Some(binding)=binding.filter(|b|b.world==world as usize&&b.entity==unsafe {*entity})else{return found;};
    let Some(chr)=(unsafe {&*world}).chr_ins_by_handle(&binding.handle)else{return found;};
    if !same_actor(binding,chr as *const _ as usize,chr.event_entity_id,chr.npc_id,
        chr.modules.data.hp,chr.chr_flags1c5.death_flag(),chr.debug_flags.character_disabled()){return found;}
    if !REPORTED.swap(true,Ordering::Relaxed){crate::log(format!("RADAHN_ENTITY native lookup missed entity={}; resolved live handle={}",binding.entity,binding.handle));}
    chr as *const ChrIns as *mut ChrIns
}
pub unsafe fn install()->Result<(),String> {
    if std::mem::offset_of!(WorldChrMan,chr_set_holder_count)!=0x1cc58||std::mem::offset_of!(WorldChrMan,chr_set_holders)!=0x1cc60 {
        return Err("WorldChrMan entity lookup layout mismatch".into());
    }
    // WorldChrMan entity lookup: native tree search first; only our live solo actor may fall back.
    let address=crate::items::checked_function(0x508d80,&[0x48,0x89,0x5c,0x24,0x10,0x48,0x89,0x6c,0x24,0x18,0x56,0x48,0x83,0xec,0x20])? as *mut c_void;
    let hook=unsafe {MhHook::new(address,lookup as *mut c_void)}.map_err(|e|format!("Radahn entity hook: {e:?}"))?;
    ORIGINAL.store(hook.trampoline() as usize,Ordering::Release);
    unsafe {MH_EnableHook(address)}.ok().map_err(|e|format!("Radahn entity enable: {e:?}"))?;
    crate::log("RADAHN_ENTITY native lookup fallback installed");Ok(())
}
#[cfg(test)]
mod tests {
    #[test]fn stale_reused_dead_and_disabled_actors_are_rejected(){
        let binding=super::Binding{world:1,address:2,handle:unsafe {std::mem::zeroed()},entity:45020850};
        assert!(super::same_actor(binding,2,45020850,4730,1,false,false));
        for (address,entity,model,hp,dead,disabled) in [
            (3,45020850,4730,1,false,false),(2,45020851,4730,1,false,false),
            (2,45020850,3251,1,false,false),(2,45020850,4730,0,false,false),
            (2,45020850,4730,1,true,false),(2,45020850,4730,1,false,true)] {
            assert!(!super::same_actor(binding,address,entity,model,hp,dead,disabled));
        }
    }
    #[test]fn only_solo_starscourge_dynamic_slots_are_eligible(){
        for entity in [45000850,45010851,45020852]{assert!(super::eligible(entity,4730,false));}
        for entity in [0,10000,45020849,45020853,48020298,48020299]{assert!(!super::eligible(entity,4730,false));}
        assert!(!super::eligible(45020850,4730,true));
        assert!(!super::eligible(45020850,3251,false));
    }
}
