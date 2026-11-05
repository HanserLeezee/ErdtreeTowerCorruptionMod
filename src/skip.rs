use crate::{Run,Phase,RuleResult};
use serde::{Serialize,Deserialize};

#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct Target {pub seed:u64,pub act:u8,pub node:usize,pub rewards:bool}
impl Target {
    pub fn new(run:&Run,travelling:Option<usize>)->RuleResult<Self> {
        let node=match run.phase {Phase::Combat{id}|Phase::Node{id}|Phase::Reward{id,..}=>id,
            Phase::Map=>travelling.filter(|id|run.available().contains(id)).ok_or("当前没有正在进入或进行的节点，请先选路")?,
            _=>return Err("当前旅程已结束".into())};
        let kind=run.nodes.get(node).ok_or("无效节点")?.kind;
        Ok(Self{seed:run.seed,act:run.act,node,rewards:kind.combat()&&!matches!(run.phase,Phase::Reward{..})})
    }
}
impl Run {
    pub fn skip_current(&mut self,target:Target)->RuleResult<bool> {
        if self.seed!=target.seed||self.act!=target.act{return Err("跳过请求不属于当前旅程".into());}
        if self.visited.contains(&target.node){return Ok(false);}
        if matches!(self.phase,Phase::Reward{id,..} if id==target.node)&&target.rewards{return Ok(false);}
        if self.regional_event.receipt.is_some()||self.regional_event.mausoleum.as_ref().is_some_and(|m|m.receipt.is_some()) {
            return Err("等待物品发放完成后跳过".into());
        }
        let mut next=self.clone();
        if next.phase==Phase::Map {next.enter(target.node)?;}
        let id=match next.phase {Phase::Node{id}|Phase::Combat{id}|Phase::Reward{id,..}=>id,_=>return Err("当前不能跳过".into())};
        if id!=target.node||target.rewards&&!next.nodes[id].kind.combat(){return Err("跳过节点与当前房间不符".into());}
        if target.rewards {next.win_combat()?;}else{next.finish(id);}
        next.validate()?;*self=next;Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skip_before_spawn_then_skip_reward_without_repeated_rewards() {
        let mut run=Run::new(42,crate::Class::Samurai);let node=run.available()[0];
        let request=Target::new(&run,Some(node)).unwrap();
        assert!(run.skip_current(request).unwrap());assert!(matches!(run.phase,Phase::Reward{..}));
        let snapshot=run.clone();assert!(!run.skip_current(request).unwrap());assert_eq!(run,snapshot);
        let next=Target::new(&run,None).unwrap();assert!(!next.rewards);run.skip_current(next).unwrap();
        assert_eq!(run.phase,Phase::Map);assert_eq!(run.visited,vec![node]);assert!(!run.skip_current(next).unwrap());
    }
    #[test]
    fn all_room_types_and_acts_skip_without_free_service_rewards() {
        let mut kinds=std::collections::HashSet::new();
        for seed in 0..30 {
            let mut run=Run::new(seed,crate::Class::Samurai);
            for _ in 0..100 {
                if run.phase==Phase::Won{break;}
                if run.phase==Phase::Map {let id=run.available()[0];run.enter(id).unwrap();}
                let t=Target::new(&run,None).unwrap();kinds.insert(format!("{:?}",run.nodes[t.node].kind));
                let gold=run.gold;let inventory=run.claimed_items.clone();
                run.skip_current(t).unwrap();assert_eq!(run.gold,gold);assert_eq!(run.claimed_items,inventory);
                Run::decode(&run.encode().unwrap()).unwrap();
            }
            assert_eq!(run.phase,Phase::Won);
        }
        assert_eq!(kinds.len(),7);
    }
    #[test]
    fn stale_and_unselected_targets_do_not_change_progress() {
        let mut run=Run::new(42,crate::Class::Samurai);let before=run.clone();
        assert!(Target::new(&run,None).is_err());
        let mut target=Target::new(&run,Some(run.available()[0])).unwrap();target.seed+=1;
        assert!(run.skip_current(target).is_err());assert_eq!(run,before);
    }
}
