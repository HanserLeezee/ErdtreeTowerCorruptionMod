// 联机大厅（房主侧）：启动器“开房”后写下 tower-lobby.json（erdtree_tower::lobby::HostRoom），游戏里用 Steam 大厅把房间公开出去，
// 每 5 秒更新人数与进度；游戏关掉 Steam 自动解散大厅。浏览与加入在启动器里（launcher/Tower.Launcher/SteamLobby.cs）。
// Steam 接口按 steam_messages 的做法从已加载的 steam_api64（无缝联机会把原版改名为 _o）里取，不新加载任何库。
// CallResult 不注册回调，按调用编号轮询 ISteamUtils::IsAPICallCompleted / GetAPICallResult。
use std::{ffi::{c_void,CStr,CString},time::{Duration,Instant}};
use erdtree_tower::lobby::HostRoom;

#[link(name="kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name:*const u16)->*mut c_void;
    fn GetProcAddress(module:*mut c_void,name:*const i8)->*mut c_void;
}
type Accessor=unsafe extern "C" fn()->*mut c_void;
type CreateLobby=unsafe extern "C" fn(*mut c_void,i32,i32)->u64;
type SetLobbyData=unsafe extern "C" fn(*mut c_void,u64,*const i8,*const i8)->bool;
type IsCompleted=unsafe extern "C" fn(*mut c_void,u64,*mut bool)->bool;
type CallResult=unsafe extern "C" fn(*mut c_void,u64,*mut c_void,i32,i32,*mut bool)->bool;
type PersonaName=unsafe extern "C" fn(*mut c_void)->*const i8;
/// k_ELobbyTypePublic
const PUBLIC:i32=2;
/// LobbyCreated_t：k_iSteamMatchmakingCallbacks + 13；Windows 上按 8 字节对齐（EResult、补齐、大厅编号）
const LOBBY_CREATED:i32=513;
#[repr(C)]
#[derive(Default)]
struct LobbyCreated {result:i32,lobby:u64}

struct Api {
    matchmaking:*mut c_void,utils:*mut c_void,friends:*mut c_void,
    create:CreateLobby,set:SetLobbyData,completed:IsCompleted,result:CallResult,persona:PersonaName,
}
impl Api {
    fn load()->Result<Self,String> {
        let module=["steam_api64_o.dll","steam_api64.dll"].into_iter().find_map(|name|{
            let wide:Vec<_>=name.encode_utf16().chain(std::iter::once(0)).collect();
            let module=unsafe {GetModuleHandleW(wide.as_ptr())};
            (!module.is_null()&&!unsafe {GetProcAddress(module,c"SteamAPI_SteamMatchmaking_v009".as_ptr())}.is_null()).then_some(module)
        }).ok_or("没有找到已加载的 Steam 大厅接口")?;
        let symbol=|names:&[&CStr]|->Result<*mut c_void,String>{
            names.iter().map(|n|unsafe {GetProcAddress(module,n.as_ptr())}).find(|p|!p.is_null())
                .ok_or_else(||format!("Steam API 缺少 {}",names[0].to_string_lossy()))
        };
        unsafe {
            let interface=|names:&[&CStr]|->Result<*mut c_void,String>{
                let accessor:Accessor=std::mem::transmute(symbol(names)?);
                let p=accessor();if p.is_null(){Err("Steam 接口未就绪，请确认 Steam 已登录".into())}else{Ok(p)}
            };
            Ok(Self{
                matchmaking:interface(&[c"SteamAPI_SteamMatchmaking_v009"])?,
                utils:interface(&[c"SteamAPI_SteamUtils_v010",c"SteamAPI_SteamUtils_v009"])?,
                friends:interface(&[c"SteamAPI_SteamFriends_v017",c"SteamAPI_SteamFriends_v016",c"SteamAPI_SteamFriends_v015"])?,
                create:std::mem::transmute(symbol(&[c"SteamAPI_ISteamMatchmaking_CreateLobby"])?),
                set:std::mem::transmute(symbol(&[c"SteamAPI_ISteamMatchmaking_SetLobbyData"])?),
                completed:std::mem::transmute(symbol(&[c"SteamAPI_ISteamUtils_IsAPICallCompleted"])?),
                result:std::mem::transmute(symbol(&[c"SteamAPI_ISteamUtils_GetAPICallResult"])?),
                persona:std::mem::transmute(symbol(&[c"SteamAPI_ISteamFriends_GetPersonaName"])?),
            })
        }
    }
    fn persona(&self)->String {
        let p=unsafe {(self.persona)(self.friends)};
        if p.is_null(){return "褪色者".into();}
        unsafe {CStr::from_ptr(p)}.to_string_lossy().into_owned()
    }
    fn set(&self,lobby:u64,key:&str,value:&str)->bool {
        let (Ok(key),Ok(value))=(CString::new(key),CString::new(value)) else{return false};
        unsafe {(self.set)(self.matchmaking,lobby,key.as_ptr(),value.as_ptr())}
    }
}
// 只在游戏任务线程上调用
unsafe impl Send for Api {}

