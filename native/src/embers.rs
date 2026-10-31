use std::time::{Duration,Instant};
use eldenring::{cs::{ChrIns,ChrInsExt,FieldInsHandle,SoloParamRepository,SpEffectParam,WorldChrMan},param::SP_EFFECT_PARAM_ST};
use fromsoftware_shared::FromStatic;
use erdtree_tower::{Phase,Run,embers::{self,Ember,Gain,Strike,StrikeRequest}};

/// 余烬效果：scene-builder ember-patch 写入的 9670000 段
pub const FIRST:i32=9670000;
const EXECUTION:i32=FIRST;
const BLOOD:i32=FIRST+10;
const FROST:i32=FIRST+11;
const REVENGE:i32=FIRST+20;
const FLASH:i32=FIRST+40;
/// 灵唤铃的号令：挂在召唤物身上
const SPIRIT_COMMAND:i32=FIRST+50;
fn frenzy_effect(stack:u8)->i32 {FIRST+30+i32::from(stack)}
/// 旧版本常驻效果只用于清理；原生处决由攻击表的 350501 事件识别。
const CHAIN_RESIDENT:i32=FIRST+5;
#[cfg(test)]
const CHAIN_MARK:i32=FIRST+7;
use std::sync::atomic::{AtomicUsize,AtomicBool,Ordering};
static APPLY_ORIGINAL:AtomicUsize=AtomicUsize::new(0);
static CHAIN_OWNER:AtomicUsize=AtomicUsize::new(0);
static CRITICAL_PENDING:AtomicBool=AtomicBool::new(false);
static CRITICAL_HOOK_READY:AtomicBool=AtomicBool::new(false);
fn own_critical(owner:usize,source:usize,effect:i32)->bool {owner!=0&&source==owner&&effect==350501}
unsafe extern "C" fn observe_native_critical(_target:usize,effect:i32,source:usize) {
    if own_critical(CHAIN_OWNER.load(Ordering::Relaxed),source,effect){CRITICAL_PENDING.store(true,Ordering::Release);}
}
#[unsafe(naked)]
unsafe extern "C" fn critical_entry() {
    core::arch::naked_asm!(
        "sub rsp, 0xc8", "mov [rsp+0x20], rax", "mov [rsp+0x28], rcx", "mov [rsp+0x30], rdx",
        "mov [rsp+0x38], r8", "mov [rsp+0x40], r9", "mov [rsp+0x48], r10", "mov [rsp+0x50], r11",
        "movdqu [rsp+0x60], xmm0", "movdqu [rsp+0x70], xmm1", "movdqu [rsp+0x80], xmm2",
        "movdqu [rsp+0x90], xmm3", "movdqu [rsp+0xa0], xmm4", "movdqu [rsp+0xb0], xmm5",
        "call {}", "movdqu xmm0, [rsp+0x60]", "movdqu xmm1, [rsp+0x70]", "movdqu xmm2, [rsp+0x80]",
        "movdqu xmm3, [rsp+0x90]", "movdqu xmm4, [rsp+0xa0]", "movdqu xmm5, [rsp+0xb0]",
        "mov rax, [rsp+0x20]", "mov rcx, [rsp+0x28]", "mov rdx, [rsp+0x30]", "mov r8, [rsp+0x38]",
        "mov r9, [rsp+0x40]", "mov r10, [rsp+0x48]", "mov r11, [rsp+0x50]", "add rsp, 0xc8",
        "jmp qword ptr [rip+{}]",sym observe_native_critical,sym APPLY_ORIGINAL);
}
pub unsafe fn install_critical_trace()->Result<(),String> {
    use hudhook::mh::{MhHook,MH_EnableHook};
    use std::ffi::c_void;
    let address=super::items::checked_function(0x3e8e70,&[0x48,0x8b,0xc4,0x57,0x41,0x54,0x41,0x55,0x41,0x56,0x41,0x57,0x48,0x83,0xec,0x50])? as *mut c_void;
    let hook=unsafe{MhHook::new(address,critical_entry as *mut c_void)}.map_err(|e|format!("Critical trace hook: {e:?}"))?;
    APPLY_ORIGINAL.store(hook.trampoline() as usize,Ordering::Release);
    unsafe{MH_EnableHook(address)}.ok().map_err(|e|format!("Critical trace enable: {e:?}"))?;
    CRITICAL_HOOK_READY.store(true,Ordering::Release);
    super::log("CHAIN_TEST native attack effect listener installed: 350501 / 3e8e70");Ok(())
}
#[derive(Clone,Copy,Default)]
pub struct ChainTrace {pub level:u32,pub resident:bool,pub marker:bool,pub rate:Option<f32>,pub critical_only:bool,pub effects:usize}
static CHAIN_TRACE:std::sync::Mutex<ChainTrace>=std::sync::Mutex::new(ChainTrace{level:0,resident:false,marker:false,rate:None,critical_only:false,effects:0});
static CRITICAL_EVENTS:std::sync::atomic::AtomicU32=std::sync::atomic::AtomicU32::new(0);
pub fn chain_trace()->(ChainTrace,u32) {(CHAIN_TRACE.lock().map(|t|*t).unwrap_or_default(),CRITICAL_EVENTS.load(std::sync::atomic::Ordering::Relaxed))}
pub fn begin_chain_test(){CRITICAL_EVENTS.store(0,std::sync::atomic::Ordering::Relaxed);CRITICAL_PENDING.store(false,Ordering::Release);}
fn observe_chain(chr:&ChrIns,level:u32) {
    let present:Vec<_>=chr.special_effect.entries().map(|e|e.param_id).collect();
    let active:Vec<_>=slots(Ember::Chain).unwrap().into_iter().filter(|id|present.contains(id)).collect();
    let row=unsafe{SoloParamRepository::instance()}.ok().and_then(|p|active.first().and_then(|id|p.get::<SpEffectParam>(*id as u32)));
    if let Ok(mut trace)=CHAIN_TRACE.lock(){*trace=ChainTrace{level,resident:CRITICAL_HOOK_READY.load(Ordering::Acquire),marker:CRITICAL_PENDING.load(Ordering::Acquire),
        rate:row.map(|r|r.physics_attack_rate()),critical_only:row.is_some_and(|r|r.state_info()==367&&r.throw_attack_param_change()),effects:active.len()};}
}
fn player_ids()->impl Iterator<Item=i32> {[EXECUTION,REVENGE,CHAIN_RESIDENT].into_iter().chain((1..=embers::FRENZY_STACKS).map(frenzy_effect))}
/// 叠层余烬不设上限：每种两行轮换（灵魂收容挂在召唤物身上，其余挂在玩家身上）。
/// 层数变了就把新倍率写进没挂着的那一行、换上它，再摘掉旧的那一行；不改正在生效的那一行
fn slots(ember:Ember)->Option<[i32;2]> {
    let base=match ember {Ember::Godslayer=>60,Ember::SoulEater=>70,Ember::Chain=>80,Ember::SpiritKeeper=>90,_=>return None};
    Some([FIRST+base,FIRST+base+1])
}
const PLAYER_STACKING:[Ember;3]=[Ember::Godslayer,Ember::SoulEater,Ember::Chain];
fn holds(row:&SP_EFFECT_PARAM_ST,ember:Ember,rates:embers::Rates)->bool {
    if ember==Ember::SpiritKeeper {row.physics_attack_power_rate()==rates.attack&&row.slash_damage_cut_rate()==rates.cut}
    else {row.physics_attack_rate()==rates.attack&&row.max_hp_rate()==rates.hp}
}
fn write(row:&mut SP_EFFECT_PARAM_ST,ember:Ember,rates:embers::Rates) {
    if ember==Ember::SpiritKeeper {
        let a=rates.attack;
        row.set_physics_attack_power_rate(a);row.set_magic_attack_power_rate(a);row.set_fire_attack_power_rate(a);row.set_thunder_attack_power_rate(a);row.set_dark_attack_power_rate(a);
        let c=rates.cut;
        row.set_slash_damage_cut_rate(c);row.set_blow_damage_cut_rate(c);row.set_thrust_damage_cut_rate(c);row.set_neutral_damage_cut_rate(c);
        row.set_magic_damage_cut_rate(c);row.set_fire_damage_cut_rate(c);row.set_thunder_damage_cut_rate(c);row.set_dark_damage_cut_rate(c);
    } else {
        let a=rates.attack;
        row.set_physics_attack_rate(a);row.set_magic_attack_rate(a);row.set_fire_attack_rate(a);row.set_thunder_attack_rate(a);row.set_dark_attack_rate(a);
        row.set_max_hp_rate(rates.hp);
    }
}
/// 这一层数该挂的那一行；present：目标角色身上已有的效果。None 表示不挂（没有层数、参数缺失，或两行都还挂着、等下一帧）
fn scaled_slot(ember:Ember,level:u32,present:&[i32])->Option<i32> {
    let (slots,rates)=(slots(ember)?,embers::stack_rates(ember,level)?);
    if level==0 {return None;}
    let params=unsafe {SoloParamRepository::instance_mut()}.ok()?;
    let fits=|id:i32,params:&SoloParamRepository|params.get::<SpEffectParam>(id as u32).is_some_and(|row|holds(row,ember,rates));
    if let Some(id)=slots.iter().copied().filter(|id|fits(*id,params)).max_by_key(|id|present.contains(id)) {return Some(id);}
    let free=slots.into_iter().find(|id|!present.contains(id))?;
    write(params.get_mut::<SpEffectParam>(free as u32)?,ember,rates);
    Some(free)
}
/// 按 scaled_slot 的结果在一个角色身上换行：先挂新的再摘旧的（屠神者摘掉最大生命倍率时不会先把当前生命压低）
fn swap<C:ChrInsExt+?Sized>(chr:&mut C,present:&[i32],ember:Ember,want:Option<i32>) {
    let Some(slots)=slots(ember) else{return};
    if let Some(id)=want&&!present.contains(&id) {chr.apply_speffect(id,false);}
    for id in slots {if present.contains(&id)&&Some(id)!=want {chr.remove_speffect(id);}}
}
/// 击倒时按血条等级记魂点：带血条的 Boss、精英（含 Boss 房的随从）、普通敌人
fn kill_rank(bar_rank:u8)->u8 {match bar_rank {0=>2,3=>0,_=>1}}
fn ready(ids:&[i32])->bool {unsafe {SoloParamRepository::instance()}.is_ok_and(|p|ids.iter().all(|id|p.get::<SpEffectParam>(*id as u32).is_some()))}
/// 低于敌人最大生命 0.3% 的掉血视为持续伤害，不算命中；同一敌人 0.3 秒内只算一次
const HIT_PERMILLE:i64=3;
const HIT_INTERVAL:Duration=Duration::from_millis(300);

