use eldenring::cs::{ItemCategory,ItemId};

#[link(name="kernel32")]
unsafe extern "system" {fn GetModuleHandleW(name:*const u16)->*mut std::ffi::c_void;}

pub fn active()->bool {
    if std::env::var("ERDTREE_TOWER_SEAMLESS_ENTRY").as_deref()!=Ok("1") {return false;}
    let name:Vec<_>="ersc.dll\0".encode_utf16().collect();
    !unsafe {GetModuleHandleW(name.as_ptr())}.is_null()
}
pub fn blocked(online:bool,seamless:bool)->bool {online&&!seamless}
/// 会话里每个队员的 Steam 名字与本机的 Steam ID（读名单时刷新），路线图的队员图例用来把投票序号对上人
static NAMES:std::sync::Mutex<Vec<(u64,String)>>=std::sync::Mutex::new(Vec::new());
static LOCAL:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
fn remember(session:&eldenring::cs::CSSessionManager) {
    let names:Vec<_>=session.players.iter().map(|p|(p.base.steam_id,p.base.steam_name.to_string().unwrap_or_default().trim().to_owned())).collect();
    if let Some(local)=session.players.iter().find(|p|p.is_local_player) {LOCAL.store(local.base.steam_id,std::sync::atomic::Ordering::Relaxed);}
    if let Ok(mut known)=NAMES.lock() {*known=names;}
}
pub fn player_name(id:u64)->Option<String> {NAMES.lock().ok()?.iter().find(|(p,_)|*p==id).map(|(_,n)|n.clone()).filter(|n|!n.is_empty())}
pub fn local_player()->Option<u64> {Some(LOCAL.load(std::sync::atomic::Ordering::Relaxed)).filter(|id|*id!=0)}
pub fn roster(session:&eldenring::cs::CSSessionManager)->Result<erdtree_tower::party_roster::Roster,String> {
    use eldenring::cs::{LobbyState,ProtocolState};
    use erdtree_tower::party_roster::{Member,Roster};
    if !active()||!matches!(session.lobby_state,LobbyState::Host|LobbyState::Client)
        ||session.protocol_state!=ProtocolState::Ingame {return Err("等待无缝会话就绪".into());}
    remember(session);
    if session.players.len()>erdtree_tower::party_roster::MAX_PLAYERS {return Err("爬塔最多支持 8 人（含房主）".into());}
    let host=session.host_player.as_option().ok_or("会话房主未加载")?.steam_id;
    let entries:Vec<_>=session.players.iter().map(|p|Member{id:p.base.steam_id,local:p.is_local_player,host:p.is_host,joining:p.join_wait||p.rebreak_in}).collect();
    Roster::read(host,session.lobby_state==LobbyState::Host,&entries)
}
pub fn preserve(item:ItemId,seamless:bool)->bool {
    seamless&&item.category()==ItemCategory::Goods&&matches!(item.param_id(),8380001..=8380006|8380012)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seamless_online_keeps_runtime_but_vanilla_online_stays_blocked() {
        assert!(!blocked(true,true));assert!(blocked(true,false));
        assert!(!blocked(false,false));assert!(!blocked(false,true));
    }
    #[test]
    fn reset_keeps_only_seamless_tools_not_rot_or_run_loot() {
        for id in 8380000..=8380013 {
            let item=ItemId::new(ItemCategory::Goods,id).unwrap();
            assert_eq!(preserve(item,true),matches!(id,8380001..=8380006|8380012));
            assert!(!preserve(item,false));
            assert!(!preserve(ItemId::new(ItemCategory::Weapon,id).unwrap(),true));
        }
        for id in [1000,1050,8000,8158,9500000] {assert!(!preserve(ItemId::new(ItemCategory::Goods,id).unwrap(),true));}
    }
}