enum Stage {
    /// 还没读房间文件
    Unread,
    /// 不是房主启动（没有房间文件）或文件无效：什么都不做
    Off,
    Starting{retry_at:Instant,attempts:u8},
    Creating{call:u64,since:Instant,attempts:u8},
    Open{lobby:u64,sent:Vec<(&'static str,String)>,at:Instant},
}
struct Host {room:Option<HostRoom>,api:Option<Api>,stage:Stage}
static HOST:std::sync::Mutex<Host>=std::sync::Mutex::new(Host{room:None,api:None,stage:Stage::Unread});

fn room_file()->std::path::PathBuf {super::options::dir().join(erdtree_tower::lobby::FILE)}

/// 每帧调用（游戏任务线程）：state 为进度说明（大赐福 / 第 N 幕）。人数与“已开启联机”从无缝联机的会话里读
pub fn tick(state:&str,started:bool) {
    let Ok(mut host)=HOST.lock() else{return};
    let host=&mut *host;
    if matches!(host.stage,Stage::Unread) {
        host.stage=Stage::Off;
        if !super::seamless::active() {return;}
        match std::fs::read_to_string(room_file()) {
            Ok(text)=>match HostRoom::parse(&text) {
                Ok(room)=>{super::log(format!("LOBBY room file read: name={:?} max={} locked={} version={}",room.name,room.max,room.locked,room.version));
                    host.room=Some(room);host.stage=Stage::Starting{retry_at:Instant::now(),attempts:0};},
                Err(error)=>super::log(format!("LOBBY room file ignored: {error}")),
            },
            Err(_)=>{},
        }
    }
    let Some(room)=host.room.as_ref() else{return};
    let now=Instant::now();
    match host.stage {
        Stage::Unread|Stage::Off=>{},
        Stage::Starting{retry_at,attempts}=>{
            if now<retry_at {return;}
            if host.api.is_none() {
                match Api::load() {Ok(api)=>host.api=Some(api),Err(error)=>{
                    super::log_repeating(format!("LOBBY Steam unavailable: {error}"));
                    host.stage=Stage::Starting{retry_at:now+Duration::from_secs(10),attempts};return;
                }}
            }
            let api=host.api.as_ref().unwrap();
            let call=unsafe {(api.create)(api.matchmaking,PUBLIC,i32::from(room.max))};
            if call==0 {
                super::log("LOBBY CreateLobby returned no call handle");
                host.stage=if attempts>=3 {Stage::Off}else{Stage::Starting{retry_at:now+Duration::from_secs(15),attempts:attempts+1}};return;
            }
            host.stage=Stage::Creating{call,since:now,attempts};
        },
        Stage::Creating{call,since,attempts}=>{
            let api=host.api.as_ref().unwrap();
            let mut failed=false;
            if !unsafe {(api.completed)(api.utils,call,&mut failed)} {
                if now.duration_since(since)>Duration::from_secs(20) {
                    super::log("LOBBY CreateLobby timed out");
                    host.stage=if attempts>=3 {Stage::Off}else{Stage::Starting{retry_at:now+Duration::from_secs(15),attempts:attempts+1}};
                }
                return;
            }
            let mut created=LobbyCreated::default();
            let ok=unsafe {(api.result)(api.utils,call,(&mut created as *mut LobbyCreated).cast(),std::mem::size_of::<LobbyCreated>() as i32,LOBBY_CREATED,&mut failed)};
            // EResult 1 = k_EResultOK
            if !ok||failed||created.result!=1||created.lobby==0 {
                super::log(format!("LOBBY CreateLobby failed: ok={ok} failed={failed} result={}",created.result));
                host.stage=if attempts>=3 {Stage::Off}else{Stage::Starting{retry_at:now+Duration::from_secs(15),attempts:attempts+1}};
                return;
            }
            super::log(format!("LOBBY room open: lobby={:#x}",created.lobby));
            host.stage=Stage::Open{lobby:created.lobby,sent:vec![],at:now-Duration::from_secs(60)};
        },
        Stage::Open{lobby,ref mut sent,ref mut at}=>{
            if now.duration_since(*at)<Duration::from_secs(5) {return;}
            *at=now;
            let api=host.api.as_ref().unwrap();
            use eldenring::cs::{CSSessionManager,LobbyState};
            use fromsoftware_shared::FromStatic;
            let (players,open)=unsafe {CSSessionManager::instance()}.ok().map_or((1,false),|session|{
                let open=session.lobby_state==LobbyState::Host;
                ((if open {session.players.len()}else{1}).clamp(1,8) as u8,open)
            });
            let data=room.data(&api.persona(),players,state,open,started);
            for (key,value) in &data {
                if sent.iter().any(|(k,v)|k==key&&v==value) {continue;}
                if !api.set(lobby,key,value) {super::log_repeating(format!("LOBBY SetLobbyData failed: {key}"));return;}
            }
            *sent=data;
        },
    }
}
