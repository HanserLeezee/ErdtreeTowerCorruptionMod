use serde::{Deserialize,Serialize};
pub const PARTS:[&str;4]=["头部","胸部","手部","腿部"];
pub const BARE:[i32;4]=[10000,10100,10200,10300];
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub struct Appearance {pub armor:[i32;4]}
impl Default for Appearance {fn default()->Self {Self{armor:[-1;4]}}}
impl Appearance {
    pub fn encode(self)->String {serde_json::to_string(&self).unwrap()}
    pub fn decode(text:&str)->Option<Self> {serde_json::from_str::<Self>(text).ok().filter(|s|s.armor.iter().all(|id|*id>=-1))}
    pub fn set(&mut self,slot:usize,id:i32,category:impl Fn(i32)->Option<usize>)->Result<(),&'static str> {
        if slot>=4||id< -1||id!=-1&&category(id)!=Some(slot){return Err("该外观不属于此防具部位");}
        self.armor[slot]=id;Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appearance_is_separate_per_slot_and_roundtrips() {
        let mut look=Appearance::default();
        for slot in 0..4 {look.set(slot,BARE[slot],|id|BARE.iter().position(|p|*p==id)).unwrap();}
        assert_eq!(Appearance::decode(&look.encode()),Some(look));
        let before=look;assert!(look.set(0,10100,|_|Some(1)).is_err());assert_eq!(look,before);
        look.set(2,-1,|_|None).unwrap();assert_eq!(look.armor,[10000,10100,-1,10300]);
        assert!(Appearance::decode("broken").is_none());
    }
    #[test]
    fn invalid_parts_and_values_do_not_change_saved_appearance() {
        let mut look=Appearance::default();let original=look;
        for (slot,id) in [(4,10000),(0,-2),(0,999999)] {
            assert!(look.set(slot,id,|_|None).is_err());assert_eq!(look,original);
        }
        assert!(Appearance::decode(r#"{"armor":[-1,-1,-2,-1]}"#).is_none());
        assert!(Appearance::decode(r#"{"armor":[-1,-1,-1]}"#).is_none());
        look.set(1,20100,|_|Some(1)).unwrap();
        look.set(1,-1,|_|None).unwrap();assert_eq!(look,original);
    }
}
