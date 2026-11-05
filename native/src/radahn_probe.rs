use std::time::{Duration,Instant};
use eldenring::cs::ChrIns;

pub fn encounter()->Option<u16> {
    crate::enemies::debug_encounters().iter().position(|group|group.len()==1&&group[0].model==4730).and_then(|i|i.try_into().ok())
}
fn target_hp(hp:i32,max:i32)->Option<i32> {(hp>0&&max>0).then(||hp.min(((i64::from(max)*40/100).max(1)) as i32))}
pub struct Probe {
    started:Instant,seen:Option<Instant>,lowered:bool,last:Option<Instant>,signature:String,
    protection:(bool,bool),finished:bool,
}
impl Probe {
    pub fn new()->Self {
        Self{started:Instant::now(),seen:None,lowered:false,last:None,signature:String::new(),
            protection:crate::debug_menu::replace_cheats(true,false),finished:false}
    }
    pub fn tick(&mut self,chr:&mut ChrIns,player:Option<[f32;3]>) {
        if chr.npc_id!=4730 {return;}
        let now=Instant::now();let seen=*self.seen.get_or_insert(now);
        if !self.lowered&&now.duration_since(seen)>=Duration::from_secs(2)&&!chr.chr_flags1c5.death_flag()
            && let Some(hp)=target_hp(chr.modules.data.hp,chr.modules.data.max_hp) {
            crate::log(format!("RADAHN_TEST hp_gate entity={} before={} after={hp} max={}; natural AI only",chr.event_entity_id,chr.modules.data.hp,chr.modules.data.max_hp));
            chr.modules.data.hp=hp;self.lowered=true;
        }
        if self.finished{return;}
        if self.started.elapsed()>Duration::from_secs(240){crate::log("RADAHN_TEST trace window ended; no recovery or teleport applied");self.finished=true;return;}
        if self.last.is_some_and(|t|now.duration_since(t)<Duration::from_millis(250)){return;}
        let effects:Vec<_>=chr.special_effect.entries().map(|e|e.param_id).filter(|id|(13900..14000).contains(id)).collect();
        let time=&chr.modules.time_act;
        let animation=time.anim_queue.get(time.read_idx as usize).map(|a|a.anim_id);
        let event_steps:Vec<_>=(45029980..=45029984).map(|id|crate::native_talk::flag(id).ok()).collect();
        let sig=format!("hp={}/{} grounded={} disabled={} animation_queue_last={animation:?} effects={effects:?} meteor_steps={event_steps:?}",chr.modules.data.hp,chr.modules.data.max_hp,chr.modules.physics.standing_on_solid_ground,chr.debug_flags.character_disabled());
        if sig==self.signature&&self.last.is_some_and(|t|now.duration_since(t)<Duration::from_secs(1)){return;}
        self.signature=sig.clone();self.last=Some(now);
        let p=&chr.modules.physics.position;
        crate::log(format!("RADAHN_TEST elapsed_ms={} entity={} {sig} physics_position={:?} player_physics_position={player:?}",self.started.elapsed().as_millis(),chr.event_entity_id,[p.0,p.1,p.2]));
    }
}
impl Drop for Probe {fn drop(&mut self){crate::debug_menu::replace_cheats(self.protection.0,self.protection.1);crate::log("RADAHN_TEST stopped; prior debug protection restored");}}

#[cfg(test)]
mod tests {
    #[test]
    fn targets_base_game_radahn_only() {
        let index=super::encounter().expect("Radahn test encounter");
        let group=crate::enemies::boss_test_roster(index,false,false).unwrap();
        assert_eq!(group.len(),1);assert_eq!(group[0].model,4730);assert_eq!(group[0].act,2);
    }
    #[test]
    fn hp_gate_never_heals_or_revives() {
        assert_eq!(super::target_hp(1000,1000),Some(400));
        assert_eq!(super::target_hp(100,1000),Some(100));
        assert_eq!(super::target_hp(1,1),Some(1));
        assert_eq!(super::target_hp(0,1000),None);
        assert_eq!(super::target_hp(100,0),None);
    }
}
