use erdtree_tower::{auras,journal::Record,titles};

#[derive(Default)]
pub struct Unlocks {titles:Vec<u8>,auras:Vec<u8>}
impl Unlocks {
    pub fn load(owner:(u32,i32))->Self {
        Self::parse(&std::fs::read_to_string(super::hub::path(owner,"cosmetics")).unwrap_or_default())
    }
    fn parse(text:&str)->Self {
        let mut result=Self::default();
        for line in text.lines() {
            let Some((kind,ids))=line.trim().split_once('=') else{continue;};
            for id in ids.split(',').filter_map(|s|s.trim().parse::<u8>().ok()) {
                match kind {
                    "titles" if titles::lookup(id).is_some()=>result.titles.push(id),
                    "auras" if auras::lookup(id).is_some()=>result.auras.push(id),
                    _=>{},
                }
            }
        }
        result
    }
    pub fn title(&self,id:u8,records:&[Record])->bool {
        titles::lookup(id).is_some()&&(self.titles.contains(&id)||titles::unlocked(id,records))
    }
    pub fn aura(&self,id:u8,records:&[Record])->bool {
        auras::lookup(id).is_some()&&(self.auras.contains(&id)||auras::unlocked(id,records))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_grants_unlock_only_valid_cosmetics_without_run_records() {
        let grants=Unlocks::parse("titles=2,7,0,255,bad\nauras=5,9,255\nunknown=3");
        assert!(grants.title(2,&[])&&grants.title(7,&[])&&grants.aura(5,&[])&&grants.aura(9,&[]));
        assert!(!grants.title(3,&[])&&!grants.aura(7,&[])&&!grants.title(255,&[])&&!grants.aura(255,&[]));
        assert!(!grants.title(0,&[])&&!grants.aura(0,&[]));
    }
    #[test]
    fn no_grants_preserve_normal_unlocks() {
        let grants=Unlocks::parse("invalid");
        for title in titles::TITLES.iter() {assert_eq!(grants.title(title.id,&[]),titles::unlocked(title.id,&[]));}
        for aura in auras::AURAS.iter() {assert_eq!(grants.aura(aura.id,&[]),auras::unlocked(aura.id,&[]));}
    }
}
