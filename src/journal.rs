//! 旅程档案：每局结束时追加到 tower-runs.jsonl 的一行记录。结算页、往期旅程与图鉴都从这里读数据。
use serde::{Deserialize,Serialize};
use crate::{Class,Phase,Run,contracts::{Kind,Oath},embers::Ember,law::Law};

/// 一场战斗：幕、层、节点类型（怪物 / 精英 / Boss）、胜负与敌人
#[derive(Clone,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
pub struct Fight {pub act:u8,pub layer:usize,pub kind:String,pub won:bool,pub enemies:Vec<String>}

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="lowercase")]
pub enum Outcome {Won,Dead,#[default] Ended}

/// 一局旅程的结算记录
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(default)]
pub struct Record {
    /// 结束时间（Unix 秒）
    pub time:u64,
    pub seed:u64,
    /// true 时 seed 是整局种子，否则是本幕种子（读档继续的旧旅程）
    pub whole_seed:bool,
    pub class:Class,
    pub result:Outcome,
    pub act:u8,
    pub layer:usize,
    pub nodes:u32,
    pub battles:u32,
    pub items:usize,
    pub flask:u8,
    pub memory:u8,
    pub law:Law,
    pub debug:bool,
    pub oaths:Vec<Oath>,
    pub embers:Vec<Ember>,
    /// 本局拿到过的装备词条（去掉数值的名字，如“致命一击伤害”）
    pub affixes:Vec<String>,
    pub fights:Vec<Fight>,
}
impl Default for Record {
    fn default()->Self {
        Self{time:0,seed:0,whole_seed:true,class:Class::Vagabond,result:Outcome::Ended,act:1,layer:1,nodes:0,battles:0,items:0,flask:0,memory:0,
            law:Law::default(),debug:false,oaths:vec![],embers:vec![],affixes:vec![],fights:vec![]}
    }
}
impl Record {
    pub fn new(run:&Run,fights:Vec<Fight>,affixes:Vec<String>,time:u64)->Self {
        let layer=run.visited.last().and_then(|&id|run.nodes.get(id)).map_or(1,|node|node.layer+1);
        Self{time,seed:run.initial_seed.unwrap_or(run.seed),whole_seed:run.initial_seed.is_some()||run.act==1,class:run.class,
            result:match run.phase {Phase::Won=>Outcome::Won,Phase::Dead=>Outcome::Dead,_=>Outcome::Ended},
            act:run.act,layer:run.reached_layer.max(layer),nodes:run.completed_nodes,battles:run.completed_battles,items:run.claimed_items.len(),
            flask:run.flask_bonus,memory:run.memory_bonus,law:run.law,debug:run.debug,
            oaths:run.contract.as_ref().map(|c|c.active()).unwrap_or_default(),embers:run.embers.clone(),affixes,fights}
    }
    pub fn line(&self)->String {serde_json::to_string(self).unwrap_or_default()}
    /// 读整个档案：跳过坏行，连续重复写入的同一局只留一条，按写入顺序返回
    pub fn parse_all(text:&str)->Vec<Self> {
        let mut records:Vec<Self>=Vec::new();
        for record in text.lines().filter_map(|line|serde_json::from_str::<Self>(line.trim()).ok()) {
            if records.last().is_some_and(|last|last.same_run(&record)) {continue;}
            records.push(record);
        }
        records
    }
    fn same_run(&self,other:&Self)->bool {
        (self.seed,self.class,self.result,self.act,self.layer,self.nodes,self.battles)==(other.seed,other.class,other.result,other.act,other.layer,other.nodes,other.battles)
    }
}
/// 结算页的一幕：击败的 Boss、打过的战斗与精英
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct Act {pub bosses:Vec<String>,pub battles:usize,pub elites:usize}
/// 结算页的旅程回顾：三幕各自的战绩，以及倒下的那一战
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct Review {pub acts:[Act;3],pub fatal:Option<Fight>}
pub fn review(record:&Record)->Review {
    let mut review=Review::default();
    for fight in &record.fights {
        let act=&mut review.acts[usize::from(fight.act.clamp(1,3)-1)];
        act.battles+=1;
        if fight.won&&fight.kind=="精英" {act.elites+=1;}
        if fight.won&&fight.kind=="Boss" {act.bosses.push(fight.enemies.first().cloned().unwrap_or_else(||"Boss".into()));}
    }
    if record.result==Outcome::Dead {review.fatal=record.fights.last().filter(|f|!f.won).cloned();}
    review
}
/// 装备说明里的一行词条去掉数值：“致命一击伤害 +17%” → “致命一击伤害”
pub fn affix_name(line:&str)->Option<&str> {
    let (name,value)=line.trim().rsplit_once(' ')?;
    (value.starts_with(['+','-'])&&value.chars().any(|c|c.is_ascii_digit())&&!name.trim().is_empty()).then(||name.trim())
}
/// 图鉴：所有非调试旅程累计的誓约、余烬、Boss 与词条
#[derive(Clone,Debug,Default,PartialEq)]
pub struct Codex {
    pub runs:u32,
    pub wins:u32,
    pub best_law:u32,
    /// 走得最远的位置（幕、层）
    pub deepest:(u8,usize),
    /// 按 Kind::ALL 顺序：立誓次数、最高等级
    pub oaths:Vec<(Kind,u32,u8)>,
    /// 按 Ember::ALL 顺序：获得次数
    pub embers:Vec<(Ember,u32)>,
    /// 遇到过的 Boss：名字、交手次数、击败次数
    pub bosses:Vec<(String,u32,u32)>,
    /// 拿到过的词条：名字、出现在几局里
    pub affixes:Vec<(String,u32)>,
}
impl Codex {
    pub fn boss(&self,name:&str)->Option<(u32,u32)> {self.bosses.iter().find(|(n,..)|n==name).map(|(_,seen,won)|(*seen,*won))}
    pub fn affix(&self,name:&str)->u32 {self.affixes.iter().find(|(n,_)|n==name).map_or(0,|(_,count)|*count)}
}
pub fn codex(records:&[Record])->Codex {
    let mut codex=Codex{oaths:Kind::ALL.iter().map(|k|(*k,0,0)).collect(),embers:Ember::ALL.iter().map(|e|(*e,0)).collect(),..Default::default()};
    for record in records.iter().filter(|r|!r.debug) {
        codex.runs+=1;
        if record.result==Outcome::Won {codex.wins+=1;codex.best_law=codex.best_law.max(record.law.score());}
        codex.deepest=codex.deepest.max((record.act,record.layer));
        for oath in &record.oaths {if let Some(entry)=codex.oaths.iter_mut().find(|(k,..)|*k==oath.kind) {entry.1+=1;entry.2=entry.2.max(oath.level.max(1));}}
        for ember in &record.embers {if let Some(entry)=codex.embers.iter_mut().find(|(e,_)|e==ember) {entry.1+=1;}}
        for fight in record.fights.iter().filter(|f|f.kind=="Boss") {
            let Some(name)=fight.enemies.first() else{continue};
            match codex.bosses.iter_mut().find(|(n,..)|n==name) {
                Some(entry)=>{entry.1+=1;entry.2+=u32::from(fight.won);},
                None=>codex.bosses.push((name.clone(),1,u32::from(fight.won))),
            }
        }
        let mut seen:Vec<&str>=Vec::new();
        for name in record.affixes.iter().map(String::as_str).filter(|n|!n.is_empty()) {
            if seen.contains(&name) {continue;}
            seen.push(name);
            match codex.affixes.iter_mut().find(|(n,_)|n==name) {Some(entry)=>entry.1+=1,None=>codex.affixes.push((name.to_owned(),1))}
        }
    }
    codex
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fight(act:u8,layer:usize,kind:&str,won:bool,enemies:&[&str])->Fight {Fight{act,layer,kind:kind.into(),won,enemies:enemies.iter().map(|e|e.to_string()).collect()}}
    #[test]
    fn records_roundtrip_skip_bad_lines_and_collapse_repeated_writes() {
        let mut run=Run::new(42,Class::Samurai);
        run.debug_oath(Kind::Steel,3).unwrap();run.debug_ember(Ember::Frenzy,true).unwrap();run.debug=false;
        run.end_on_death();
        let record=Record::new(&run,vec![fight(1,3,"怪物",false,&["人形苍蝇"])],vec!["致命一击伤害".into()],1_800_000_000);
        assert_eq!((record.result,record.oaths.clone(),record.embers.clone(),record.whole_seed),(Outcome::Dead,vec![Oath{kind:Kind::Steel,level:3}],vec![Ember::Frenzy],true));
        let mut other=record.clone();other.seed=7;
        let text=format!("{}\n{}\nnot json\n{}\n\n",record.line(),record.line(),other.line());
        assert_eq!(Record::parse_all(&text),vec![record.clone(),other]);
        // 旧版本写下的行缺字段时取默认值
        assert_eq!(Record::parse_all(r#"{"seed":5,"result":"won"}"#)[0].result,Outcome::Won);
    }
    #[test]
    fn review_names_bosses_per_act_and_the_fatal_fight() {
        let death=fight(3,9,"精英",false,&["黑骑士","梅瑟莫士兵"]);
        let mut record=Record{fights:vec![fight(2,8,"精英",true,&["黑骑士"]),fight(2,16,"Boss",true,&["拉卡德"]),death.clone()],..Default::default()};
        assert_eq!(review(&record).fatal,None,"only a death names the fatal fight");
        record.result=Outcome::Dead;
        let review=review(&record);
        assert_eq!(review.acts[1],Act{bosses:vec!["拉卡德".into()],battles:2,elites:1});
        assert_eq!(review.acts[2],Act{bosses:vec![],battles:1,elites:0});
        assert_eq!(review.fatal,Some(death));
    }
    #[test]
    fn codex_counts_real_runs_only() {
        let law=Law::default().toggle(0).toggle(1);
        let won=Record{result:Outcome::Won,act:3,layer:16,law,oaths:vec![Oath{kind:Kind::Blood,level:2}],embers:vec![Ember::Thorns],
            affixes:vec!["致命一击伤害".into(),"致命一击伤害".into(),"最大生命".into()],fights:vec![fight(1,16,"Boss",true,&["恶兆妖鬼 玛尔基特"])],..Default::default()};
        let lost=Record{result:Outcome::Dead,act:1,layer:16,oaths:vec![Oath{kind:Kind::Blood,level:1}],affixes:vec!["最大生命".into()],fights:vec![fight(1,16,"Boss",false,&["恶兆妖鬼 玛尔基特"])],..Default::default()};
        let debug=Record{debug:true,embers:vec![Ember::Frost],..Default::default()};
        let codex=codex(&[won,lost,debug]);
        assert_eq!((codex.runs,codex.wins,codex.best_law,codex.deepest),(2,1,law.score(),(3,16)));
        assert_eq!(codex.oaths[Kind::Blood.index()],(Kind::Blood,2,2));
        assert_eq!(codex.embers[Ember::Thorns.index()].1,1);
        assert_eq!(codex.embers[Ember::Frost.index()].1,0,"debug runs do not unlock the codex");
        assert_eq!(codex.boss("恶兆妖鬼 玛尔基特"),Some((2,1)));
        assert_eq!((codex.affix("致命一击伤害"),codex.affix("最大生命"),codex.affix("物理承伤")),(1,2,0));
    }
    #[test]
    fn affix_names_drop_the_numbers() {
        assert_eq!(affix_name("致命一击伤害 +17%"),Some("致命一击伤害"));
        assert_eq!(affix_name("耐力恢复速度 +4"),Some("耐力恢复速度"));
        assert_eq!(affix_name("物理承伤 -2%"),Some("物理承伤"));
        assert_eq!(affix_name("补正组合：灵巧/信仰"),None);
    }
}
