use std::{path::{Path,PathBuf},time::Duration};
use erdtree_tower::{Run,Phase};

#[derive(Debug,PartialEq)]
pub enum Status {Waiting,Confirmed,TimedOut}
pub fn status(before:u64,current:u64,elapsed:Duration)->Status {
    if current>before {Status::Confirmed}
    else if elapsed>=Duration::from_secs(5) {Status::TimedOut}
    else {Status::Waiting}
}

pub fn archive_closed(path:&Path)->Result<Option<PathBuf>,String> {
    if !path.exists(){return Ok(None);}
    let session=path.with_extension("json");
    if session.exists() {
        let run=Run::decode(&std::fs::read_to_string(&session).map_err(|e|e.to_string())?)?;
        if !matches!(run.phase,Phase::Dead|Phase::Won){return Ok(None);}
    }
    use std::io::Write;
    let bytes=std::fs::read(path).map_err(|e|e.to_string())?;
    for i in 0..10000 {
        let backup=path.with_extension(format!("party-award.interrupted-{i}"));
        let mut file=match std::fs::OpenOptions::new().create_new(true).write(true).open(&backup) {
            Ok(file)=>file,
            Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>continue,
            Err(e)=>return Err(e.to_string()),
        };
        file.write_all(&bytes).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;
        std::fs::remove_file(path).map_err(|e|e.to_string())?;
        return Ok(Some(backup));
    }
    Err("领奖记录备份数量过多，请先整理备份".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delayed_arrival_is_not_failure_and_timeout_does_not_assume_delivery() {
        assert_eq!(status(4,4,Duration::from_millis(1)),Status::Waiting);
        assert_eq!(status(4,4,Duration::from_secs(4)),Status::Waiting);
        assert_eq!(status(4,5,Duration::from_millis(450)),Status::Confirmed);
        assert_eq!(status(4,4,Duration::from_secs(5)),Status::TimedOut);
        assert_eq!(status(4,5,Duration::from_secs(5)),Status::Confirmed);
        assert_eq!(status(4,3,Duration::from_secs(5)),Status::TimedOut);
    }
    #[test]
    fn interrupted_record_only_archives_after_journey_ends_and_preserves_evidence() {
        let dir=std::env::temp_dir().join(format!("tower-material-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&dir).unwrap();
        let path=dir.join("character.party-award");let session=path.with_extension("json");
        let mut run=Run::new(42,erdtree_tower::Class::Samurai);
        let evidence=run.encode().unwrap();
        std::fs::write(&path,&evidence).unwrap();std::fs::write(&session,&evidence).unwrap();
        assert_eq!(archive_closed(&path).unwrap(),None);
        std::fs::write(&session,"invalid").unwrap();assert!(archive_closed(&path).is_err());assert!(path.exists());
        run.end_on_death();let ended=run.encode().unwrap();std::fs::write(&session,&ended).unwrap();
        let backup=archive_closed(&path).unwrap().unwrap();
        assert_eq!(std::fs::read_to_string(&backup).unwrap(),evidence);
        assert_eq!(std::fs::read_to_string(&session).unwrap(),ended);
        assert!(!path.exists());assert_eq!(archive_closed(&path).unwrap(),None);
        std::fs::write(&path,"second").unwrap();
        let second=archive_closed(&path).unwrap().unwrap();assert_ne!(backup,second);
        assert_eq!(std::fs::read_to_string(&backup).unwrap(),evidence);
        for file in [backup,second,session]{std::fs::remove_file(file).unwrap();}
        std::fs::remove_dir(dir).unwrap();
    }
}
