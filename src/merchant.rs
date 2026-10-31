use serde::{Deserialize,Serialize};
use crate::{Run,NodeKind,RuleResult};

pub const MAX_BELLS:usize=64;
pub const BASE_STOCK:usize=37;
pub const MAX_STOCK:usize=BASE_STOCK+MAX_BELLS*5;
pub const NAMES:[&str;3]=["武器铃珠","防具铃珠","护符铃珠"];

#[derive(Clone,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
pub struct Bells {
    pub counts:[u8;3],
    pub receipts:Vec<(u8,usize)>,
    pub challenge:Option<(u8,usize)>,
}
impl Bells {
    pub fn valid(&self)->bool {
        self.receipts.len()<=MAX_BELLS&&self.counts.iter().map(|&n|n as usize).sum::<usize>()==self.receipts.len()
            &&self.receipts.iter().enumerate().all(|(i,p)|(1..=3).contains(&p.0)&&!self.receipts[..i].contains(p))
            &&self.challenge.is_none_or(|p|(1..=3).contains(&p.0)&&!self.receipts.contains(&p))
    }
    pub fn stock_count(&self)->usize {BASE_STOCK+self.receipts.len()*5}
    pub fn down(&self,act:u8,node:usize)->bool {self.challenge==Some((act,node))||self.receipts.contains(&(act,node))}
}
impl Run {
    pub fn challenge_merchant(&mut self)->RuleResult<()> {
        let node=self.active_node(NodeKind::Shop)?;
        if self.bells.down(self.act,node)||self.bells.challenge.is_some()||self.bells.receipts.len()>=MAX_BELLS{return Err("本次商人挑战不可重复触发".into());}
        self.bells.challenge=Some((self.act,node));Ok(())
    }
    pub fn defeat_hunter(&mut self)->RuleResult<Option<usize>> {
        let node=self.active_node(NodeKind::Shop)?;
        if self.bells.receipts.contains(&(self.act,node)){return Ok(None);}
        if self.bells.challenge!=Some((self.act,node)){return Err("铃珠猎人挑战尚未开始".into());}
        let kind=(crate::mix(self.seed^(self.act as u64)<<40^node as u64^0xbe11)%3) as usize;
        self.bells.counts[kind]+=1;self.bells.receipts.push((self.act,node));self.bells.challenge=None;
        Ok(Some(kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shop()->Run {
        let mut r=Run::new(44,crate::Class::Samurai);
        let node=r.nodes.iter().position(|n|n.kind==NodeKind::Shop).unwrap();
        r.debug_jump(node).unwrap();r.enter(node).unwrap();r
    }
    #[test]
    fn hunter_rewards_once_survive_save_and_block_early_exit() {
        let mut r=shop();assert!(r.defeat_hunter().is_err());r.challenge_merchant().unwrap();
        assert!(r.challenge_merchant().is_err());assert!(r.leave_shop().is_err());
        let mut restored=Run::decode(&r.encode().unwrap()).unwrap();
        let kind=r.defeat_hunter().unwrap().unwrap();assert_eq!(restored.defeat_hunter().unwrap(),Some(kind));
        assert_eq!(r.bells.counts[kind],1);assert_eq!(r.bells.stock_count(),42);
        assert_eq!(r.defeat_hunter().unwrap(),None);assert!(r.challenge_merchant().is_err());
        assert!(r.bells.valid());r.leave_shop().unwrap();assert_eq!(Run::decode(&r.encode().unwrap()).unwrap().bells,r.bells);
        assert_eq!(Run::new(44,crate::Class::Samurai).bells.counts,[0;3]);
    }
    #[test]
    fn forged_counts_and_duplicate_receipts_are_invalid() {
        let mut b=Bells::default();b.counts[0]=1;assert!(!b.valid());
        b.receipts.push((1,3));assert!(b.valid());b.receipts.push((1,3));b.counts[0]=2;assert!(!b.valid());
    }
}
