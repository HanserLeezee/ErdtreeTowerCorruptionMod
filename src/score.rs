//! 本局评分：结算页的大数字与评级（参考危机合约）。评分就是苦难律法分数；评级按走到哪里：
//! S 登顶（三幕全部通关）、A 走到第三幕、B 第二幕、C 第一幕。按旅程记录算，往期旅程也能算
use crate::journal::{Outcome,Record};

/// 每幕的层数
pub const FLOORS:u32=16;

/// 一局的评分与旅程统计
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Score {
    /// 评分：苦难律法分数
    pub law:u32,
    pub won:bool,pub act:u8,
    pub floors:u32,pub bosses:u32,pub elites:u32,pub battles:u32,
}
impl Score {
    pub fn of(record:&Record)->Self {
        let won=record.result==Outcome::Won;
        let act=record.act.clamp(1,3);
        let floors=if won {3*FLOORS}else{u32::from(act-1)*FLOORS+(record.layer as u32).min(FLOORS)};
        // Boss 按走到第几幕算（战斗记录可能不全，比如更早版本的联机成员），精英按战斗记录
        let bosses=u32::from(act-1)+u32::from(won);
        let elites=record.fights.iter().filter(|f|f.won&&f.kind=="精英").count() as u32;
        Self{law:record.law.score(),won,act,floors,bosses,elites,battles:record.battles}
    }
    pub fn total(&self)->u32 {self.law}
    /// S 登顶、A 第三幕、B 第二幕、C 第一幕
    pub fn rank(&self)->char {if self.won {'S'}else{['C','B','A'][usize::from(self.act-1)]}}
    /// 比较纪录用：先比评级，再比评分
    pub fn key(&self)->(u8,u32) {(if self.won {4}else{self.act},self.law)}
    /// 旅程统计，按显示顺序：(名称, 数量)
    pub fn lines(&self)->[(&'static str,u32);4] {[("层数",self.floors),("Boss",self.bosses),("精英",self.elites),("战斗",self.battles)]}
}
/// 往期最好的一局（不算调试旅程；records 里与 current 是同一局的那条不算）：先比评级，再比评分
pub fn best(records:&[Record],current:&Record)->Option<Score> {
    records.iter().filter(|r|!r.debug&&r.time!=current.time).map(Score::of).max_by_key(Score::key)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(result:Outcome,act:u8,layer:usize,law:u32)->Record {
        let mut r=Record{result,act,layer,battles:24,..Default::default()};
        let mut l=crate::law::Law::default();
        for id in 0..crate::law::NODES.len() {if l.score()>=law {break;}if l.available(id) {l=l.toggle(id);}}
        r.law=l;r
    }
    #[test]
    fn the_score_is_the_law_score_and_the_rank_is_how_far_the_run_went() {
        let won=Score::of(&record(Outcome::Won,3,16,100));
        assert!(won.total()>=100&&won.total()==won.law);assert_eq!(won.rank(),'S');assert_eq!((won.floors,won.bosses),(48,3));
        assert_eq!(Score::of(&record(Outcome::Dead,3,5,0)).rank(),'A');
        assert_eq!(Score::of(&record(Outcome::Dead,2,9,0)).rank(),'B');
        let early=Score::of(&record(Outcome::Dead,1,5,0));assert_eq!((early.rank(),early.total(),early.floors),('C',0,5));
        assert_eq!(Score::of(&record(Outcome::Ended,2,3,0)).rank(),'B',"a paused run is ranked by where it stopped");
    }
    #[test]
    fn best_ranks_before_score_and_skips_debug_runs_and_the_current_run() {
        let mut cleared=record(Outcome::Won,3,16,40);cleared.time=1;
        let mut deep=record(Outcome::Dead,1,3,200);deep.time=2;
        let mut debug=record(Outcome::Won,3,16,300);debug.time=3;debug.debug=true;
        let best=best(&[cleared.clone(),deep.clone(),debug],&deep).unwrap();
        assert_eq!(best.rank(),'S',"a clear beats a higher score that died early");
        assert!(super::best(&[cleared.clone()],&cleared).is_none());
    }
}
