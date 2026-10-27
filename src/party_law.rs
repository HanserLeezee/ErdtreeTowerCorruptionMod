#[derive(Default)]
pub struct FlaskCounter {epoch:u64,last:Option<u8>,pub uses:u64}
impl FlaskCounter {
    pub fn observe(&mut self,epoch:u64,active:bool,total:Option<u8>) {
        if self.epoch!=epoch {*self=Self{epoch,..Default::default()};}
        let total=total.filter(|_|active);
        if let (Some(before),Some(after))=(self.last,total) {self.uses=self.uses.saturating_add(u64::from(before.saturating_sub(after)));}
        self.last=total;
    }
}
#[derive(Default)]
pub struct DrinkDeltas {seen:std::collections::BTreeMap<u64,u64>}
impl DrinkDeltas {
    pub fn take(&mut self,counts:&[(u64,u64)])->u64 {
        let mut total=0u64;
        for &(id,count) in counts {
            let old=self.seen.entry(id).or_default();
            total=total.saturating_add(count.saturating_sub(*old));*old=(*old).max(count);
        }
        total.min(32)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn flask_counts_ignore_loading_refills_and_reset_each_encounter() {
        let mut c=FlaskCounter::default();c.observe(1,true,Some(10));c.observe(1,true,Some(8));assert_eq!(c.uses,2);
        c.observe(1,false,None);c.observe(1,true,Some(5));assert_eq!(c.uses,2);
        c.observe(1,true,Some(10));c.observe(1,true,Some(9));assert_eq!(c.uses,3);
        c.observe(2,true,Some(9));assert_eq!(c.uses,0);
    }
    #[test]fn each_teammate_drink_is_counted_once_despite_duplicate_or_stale_reports() {
        let mut d=DrinkDeltas::default();assert_eq!(d.take(&[(1,0),(2,1)]),1);
        assert_eq!(d.take(&[(1,0),(2,1)]),0);assert_eq!(d.take(&[(1,1)]),1);
        assert_eq!(d.take(&[(1,1),(2,0)]),0);assert_eq!(d.take(&[(1,1),(2,3)]),2);
    }
}
