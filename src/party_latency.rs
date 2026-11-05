use serde::{Deserialize,Serialize};
use std::{collections::VecDeque,time::Duration};

pub const INTERVAL:Duration=Duration::from_secs(2);
pub const STALE:Duration=Duration::from_secs(10);
pub const MAX_MS:u32=60_000;
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub enum Status {Host,Measured(u32),Waiting,Unavailable,Unsupported}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct Row {pub player:u64,pub status:Status}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct Snapshot {pub sequence:u64,pub rows:Vec<Row>}
impl Snapshot {
    pub fn valid(&self,host:u64,members:&[u64])->bool {
        self.sequence>0&&members.len()<=crate::party_roster::MAX_PLAYERS&&self.rows.len()==members.len()
            &&self.rows.iter().zip(members).all(|(r,&id)|r.player==id&&match r.status {
                Status::Host=>id==host,Status::Measured(ms)=>id!=host&&ms<=MAX_MS,_=>id!=host,
            })
    }
}
#[derive(Default)]
pub struct Samples {values:VecDeque<u32>,last:Option<Duration>}
impl Samples {
    pub fn observe(&mut self,now:Duration,value:Option<u32>)->Status {
        if self.last.is_some_and(|last|now.saturating_sub(last)>=STALE){self.values.clear();}
        if let Some(value)=value.filter(|&v|v<=MAX_MS){
            self.last=Some(now);self.values.push_back(value);
            if self.values.len()>4 {self.values.pop_front();}
        }
        if self.values.is_empty(){return if now<STALE {Status::Waiting}else{Status::Unavailable};}
        Status::Measured(self.values.iter().sum::<u32>()/self.values.len() as u32)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn four_sample_average_expires_and_recovers_without_old_values() {
        let mut s=Samples::default();
        assert_eq!(s.observe(Duration::ZERO,None),Status::Waiting);
        for (second,value,average) in [(0,40,40),(2,80,60),(4,120,80),(6,160,100),(8,200,140)] {
            assert_eq!(s.observe(Duration::from_secs(second),Some(value)),Status::Measured(average));
        }
        assert_eq!(s.observe(Duration::from_secs(16),None),Status::Measured(140));
        assert_eq!(s.observe(Duration::from_secs(18),None),Status::Unavailable);
        assert_eq!(s.observe(Duration::from_secs(20),Some(30)),Status::Measured(30));
    }
    #[test]
    fn missing_or_invalid_measurements_are_not_zero_ping() {
        let mut s=Samples::default();
        assert_eq!(s.observe(Duration::from_secs(10),Some(MAX_MS+1)),Status::Unavailable);
        assert_eq!(s.observe(Duration::from_secs(12),Some(0)),Status::Measured(0));
    }
    #[test]
    fn snapshots_require_the_exact_roster_and_one_host() {
        let valid=Snapshot{sequence:1,rows:vec![Row{player:1,status:Status::Host},Row{player:2,status:Status::Measured(42)}]};
        assert!(valid.valid(1,&[1,2]));
        let mut bad=valid.clone();bad.rows[1].player=1;assert!(!bad.valid(1,&[1,2]));
        bad=valid.clone();bad.rows[1].status=Status::Host;assert!(!bad.valid(1,&[1,2]));
        bad=valid.clone();bad.rows[1].status=Status::Measured(MAX_MS+1);assert!(!bad.valid(1,&[1,2]));
        bad=valid.clone();bad.sequence=0;assert!(!bad.valid(1,&[1,2]));
        assert!(!valid.valid(1,&[1,2,3]));
        assert_eq!(serde_json::from_str::<Snapshot>(&serde_json::to_string(&valid).unwrap()).unwrap(),valid);
        let frame=crate::party_handshake::Payload::Latency{session:7,snapshot:valid.clone()};
        match crate::party_handshake::Payload::decode(&frame.encode().unwrap()).unwrap(){
            crate::party_handshake::Payload::Latency{session,snapshot}=>{assert_eq!(session,7);assert_eq!(snapshot,valid);},
            _=>panic!("latency packet did not roundtrip"),
        }
    }
}