pub fn held(run:Option<&Run>)->Vec<Ember> {
    run.filter(|r|!matches!(r.phase,Phase::Dead|Phase::Won)).map(|r|r.embers.clone()).unwrap_or_default()
}
/// 玩家身上的余烬效果：无声处刑常驻，以血还血与癫火狂热只在战斗中按状态挂上
pub fn sync(chr:&mut ChrIns,run:Option<&Run>,fight:&Fight) {
    let mut want=Vec::new();
    let embers=held(run);
    let owner=if embers.contains(&Ember::Chain){std::ptr::from_ref(chr) as usize}else{0};
    if CHAIN_OWNER.swap(owner,Ordering::AcqRel)!=owner{CRITICAL_PENDING.store(false,Ordering::Release);}
    if embers.contains(&Ember::Execution) {want.push(EXECUTION);}
    if fight.revenge_active() {want.push(REVENGE);}
    if fight.frenzy>0 {want.push(frenzy_effect(fight.frenzy));}
    let present:Vec<_>=chr.special_effect.entries().map(|e|e.param_id).collect();
    let stacks=run.map(|r|r.ember_stacks).unwrap_or_default();
    for ember in PLAYER_STACKING {
        let level=if embers.contains(&ember) {stacks.level(ember)}else{0};
        swap(chr,&present,ember,scaled_slot(ember,level,&present));
    }
    for id in player_ids() {if present.contains(&id)&&!want.contains(&id){chr.remove_speffect(id);}}
    if want.is_empty()||!ready(&want) {observe_chain(chr,stacks.level(Ember::Chain));return;}
    for id in want {if !present.contains(&id){chr.apply_speffect(id,false);}}
    observe_chain(chr,stacks.level(Ember::Chain));
}
fn distance(a:[f32;3],b:[f32;3])->f32 {((a[0]-b[0]).powi(2)+(a[1]-b[1]).powi(2)+(a[2]-b[2]).powi(2)).sqrt()}
/// slot：这场战斗敌人名单里的位置（联机时房主与成员一致，成员发来的敌人侧效果按它找敌人）
struct Foe {handle:FieldInsHandle,hp:i32,max:i32,pos:[f32;3],alive:bool,boss:bool,rank:u8,slot:u8,hit_at:Option<Instant>}
/// 一帧的结果：噬神之蛇的饱食是否要补红瓶、让叠层余烬加层的事件、联机成员要请房主执行的敌人侧效果
#[derive(Default)]
pub struct Events {pub feast:bool,pub gains:Vec<Gain>,pub strikes:Vec<Strike>}
/// 本机玩家与本机召唤物：联机时只算他们的命中与击倒（看敌人的 last_hit_by）
fn own_handles(world:&WorldChrMan)->Vec<FieldInsHandle> {
    world.main_player.as_ref().map(|p|p.chr_ins.field_ins_handle).into_iter().chain(world.summon_buddy_chr_set.characters().map(|c|c.field_ins_handle)).collect()
}
/// 一场战斗内的余烬状态：按敌人生命变化判断命中与倒下，按玩家生命变化判断受伤
#[derive(Default)]
pub struct Fight {
    key:Option<(u64,u8,usize)>,foes:Vec<Foe>,player_hp:Option<i32>,flasks:Option<u8>,
    lifesteal:i64,rose:bool,dragon_ready:Option<Instant>,revenge_until:Option<Instant>,pub frenzy:u8,feasts:u8,missing:bool,
    /// 上一帧在场的召唤物位置（灵骸迸裂在它消失时引爆）
    spirit:Option<[f32;3]>,
    /// 上一帧玩家身上有处刑之链的致命一击标记
    critical_edge:CriticalEdge,
    /// 房主：每个成员已执行到的敌人侧效果序号
    remote:Vec<(u64,u32)>,
    /// 联机时这场战斗是否已记过第一次“自己的命中”与“别人的命中”（日志，用来核对 last_hit_by 的归属）
    attributed:[bool;2],
}
impl Fight {
    pub fn active(&self)->bool {self.key.is_some()}
    pub fn revenge_active(&self)->bool {self.revenge_until.is_some_and(|until|Instant::now()<until)}
    /// foes：(句柄, 血条等级 enemies::bar_rank, 名单位置)；authority：房主或单人（敌人侧效果直接执行，否则交给房主）；
    /// party：联机（命中与击倒只算本机玩家与召唤物的）；keeper：灵魂收容当前层数
    pub fn tick(&mut self,world:&mut WorldChrMan,key:(u64,u8,usize),held:&[Ember],foes:&[(FieldInsHandle,u8,u8)],authority:bool,party:bool,flasks:Option<u8>,keeper:u32)->Events {
        if self.key!=Some(key) {*self=Self{key:Some(key),..Default::default()};}
        let drank=matches!((self.flasks,flasks),(Some(before),Some(after)) if after<before);self.flasks=flasks;
        let mut events=Events::default();
        if held.is_empty() {return events;}
        let has=|e:Ember|held.contains(&e);
        let now=Instant::now();
        let Some(player)=world.main_player.as_ref() else{return events};
        if player.chr_ins.modules.data.hp<=0||player.chr_ins.chr_flags1c5.death_flag() {return events;}
        if has(Ember::Chain) {
            let marked=CRITICAL_PENDING.swap(false,Ordering::AcqRel);
            let throwing=unsafe{std::ptr::addr_of!(player.chr_ins.modules.throw.throw_node.throw_state).cast::<u32>().read()}==3;
            if self.critical_edge.observe(marked,throwing) {
                events.gains.push(Gain::Critical);
                let count=CRITICAL_EVENTS.fetch_add(1,std::sync::atomic::Ordering::Relaxed)+1;
                super::log(format!("CHAIN_TEST real critical marker observed count={count}"));
            }
        }
        let data=&player.chr_ins.modules.data;
        let (hp,max_hp,stamina,max_stamina)=(data.hp,data.max_hp,data.stamina,data.max_stamina);
        let p=player.chr_ins.modules.physics.position;let here=[p.0,p.1,p.2];
        let taken=self.player_hp.map_or(0,|before|(before-hp).max(0));self.player_hp=Some(hp);
        // 敌人：命中与倒下。单人时都算自己的；联机时按敌人的 last_hit_by 只算本机玩家与召唤物的（队友的不算进自己的余烬）
        let own=own_handles(world);
        let (mut hits,mut deaths)=(Vec::new(),Vec::new());
        for &(ref handle,rank,slot) in foes {
            let Some(chr)=world.chr_ins_by_handle(handle) else{continue};
            let (foe_hp,foe_max)=(chr.modules.data.hp,chr.modules.data.max_hp);
            let q=chr.modules.physics.position;let pos=[q.0,q.1,q.2];
            let alive=foe_hp>0&&!chr.chr_flags1c5.death_flag();
            let attacker=chr.last_hit_by;let mine=!party||own.contains(&attacker);
            let Some(index)=self.foes.iter().position(|f|f.handle==*handle) else {
                self.foes.push(Foe{handle:*handle,hp:foe_hp,max:foe_max,pos,alive,boss:rank<=1,rank,slot,hit_at:None});continue;
            };
            let foe=&mut self.foes[index];
            let drop=foe.hp-foe_hp;
            let died=foe.alive&&!alive;
            let hit=!died&&alive&&drop>0&&i64::from(drop)*1000>=i64::from(foe_max.max(1))*HIT_PERMILLE&&foe.hit_at.is_none_or(|t|now.duration_since(t)>=HIT_INTERVAL);
            if hit {foe.hit_at=Some(now);}
            (foe.hp,foe.max,foe.pos,foe.alive)=(foe_hp,foe_max,pos,alive);
            // 联机：每场战斗记一次“算进自己的”与“不算的”命中或击倒，实测时据此确认 last_hit_by 的归属
            if party&&(hit||died)&&!self.attributed[usize::from(!mine)] {
                self.attributed[usize::from(!mine)]=true;
                super::log(format!("TOWER_EMBER party {} by {attacker:?} counted={mine}",if died{"kill"}else{"hit"}));
            }
            if !mine {continue;}
            if died {deaths.push(index);} else if hit {hits.push((index,drop));}
        }
        // 召唤物：只看本机玩家的骨灰
        let spirit=world.summon_buddy_chr_set.characters().find(|c|c.modules.data.hp>0&&!c.chr_flags1c5.death_flag())
            .map(|c|{let q=c.modules.physics.position;(c.modules.data.hp,c.modules.data.max_hp,[q.0,q.1,q.2])});
        let vanished=self.spirit.filter(|_|spirit.is_none());self.spirit=spirit.map(|(_,_,pos)|pos);
        for &dead in &deaths {events.gains.push(Gain::Kill{rank:kill_rank(self.foes[dead].rank),spirit:spirit.is_some()||vanished.is_some()});}
        // 灵魂收容：每只在场的召唤物挂上当前层数的效果
        if has(Ember::SpiritKeeper) {
            let living=|c:&&mut _|{let c:&ChrIns=c;c.modules.data.hp>0&&!c.chr_flags1c5.death_flag()};
            let present:Vec<i32>=world.summon_buddy_chr_set.characters().filter(living).flat_map(|c|c.special_effect.entries().map(|e|e.param_id).collect::<Vec<_>>()).collect();
            let want=scaled_slot(Ember::SpiritKeeper,keeper,&present);
            for chr in world.summon_buddy_chr_set.characters().filter(living) {
                let own:Vec<_>=chr.special_effect.entries().map(|e|e.param_id).collect();
                swap(chr,&own,Ember::SpiritKeeper,want);
            }
        }
        // 玩家侧：所有玩家都生效
        let mut heal=0i64;
        // 仿身泪滴的替身：受到的伤害一部分转给召唤物（召唤物至少留 1 点生命，不由余烬直接打倒）
        let mut spirit_loss=0i32;
        if has(Ember::SpiritLink)&&taken>0 && let Some((spirit_hp,_,_))=spirit {
            spirit_loss=((i64::from(taken)*i64::from(embers::SPIRIT_SHARE_PERCENT)/100) as i32).min(spirit_hp-1).max(0);
            heal+=i64::from(spirit_loss);
        }
        if has(Ember::Miquella) {
            self.lifesteal+=hits.iter().map(|(_,d)|i64::from(*d)).sum::<i64>()*i64::from(embers::LIFESTEAL_PERCENT);
            heal+=self.lifesteal/100;self.lifesteal%=100;
        }
        if has(Ember::Frenzy) {
            if taken>0 {self.frenzy=0;} else {self.frenzy=(self.frenzy as usize+hits.len()).min(usize::from(embers::FRENZY_STACKS)) as u8;}
        }
        if has(Ember::Revenge) {
            if !hits.is_empty() {self.revenge_until=None;}
            if taken>0 {self.revenge_until=Some(now+Duration::from_secs(embers::REVENGE_SECONDS));}
        }
        let rise=has(Ember::Rise)&&!self.rose&&i64::from(hp)*100<i64::from(max_hp)*i64::from(embers::RISE_BELOW_PERCENT);
        let dragon=has(Ember::DragonHeart)&&stamina<=0&&max_stamina>0&&self.dragon_ready.is_none_or(|t|now>=t);
        if heal>0||rise||dragon {
            let data=&mut world.main_player.as_mut().unwrap().chr_ins.modules.data;
            if rise {
                data.hp=data.hp.max((i64::from(data.max_hp)*i64::from(embers::RISE_TO_PERCENT)/100) as i32);self.rose=true;
                super::log(format!("TOWER_EMBER rise hp={}/{}",data.hp,data.max_hp));
            }
            if heal>0 {data.hp=(i64::from(data.hp)+heal).min(i64::from(data.max_hp)) as i32;}
            if dragon {
                data.stamina=(i64::from(data.stamina.max(0))+i64::from(data.max_stamina)*i64::from(embers::DRAGON_RESTORE_PERCENT)/100).min(i64::from(data.max_stamina)) as i32;
                self.dragon_ready=Some(now+Duration::from_secs(embers::DRAGON_COOLDOWN_SECONDS));
            }
            self.player_hp=Some(data.hp);
        }
        // 众灵的盛宴、灵唤铃的号令、替身的分担：都作用在本机召唤物上
        if spirit.is_some()&&(spirit_loss>0||has(Ember::SpiritCommand)||has(Ember::SpiritFeast)&&!deaths.is_empty()) {
            let feast_heal=if has(Ember::SpiritFeast) {deaths.len() as i64}else{0};
            if let Some(chr)=world.summon_buddy_chr_set.characters().find(|c|c.modules.data.hp>0&&!c.chr_flags1c5.death_flag()) {
                let data=&mut chr.modules.data;
                let restored=i64::from(data.max_hp)*i64::from(embers::SPIRIT_FEAST_HEAL_PERCENT)/100*feast_heal;
                data.hp=(i64::from(data.hp)-i64::from(spirit_loss)+restored).clamp(1,i64::from(data.max_hp)) as i32;
                if has(Ember::SpiritCommand)&&ready(&[SPIRIT_COMMAND])&&!chr.special_effect.entries().any(|e|e.param_id==SPIRIT_COMMAND) {chr.apply_speffect(SPIRIT_COMMAND,false);}
            }
            if feast_heal>0 {
                let data=&mut world.main_player.as_mut().unwrap().chr_ins.modules.data;
                data.fp=(i64::from(data.fp.max(0))+i64::from(data.max_fp)*i64::from(embers::SPIRIT_FEAST_FP_PERCENT)/100*feast_heal).min(i64::from(data.max_fp)) as i32;
            }
        }
        events.feast=has(Ember::Feast)&&!deaths.is_empty()&&self.feasts<embers::FEAST_PER_FIGHT;
        if events.feast {self.feasts+=1;}
        // 敌人侧：房主与单人直接执行；联机成员算出来交给房主（命中、击倒都只算自己的）
        let mut strikes:Vec<(usize,Strike)>=Vec::new();
        for &(index,_) in &hits {
            for ember in [Ember::Bloodlord,Ember::Frost] {if has(ember) {strikes.push((index,Strike::Buildup{slot:self.foes[index].slot,ember}));}}
        }
        let mut wound=|origin:[f32;3],radius:f32,ember:Ember,foes:&[Foe]| {
            for (index,foe) in foes.iter().enumerate() {if foe.alive&&distance(foe.pos,origin)<=radius {strikes.push((index,Strike::Wound{slot:foe.slot,ember}));}}
        };
        if has(Ember::Deathflame) {for &dead in &deaths {wound(self.foes[dead].pos,embers::DEATHFLAME_RADIUS,Ember::Deathflame,&self.foes);}}
        if has(Ember::Echo)&&drank {wound(here,embers::ECHO_RADIUS,Ember::Echo,&self.foes);}
        if has(Ember::SpiritBurst) && let Some(origin)=vanished {wound(origin,embers::SPIRIT_BURST_RADIUS,Ember::SpiritBurst,&self.foes);super::log("TOWER_EMBER spirit burst");}
        if has(Ember::Thorns)&&taken>0 {
            let nearest=self.foes.iter().enumerate().filter(|(_,f)|f.alive).min_by(|(_,a),(_,b)|distance(a.pos,here).total_cmp(&distance(b.pos,here)));
            if let Some((index,foe))=nearest {strikes.push((index,Strike::Thorns{slot:foe.slot,taken:taken.min(embers::THORNS_TAKEN_LIMIT)}));}
        }
        if authority {for (index,strike) in strikes {self.strike(world,index,strike);}}
        else {events.strikes=strikes.into_iter().map(|(_,strike)|strike).collect();}
        events
    }
    /// 执行一条敌人侧效果。不由余烬直接击杀（至少留 1 点生命），避免绕过游戏的死亡流程
    fn strike(&mut self,world:&mut WorldChrMan,index:usize,strike:Strike) {
        if !ready(&[BLOOD,FROST,FLASH]) {if !self.missing{super::log("TOWER_EMBER missing ember effects; update regulation.bin");self.missing=true;}return;}
        let foe=&mut self.foes[index];
        let Some(chr)=world.chr_ins_by_handle_mut(&foe.handle) else{return};
        if chr.modules.data.hp<=0||chr.chr_flags1c5.death_flag() {return;}
        let max=chr.modules.data.max_hp;
        let damage=match strike {
            Strike::Buildup{ember,..}=>{chr.apply_speffect(if ember==Ember::Bloodlord{BLOOD}else{FROST},false);return;},
            Strike::Wound{ember,..}=>embers::enemy_damage(max,embers::wound_percent(ember).unwrap_or(0),foe.boss),
            Strike::Thorns{taken,..}=>embers::thorns_damage(taken,max,foe.boss),
        };
        if damage<=0 {return;}
        chr.modules.data.hp=(chr.modules.data.hp-damage).max(1);foe.hp=chr.modules.data.hp;
        chr.apply_speffect(FLASH,false);
    }
    /// 房主：执行成员发来的敌人侧效果。每个成员按序号只执行一次；成员没带对应的余烬、或找不到那个敌人的跳过
    pub fn apply_remote(&mut self,world:&mut WorldChrMan,foes:&[(FieldInsHandle,u8,u8)],member:u64,held:&[Ember],requests:&[StrikeRequest]) {
        let entry=match self.remote.iter().position(|(m,_)|*m==member) {Some(i)=>i,None=>{self.remote.push((member,0));self.remote.len()-1}};
        let last=self.remote[entry].1;
        let mut fresh:Vec<_>=requests.iter().filter(|r|r.seq>last).copied().collect();
        fresh.sort_by_key(|r|r.seq);
        let Some(newest)=fresh.last().map(|r|r.seq) else{return};
        self.remote[entry].1=newest;
        for request in fresh {
            if !request.strike.valid(held) {super::log(format!("TOWER_EMBER member {member} strike rejected: {:?}",request.strike));continue;}
            let Some(&(handle,rank,slot))=foes.iter().find(|f|f.2==request.strike.slot()) else{continue};
            let index=match self.foes.iter().position(|f|f.handle==handle) {
                Some(index)=>index,
                None=>{
                    let Some(chr)=world.chr_ins_by_handle(&handle) else{continue};
                    let q=chr.modules.physics.position;
                    let alive=chr.modules.data.hp>0&&!chr.chr_flags1c5.death_flag();
                    self.foes.push(Foe{handle,hp:chr.modules.data.hp,max:chr.modules.data.max_hp,pos:[q.0,q.1,q.2],alive,boss:rank<=1,rank,slot,hit_at:None});
                    self.foes.len()-1
                },
            };
            self.strike(world,index,request.strike);
        }
    }
}
#[derive(Default)]
struct CriticalEdge {counted:bool,clear_frames:u8}
impl CriticalEdge {
    fn observe(&mut self,marked:bool,throwing:bool)->bool {
        if marked||throwing {self.clear_frames=0;}else{self.clear_frames=self.clear_frames.saturating_add(1);if self.clear_frames>=2{self.counted=false;}}
        if marked&&!self.counted {self.counted=true;true}else{false}
    }
}
#[test]
fn critical_marker_counts_once_per_execution_not_per_damage_stage() {
    let mut edge=CriticalEdge::default();
    assert!(!edge.observe(false,false));assert!(!edge.observe(false,true));
    assert!(edge.observe(true,true));assert!(!edge.observe(true,true));
    assert!(!edge.observe(false,true));assert!(!edge.observe(true,true));
    assert!(!edge.observe(true,false));assert!(!edge.observe(false,false));assert!(!edge.observe(false,false));
    assert!(edge.observe(true,true));
}
#[test]
fn critical_listener_rejects_other_players_and_noncritical_effects() {
    assert!(own_critical(123,123,350501));
    assert!(!own_critical(123,456,350501));assert!(!own_critical(0,0,350501));
    for effect in [350500,350502,9670007,0,-1]{assert!(!own_critical(123,123,effect));}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effect_ids_do_not_overlap() {
        let ids:Vec<_>=player_ids().collect();
        assert_eq!(ids.len(),3+usize::from(embers::FRENZY_STACKS));
        assert_eq!(frenzy_effect(embers::FRENZY_STACKS),FIRST+38);
        let mut all=ids.clone();all.extend([BLOOD,FROST,FLASH,SPIRIT_COMMAND,CHAIN_MARK,CHAIN_RESIDENT+1,EXECUTION+1,EXECUTION+2]);
        all.extend(Ember::ALL.into_iter().filter_map(slots).flatten());
        let count=all.len();all.sort();all.dedup();assert_eq!(all.len(),count,"effect ids overlap");
        assert!(all.iter().all(|id|(FIRST..FIRST+100).contains(id)),"inside the ember range");
        assert!(Ember::ALL.into_iter().all(|e|e.stacking()==slots(e).is_some()));
        assert!(PLAYER_STACKING.iter().all(|e|e.stacking()&&*e!=Ember::SpiritKeeper));
        assert_eq!([0,1,2,3].map(kill_rank),[2,1,1,0]);
    }
}
