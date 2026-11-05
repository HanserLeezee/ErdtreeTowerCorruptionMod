use serde::{Deserialize,Serialize};
use crate::RuleResult;

#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Weapon {pub id:u32,pub gem:i32}
impl Weapon {pub fn valid(self)->bool {self.id>=100000&&self.id>>28==0&&self.gem>=-1}}

#[derive(Clone,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyState {
    #[serde(default="legacy_accepted")] pub accepted:bool,
    #[serde(default)] pub lowered:bool,
    #[serde(default)] pub opened:bool,
    pub receipt:Option<Receipt>,pub completed:bool,
}
fn legacy_accepted()->bool {true}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {pub weapon:Weapon,pub target:u64}
impl CopyState {
    pub fn accepted()->Self {Self{accepted:true,..Default::default()}}
    pub fn observe_flow(&mut self,lowered:bool,opened:bool)->bool {
        if !self.accepted{return false;}
        let before=(self.lowered,self.opened);
        self.lowered|=lowered;self.opened|=self.lowered&&opened;
        before!=(self.lowered,self.opened)
    }
    pub fn begin(&mut self,weapon:Weapon,owned:u64)->RuleResult<()> {
        if !self.accepted||self.completed||self.receipt.is_some(){return Err("尚未前往灵庙，或本次复制已经使用".into());}
        if !self.lowered||!self.opened{return Err("请先清除灵庙脚上的骷髅，等灵庙跪下后打开大门".into());}
        if !weapon.valid()||owned==0{return Err("请选择背包中持有的武器".into());}
        let target=owned.checked_add(1).ok_or("武器数量超限")?;
        self.receipt=Some(Receipt{weapon,target});Ok(())
    }
    pub fn confirm(&mut self,owned:u64)->bool {
        if self.receipt.as_ref().is_none_or(|r|owned<r.target){return false;}
        self.receipt=None;self.completed=true;true
    }
    pub fn valid(&self)->bool {
        (!self.opened||self.lowered)&&(self.accepted||(!self.lowered&&!self.opened&&!self.completed&&self.receipt.is_none()))&&!(self.completed&&self.receipt.is_some())&&self.receipt.as_ref().is_none_or(|r|r.weapon.valid()&&r.target>=2)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ready()->CopyState {let mut state=CopyState::accepted();state.observe_flow(true,true);state}
    #[test]fn copying_requires_descent_and_an_open_door_and_progress_survives_reload(){
        let weapon=Weapon{id:100000,gem:-1};let mut state=CopyState::accepted();
        assert!(state.begin(weapon,1).is_err());
        assert!(!state.observe_flow(false,true));assert!(!state.opened);
        assert!(state.observe_flow(true,false));assert!(state.begin(weapon,1).is_err());
        assert!(state.observe_flow(true,true));assert!(!state.observe_flow(false,false));
        let mut loaded:CopyState=serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        assert!(loaded.valid());loaded.begin(weapon,1).unwrap();assert!(loaded.confirm(2));assert!(loaded.begin(weapon,2).is_err());
        let mut offer=CopyState::default();assert!(!offer.observe_flow(true,true));assert!(offer.valid());
    }
    fn offered_run()->crate::Run {
        let (mut run,node)=(0..500).find_map(|seed|{
            let run=crate::Run::new(seed,crate::Class::Samurai);
            let id=(0..run.nodes.len()).find(|&id|run.nodes[id].kind==crate::NodeKind::Event&&run.event_draw(id)==crate::events::Draw::Mausoleum)?;Some((run,id))
        }).unwrap();run.debug_jump(node).unwrap();run.enter(node).unwrap();run
    }
    #[test]fn offer_skip_awards_800_once_and_cannot_also_copy(){
        let mut run=offered_run();run.gold=1200;
        assert!(run.regional_event.mausoleum.as_mut().unwrap().begin(Weapon{id:100000,gem:-1},1).is_err());
        let before=run.clone();assert!(run.choose_trial(crate::regional_events::Trial::Descent).is_err());assert_eq!(run,before);
        run.choose_trial(crate::regional_events::Trial::Skip).unwrap();assert_eq!(run.gold,2000);
        assert!(run.regional_event.mausoleum.is_none());assert!(run.choose_trial(crate::regional_events::Trial::Skip).is_err());
        run.leave_regional_event().unwrap();assert_eq!(run.phase,crate::Phase::Map);
    }
    #[test]fn old_unused_room_returns_to_offer_but_used_copies_remain_used(){
        for completed in [false,true] {
            let mut run=offered_run();run.choose_trial(crate::regional_events::Trial::Mausoleum).unwrap();
            run.regional_event.mausoleum.as_mut().unwrap().completed=completed;
            let mut json=serde_json::to_value(&run).unwrap();json["regional_event"]["mausoleum"].as_object_mut().unwrap().remove("accepted");
            let loaded=crate::Run::decode(&json.to_string()).unwrap();
            assert_eq!(loaded.regional_event.trial,!completed);assert_eq!(loaded.regional_event.mausoleum.unwrap().accepted,completed);
        }
    }
    #[test]fn skip_overflow_does_not_consume_the_offer(){
        let mut run=offered_run();run.gold=u32::MAX;let before=run.clone();assert!(run.choose_trial(crate::regional_events::Trial::Skip).is_err());assert_eq!(run,before);
    }
    #[test]fn event_enters_saves_and_prevents_leaving_before_receipt_is_confirmed(){
        let (mut run,node)=(0..500).find_map(|seed|{
            let run=crate::Run::new(seed,crate::Class::Samurai);
            let node=(0..run.nodes.len()).find(|&id|run.nodes[id].kind==crate::NodeKind::Event&&run.event_draw(id)==crate::events::Draw::Mausoleum)?;Some((run,node))
        }).unwrap();
        let mut old=run.clone();old.event_rules=3;
        assert!((0..old.nodes.len()).all(|id|old.event_draw(id)!=crate::events::Draw::Mausoleum));
        run.debug_jump(node).unwrap();run.enter(node).unwrap();
        run.choose_trial(crate::regional_events::Trial::Mausoleum).unwrap();
        run.regional_event.mausoleum.as_mut().unwrap().observe_flow(true,true);
        run.regional_event.mausoleum.as_mut().unwrap().begin(Weapon{id:160020623,gem:12200},1).unwrap();
        let mut loaded=crate::Run::decode(&run.encode().unwrap()).unwrap();assert!(loaded.leave_regional_event().is_err());
        loaded.regional_event.mausoleum.as_mut().unwrap().confirm(2);
        assert!(crate::Run::decode(&loaded.encode().unwrap()).is_ok());
        loaded.leave_regional_event().unwrap();assert_eq!(loaded.phase,crate::Phase::Map);
    }
    #[test]fn preserves_exact_weapon_and_ash_and_only_confirms_after_inventory_increases(){
        let weapon=Weapon{id:160020623,gem:12200};let mut state=ready();
        state.begin(weapon,2).unwrap();assert_eq!(state.receipt.as_ref().unwrap(),&Receipt{weapon,target:3});
        assert!(!state.confirm(2));assert!(state.begin(weapon,2).is_err());
        assert!(state.confirm(3));assert!(state.completed);assert!(state.begin(weapon,3).is_err());
    }
    #[test]fn resume_retains_receipt_and_does_not_create_a_new_target(){
        let mut state=ready();state.begin(Weapon{id:100025,gem:-1},1).unwrap();
        let mut loaded:CopyState=serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
        assert_eq!(loaded,state);assert!(loaded.valid());assert!(loaded.confirm(2));assert!(loaded.valid());
    }
    #[test]fn rejects_nonweapons_missing_inventory_and_overflow_without_consuming_use(){
        for (weapon,owned) in [(Weapon{id:0x40000001,gem:-1},1),(Weapon{id:100000,gem:-2},1),(Weapon{id:100000,gem:0},0),(Weapon{id:100000,gem:-1},u64::MAX)] {
            let mut state=ready();let before=state.clone();assert!(state.begin(weapon,owned).is_err());assert_eq!(state,before);
        }
    }
    #[test]fn players_have_independent_uses(){
        let mut host=ready();let guest=CopyState::default();host.begin(Weapon{id:100000,gem:-1},1).unwrap();host.confirm(2);
        assert!(host.completed);assert_eq!(guest,CopyState::default());
    }
}
