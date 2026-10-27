use crate::RuleResult;
use serde::{Deserialize,Serialize};
use std::collections::VecDeque;

#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub enum Kind { Guards(u8), Scarabs(u8) }
impl Kind {
    pub fn valid(self)->bool {match self {Self::Guards(slot)=>slot<3,Self::Scarabs(attempt)=>(1..=3).contains(&attempt)}}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct Job {pub epoch:u64,pub wave:u8,pub player:u64,pub kind:Kind}
pub struct Queue {
    epoch:u64,members:Vec<u64>,seen:Vec<Job>,waiting:VecDeque<Job>,active:Option<Job>,completed:Option<Job>,next_wave:u8,
    // [fix] One shared guard fight clears the room for the whole party. A second guard wave would
    // reuse the same pre-placed map entities, whose defeated bodies never unload, so it could never
    // finish binding and left every player locked in the event room.
    guards_cleared:bool,
}
impl Queue {
    pub fn new(epoch:u64,members:Vec<u64>)->RuleResult<Self> {
        if members.is_empty()||members.len()>crate::party_roster::MAX_PLAYERS||members.iter().enumerate().any(|(i,id)|*id==0||members[..i].contains(id)) {
            return Err("事件队伍名单无效".into());
        }
        Ok(Self{epoch,members,seen:vec![],waiting:VecDeque::new(),active:None,completed:None,next_wave:0,guards_cleared:false})
    }
    pub fn request(&mut self,epoch:u64,player:u64,kind:Kind)->RuleResult<bool> {
        if epoch!=self.epoch||!self.members.contains(&player)||!kind.valid(){return Err("事件请求不属于当前节点或队员".into());}
        if self.guards_cleared&&matches!(kind,Kind::Guards(_)){return Ok(false);}
        if self.seen.iter().any(|j|j.player==player&&j.kind==kind){return Ok(false);}
        self.next_wave=self.next_wave.checked_add(1).ok_or("事件重试次数过多")?;
        let job=Job{epoch,wave:self.next_wave,player,kind};
        self.seen.push(job);self.waiting.push_back(job);Ok(true)
    }
    pub fn start(&mut self)->Option<Job> {
        if self.active.is_some(){return None;}
        self.active=self.waiting.pop_front();self.active
    }
    fn take_active(&mut self,epoch:u64,wave:u8)->RuleResult<Job> {
        let job=self.active.filter(|j|j.epoch==epoch&&j.wave==wave).ok_or("事件结束指令已过期或没有对应波次")?;
        self.active=None;self.completed=Some(job);Ok(job)
    }
    pub fn finish(&mut self,epoch:u64,wave:u8)->RuleResult<Job> {
        let job=self.take_active(epoch,wave)?;
        if matches!(job.kind,Kind::Guards(_)) {
            self.guards_cleared=true;
            self.waiting.retain(|j|!matches!(j.kind,Kind::Guards(_)));
            self.next_wave=job.wave;
            for next in &mut self.waiting {
                let old=*next;self.next_wave+=1;next.wave=self.next_wave;
                if let Some(seen)=self.seen.iter_mut().find(|j|**j==old){*seen=*next;}
            }
        }
        Ok(job)
    }
    pub fn guards_cleared(&self)->bool {self.guards_cleared}
    pub fn busy(&self)->bool {self.active.is_some()||!self.waiting.is_empty()}
    pub fn cancel(&mut self,epoch:u64,wave:u8)->RuleResult<Job> {
        let job=self.take_active(epoch,wave)?;self.seen.retain(|j|*j!=job);Ok(job)
    }
    pub fn active(&self)->Option<Job> {self.active}
    pub fn completed(&self)->Option<Job> {self.completed}
}

#[cfg(test)]
mod tests {
    #[test]
    fn shared_guards_leave_contiguous_scarab_waves_and_retryable_receipts() {
        let mut q=super::Queue::new(1,vec![1,2]).unwrap();
        q.request(1,1,super::Kind::Guards(0)).unwrap();q.request(1,2,super::Kind::Guards(1)).unwrap();
        q.request(1,2,super::Kind::Scarabs(1)).unwrap();q.request(1,1,super::Kind::Scarabs(1)).unwrap();
        assert_eq!(q.start().unwrap().wave,1);q.finish(1,1).unwrap();
        let second=q.start().unwrap();assert_eq!(second.wave,2);assert_eq!(second.kind,super::Kind::Scarabs(1));
        q.cancel(1,2).unwrap();assert!(q.request(1,2,super::Kind::Scarabs(1)).unwrap());
        assert_eq!(q.start().unwrap().wave,3);q.finish(1,3).unwrap();
        assert_eq!(q.start().unwrap().wave,4);q.finish(1,4).unwrap();assert!(!q.busy());
    }
    use super::*;
    #[test]
    fn cancelled_payment_can_retry_without_reusing_its_wave() {
        let mut q=Queue::new(1,vec![1]).unwrap();
        q.request(1,1,Kind::Scarabs(1)).unwrap();let first=q.start().unwrap();
        q.cancel(1,first.wave).unwrap();assert!(q.cancel(1,first.wave).is_err());
        assert!(q.request(1,1,Kind::Scarabs(1)).unwrap());let retry=q.start().unwrap();
        assert_eq!(retry.wave,first.wave+1);assert!(q.finish(1,first.wave).is_err());
    }
    #[test]
    fn two_players_keep_separate_requests_without_overlapping_waves() {
        let mut q=Queue::new(7,vec![10,20]).unwrap();
        assert!(q.request(7,10,Kind::Guards(0)).unwrap());
        assert!(q.request(7,20,Kind::Guards(0)).unwrap());
        assert!(!q.request(7,10,Kind::Guards(0)).unwrap());
        let first=q.start().unwrap();assert_eq!(first.player,10);
        assert!(q.start().is_none());assert!(q.busy());
        assert!(q.finish(6,first.wave).is_err());assert!(q.finish(7,first.wave+1).is_err());
        assert_eq!(q.finish(7,first.wave).unwrap(),first);
        assert!(q.finish(7,first.wave).is_err());
        // [fix] The shared guard fight already cleared the room: the second player's queued wave is
        // dropped instead of trying to reload the same, already defeated map guards.
        assert!(q.guards_cleared());assert!(q.start().is_none());assert!(!q.busy());
        assert!(!q.request(7,10,Kind::Guards(0)).unwrap());
        assert!(!q.request(7,20,Kind::Guards(2)).unwrap());
        assert!(!q.busy());
    }
    #[test]
    fn guard_clear_keeps_scarab_waves_and_aborted_scarabs_do_not_clear_guards() {
        let mut q=Queue::new(3,vec![1,2]).unwrap();
        assert!(q.request(3,1,Kind::Scarabs(1)).unwrap());
        let scarab=q.start().unwrap();q.cancel(3,scarab.wave).unwrap();
        assert!(!q.guards_cleared());
        assert!(q.request(3,1,Kind::Guards(1)).unwrap());
        assert!(q.request(3,2,Kind::Scarabs(1)).unwrap());
        assert!(q.request(3,2,Kind::Guards(1)).unwrap());
        let guards=q.start().unwrap();assert_eq!(guards.kind,Kind::Guards(1));
        q.finish(3,guards.wave).unwrap();
        let next=q.start().unwrap();assert_eq!((next.player,next.kind),(2,Kind::Scarabs(1)));
        q.finish(3,next.wave).unwrap();assert!(q.start().is_none());assert!(!q.busy());
    }
    #[test]
    fn all_player_attempts_are_bounded_and_replays_never_repeat() {
        let mut q=Queue::new(8,(1..=8).collect()).unwrap();
        for player in 1..=8 {for attempt in 1..=3 {assert!(q.request(8,player,Kind::Scarabs(attempt)).unwrap());}}
        for wave in 1..=24 {let job=q.start().unwrap();assert_eq!(job.wave,wave);q.finish(8,wave).unwrap();}
        for player in 1..=8 {for attempt in 1..=3 {assert!(!q.request(8,player,Kind::Scarabs(attempt)).unwrap());}}
        assert!(!q.busy());
    }
    #[test]
    fn stale_unknown_and_invalid_requests_cannot_enqueue_work() {
        let mut q=Queue::new(9,vec![1,2]).unwrap();
        for (epoch,player,kind) in [(8,1,Kind::Guards(0)),(9,3,Kind::Guards(0)),(9,1,Kind::Guards(3)),(9,1,Kind::Scarabs(0)),(9,1,Kind::Scarabs(4))] {
            assert!(q.request(epoch,player,kind).is_err());assert!(!q.busy());
        }
        for ids in [vec![],vec![0],vec![1,1],(1..=9).collect()] {assert!(Queue::new(9,ids).is_err());}
    }
}
