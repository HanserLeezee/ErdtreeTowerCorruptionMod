// 旅程经过：每场战斗结束时记一行（幕、层、节点类型、胜负、敌人），存放在存档旁边的 .history 文件，新开一局时清空。
// 整局结束时连同构筑写成一条 journal::Record，追加到启动目录的 tower-runs.jsonl；结算页、往期旅程与图鉴都读它。
use std::{io::Write,path::{Path,PathBuf}};
use erdtree_tower::{NodeKind,Run,journal::{Fight,Record}};

fn line(fight:&Fight)->String {format!("{}|{}|{}|{}|{}",fight.act,fight.layer,fight.kind,u8::from(fight.won),fight.enemies.join("、"))}
fn parse(line:&str)->Option<Fight> {
    let p:Vec<_>=line.splitn(5,'|').collect();
    if p.len()!=5 {return None;}
    Some(Fight{act:p[0].parse().ok()?,layer:p[1].parse().ok()?,kind:p[2].into(),won:p[3]=="1",enemies:p[4].split('、').filter(|s|!s.is_empty()).map(str::to_owned).collect()})
}
pub fn path(owner:(u32,i32))->PathBuf {super::hub::path(owner,"history")}
pub fn append(path:&Path,fight:&Fight) {
    let result=std::fs::OpenOptions::new().create(true).append(true).open(path).and_then(|mut f|writeln!(f,"{}",line(fight)));
    if let Err(error)=result {super::log(format!("Run history write failed: {error}"));}
}
pub fn read(path:&Path)->Vec<Fight> {std::fs::read_to_string(path).map(|t|t.lines().filter_map(parse).collect()).unwrap_or_default()}
pub fn clear(path:&Path) {if path.exists() {if let Err(error)=std::fs::remove_file(path){super::log(format!("Run history reset failed: {error}"));}}}
/// 一场战斗的记录：节点类型取自战斗节点，敌人名字去重
pub fn entry(run:&Run,node:Option<usize>,won:bool,enemies:&[&str])->Fight {
    let kind=node.and_then(|id|run.nodes.get(id)).map_or("战斗",|n|if n.kind==NodeKind::Boss{"Boss"}else{n.kind.label()});
    let layer=node.and_then(|id|run.nodes.get(id)).map_or(run.reached_layer,|n|n.layer+1);
    let mut names:Vec<String>=Vec::new();
    for name in enemies {if !names.iter().any(|n|n==name) {names.push((*name).to_owned());}}
    Fight{act:run.act,layer,kind:kind.into(),won,enemies:names}
}
fn archive_path()->PathBuf {super::LOG_PATH.with_file_name("tower-runs.jsonl")}
/// 本局拿到过的装备词条名（去掉数值）
fn affixes(run:&Run)->Vec<String> {
    let mut names:Vec<String>=Vec::new();
    for gear in run.claimed_items.iter().filter_map(|id|super::reward_ui::affix_summary(*id)) {
        for name in gear.lines.iter().filter_map(|line|erdtree_tower::journal::affix_name(line)) {if !names.iter().any(|n|n==name){names.push(name.to_owned());}}
    }
    names
}
/// 整局结束：生成结算记录并追加到 tower-runs.jsonl（一行一局），按角色存档给这一局的职业加熟练度，一并返回给结算页
pub fn finish(run:&Run,fights:Vec<Fight>,owner:Option<(u32,i32)>)->(Record,Option<erdtree_tower::legacy::Gain>) {
    let time=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0,|t|t.as_secs());
    let record=Record::new(run,fights,affixes(run),time);
    let result=std::fs::OpenOptions::new().create(true).append(true).open(archive_path()).and_then(|mut f|writeln!(f,"{}",record.line()));
    if let Err(error)=result {super::log(format!("Run archive write failed: {error}"));}
    let gain=owner.and_then(|owner|super::legacy::award(owner,&record));
    (record,gain)
}
/// 旅程档案里的全部记录（按写入顺序）
pub fn records()->Vec<Record> {Record::parse_all(&std::fs::read_to_string(archive_path()).unwrap_or_default())}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_lines_roundtrip() {
        let fight=Fight{act:3,layer:9,kind:"精英".into(),won:false,enemies:vec!["黑骑士".into(),"梅瑟莫士兵".into()]};
        assert_eq!(parse(&line(&fight)),Some(fight));
        assert_eq!(parse("bad line"),None);
    }
}
