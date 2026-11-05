use std::time::Instant;
use erdtree_tower::{party_latency::{INTERVAL,STALE,Row,Status,Snapshot,Samples},party_roster::Roster};

pub struct Tracker {
    started:Instant,sampled:Option<Instant>,sequence:u64,windows:Vec<(u64,Samples)>,
    snapshot:Option<(Instant,Snapshot)>,
}
impl Default for Tracker {
    fn default()->Self {Self{started:Instant::now(),sampled:None,sequence:0,windows:vec![],snapshot:None}}
}
impl Tracker {
    pub fn poll(&mut self,transport:&super::steam_messages::Transport,roster:&Roster,seen:&[(u64,Instant)],session:u64) {
        if roster.local!=roster.host||self.sampled.is_some_and(|at|at.elapsed()<INTERVAL){return;}
        let now=Instant::now();self.sampled=Some(now);self.sequence+=1;
        let elapsed=now.duration_since(self.started);
        self.windows.retain(|(player,_)|roster.members.contains(player));
        let supported=transport.latency_supported();
        let rows=roster.members.iter().map(|&player|{
            let status=if player==roster.host {Status::Host}else if !supported {Status::Unsupported}else{
                if !self.windows.iter().any(|(id,_)|*id==player){self.windows.push((player,Samples::default()));}
                let stale=seen.iter().find(|(id,_)|*id==player).is_some_and(|(_,at)|at.elapsed()>=STALE);
                let samples=&mut self.windows.iter_mut().find(|(id,_)|*id==player).unwrap().1;
                if stale {*samples=Samples::default();Status::Unavailable}else{samples.observe(elapsed,transport.ping(player))}
            };
            Row{player,status}
        }).collect();
        let snapshot=Snapshot{sequence:self.sequence,rows};
        if let Ok(bytes)=(erdtree_tower::party_handshake::Payload::Latency{session,snapshot:snapshot.clone()}).encode(){
            for &peer in &roster.members {if peer!=roster.local {let _=transport.send(peer,&bytes);}}
        }
        self.snapshot=Some((now,snapshot));
    }
    pub fn accept(&mut self,roster:&Roster,peer:u64,session:u64,expected:u64,snapshot:Snapshot)->bool {
        if roster.local==roster.host||peer!=roster.host||session!=expected||!snapshot.valid(roster.host,&roster.members)
            ||self.snapshot.as_ref().is_some_and(|(_,old)|snapshot.sequence<=old.sequence){return false;}
        self.snapshot=Some((Instant::now(),snapshot));true
    }
    pub fn rows(&self,roster:&Roster)->Vec<Row> {
        if let Some((at,snapshot))=&self.snapshot {
            return snapshot.rows.iter().filter(|r|roster.members.contains(&r.player)).map(|r|Row{player:r.player,status:if at.elapsed()>=STALE&&r.player!=roster.host {Status::Unavailable}else{r.status.clone()}}).collect();
        }
        roster.members.iter().map(|&player|Row{player,status:if player==roster.host {Status::Host}else if self.started.elapsed()>=STALE {Status::Unavailable}else{Status::Waiting}}).collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(sequence:u64)->Snapshot {Snapshot{sequence,rows:vec![Row{player:1,status:Status::Host},Row{player:2,status:Status::Measured(60)}]}}
    #[test]
    fn accepts_only_current_host_session_and_fresh_sequence() {
        let roster=Roster{local:2,host:1,members:vec![1,2]};let mut tracker=Tracker::default();
        assert!(!tracker.accept(&roster,2,7,7,snapshot(1)));
        assert!(!tracker.accept(&roster,1,6,7,snapshot(1)));
        assert!(tracker.accept(&roster,1,7,7,snapshot(2)));
        assert!(!tracker.accept(&roster,1,7,7,snapshot(2)));
        assert!(!tracker.accept(&roster,1,7,7,snapshot(1)));
        assert_eq!(tracker.rows(&roster),snapshot(2).rows);
    }
    #[test]
    fn departed_players_disappear_before_next_host_snapshot() {
        let mut roster=Roster{local:2,host:1,members:vec![1,2,3]};
        let mut tracker=Tracker::default();
        let mut update=snapshot(1);
        update.rows.push(Row{player:3,status:Status::Measured(90)});
        assert!(tracker.accept(&roster,1,7,7,update));
        roster.members.retain(|id|*id!=3);
        assert_eq!(tracker.rows(&roster),snapshot(1).rows);
        assert!(tracker.accept(&roster,1,7,7,snapshot(2)));
    }
    #[test]
    fn stale_host_updates_clear_display_and_reconnect_starts_empty() {
        let roster=Roster{local:2,host:1,members:vec![1,2]};let mut tracker=Tracker::default();
        assert_eq!(tracker.rows(&roster)[1].status,Status::Waiting);
        tracker.snapshot=Some((Instant::now()-STALE,snapshot(1)));
        assert_eq!(tracker.rows(&roster)[0].status,Status::Host);
        assert_eq!(tracker.rows(&roster)[1].status,Status::Unavailable);
        assert!(tracker.accept(&roster,1,7,7,snapshot(2)));
        assert_eq!(tracker.rows(&roster)[1].status,Status::Measured(60));
        assert_eq!(Tracker::default().rows(&roster)[1].status,Status::Waiting);
    }
}
