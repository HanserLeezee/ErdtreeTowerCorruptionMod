use eldenring::cs::{CSEventFlagMan,PlayerIns};
use fromsoftware_shared::FromStatic;

// Vanilla ItemLotParam_map acquisition flags and common event 1450's derived affinity flags.
const UNLOCKS:&[(u32,&[u32])]=&[
    (8970,&[65610,65620,65630]),(8971,&[65640,65650]),
    (8972,&[65660,65670]),(8973,&[65680,65690]),(8974,&[65720,65700,65710]),
];
fn missing_flags(owns:impl Fn(u32)->bool,enabled:impl Fn(u32)->bool)->Vec<u32> {
    UNLOCKS.iter().filter(|(id,_)|owns(*id)).flat_map(|(_,flags)|flags.iter().copied()).filter(|id|!enabled(*id)).collect()
}
pub fn sync_owned()->Result<(),String> {
    let player=unsafe {PlayerIns::local_player()}.map_err(|_|"角色未加载，无法核对砥石刀")?;
    let data=unsafe {player.player_game_data.as_ref()};
    let flags=unsafe {CSEventFlagMan::instance_mut()}.map_err(|_|"质变解锁事件系统未加载")?;
    let pending=missing_flags(|id|crate::loot::count(data,0x40000000|id)>0,|id|flags.virtual_memory_flag.get_flag(id));
    for &id in &pending {flags.virtual_memory_flag.set_flag(id,true);}
    if !pending.is_empty(){crate::log(format!("Whetblade affinity unlocks repaired from owned goods: {pending:?}"));}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn black_whetblade_unlocks_only_its_three_affinities() {
        assert_eq!(missing_flags(|id|id==8974, |_|false),vec![65720,65700,65710]);
    }
    #[test]
    fn no_whetblade_or_only_basic_knife_does_not_unlock_affinities() {
        assert!(missing_flags(|_|false, |_|false).is_empty());
        assert!(missing_flags(|id|id==8590, |_|false).is_empty());
    }
    #[test]
    fn repairs_partial_unlock_without_rewriting_existing_flags() {
        assert_eq!(missing_flags(|id|id==8974, |id|id==65720),vec![65700,65710]);
        assert!(missing_flags(|_|true, |_|true).is_empty());
    }
    #[test]
    fn all_five_whetblades_match_vanilla_event_1450() {
        assert_eq!(missing_flags(|_|true, |_|false),vec![65610,65620,65630,65640,65650,65660,65670,65680,65690,65720,65700,65710]);
    }
}
