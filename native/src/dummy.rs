// 篝火营地的测伤假人（只在单人时出现）：玩家进营地站稳后，在前方偏右处生成一个葛瑞克士兵，不攻击、不移动，挨打后生命立刻回满。
// 按它每帧掉的生命统计伤害（出血、中毒等状态伤害也算）：从第一击开始一轮，停手 4 秒后结算；万一一击打空血条，按满血计并在 1 秒后重新生成。
use eldenring::cs::{ChrDebugSpawnRequest,ChrLoadStatus,FieldInsHandle,NpcParam,NpcThinkParam,SoloParamRepository,WorldChrMan};
use fromsoftware_shared::{F32Vector4,FromStatic};
use std::time::{Duration,Instant};

/// 葛瑞克士兵（模型 4311）在爬塔里的敌人参数
const MODEL:i32=4311;
const NPC:i32=900000000+MODEL*10000;
const THINK:i32=MODEL*10000;
/// 停手多久结算一轮（秒）
const IDLE:f64=4.0;

/// 一轮的统计
#[derive(Clone,Copy,Debug,Default,PartialEq)]
pub struct Round {pub total:u64,pub hits:u32,pub seconds:f64,pub biggest:u32}
impl Round {
    /// 不到 1 秒按 1 秒算，避免只打一下时数字虚高
    pub fn dps(&self)->f64 {self.total as f64/self.seconds.max(1.0)}
}
/// 伤害计：当前一轮与上一轮的结果
#[derive(Clone,Copy,Debug,Default,PartialEq)]
pub struct Meter {current:Round,first:Option<f64>,last:Option<f64>,pub last_hit:u32,pub finished:Option<Round>}
impl Meter {
    pub fn hit(&mut self,damage:u32,now:f64) {
        if damage==0 {return;}
        if self.first.is_none() {self.current=Round::default();self.first=Some(now);}
        self.current.total+=u64::from(damage);self.current.hits+=1;self.current.biggest=self.current.biggest.max(damage);
        self.last_hit=damage;self.last=Some(now);
    }
    /// 停手超过 IDLE 秒时结算本轮（用时为第一击到最后一击）
    pub fn tick(&mut self,now:f64) {
        if let (Some(first),Some(last))=(self.first,self.last) && now-last>IDLE {
            self.finished=Some(Round{seconds:last-first,..self.current});
            self.first=None;self.last=None;
        }
    }
    /// 进行中的一轮（用时为第一击到现在）
    pub fn active(&self,now:f64)->Option<Round> {self.first.map(|first|Round{seconds:now-first,..self.current})}
}

/// 界面上显示的内容
#[derive(Clone,Copy,Debug,Default)]
pub struct View {pub active:Option<Round>,pub finished:Option<Round>,pub last_hit:u32,pub overflow:bool,pub ready:bool}

#[derive(Default)]
pub struct Dummy {
    bound:Option<(FieldInsHandle,usize)>,
    requested:Option<Instant>,
    respawn_at:Option<Instant>,
    failed:bool,
    last_hp:i32,
    overflow:bool,
    meter:Meter,
}
impl Dummy {
    pub fn view(&self,now:f64)->View {
        View{active:self.meter.active(now),finished:self.meter.finished,last_hit:self.meter.last_hit,overflow:self.overflow,ready:self.bound.is_some()}
    }
    pub fn tick(&mut self,world:&mut WorldChrMan,now:f64)->Result<(),String> {
        self.meter.tick(now);
        if let Some((handle,address))=self.bound {
            let Some(chr)=world.chr_ins_by_handle_mut(&handle).filter(|c|std::ptr::from_ref(*c) as usize==address) else {
                self.bound=None;return Ok(());
            };
            let (hp,max)=(chr.modules.data.hp,chr.modules.data.max_hp);
            if hp<self.last_hp {self.meter.hit((self.last_hp-hp.max(0)) as u32,now);}
            if hp<=0||chr.chr_flags1c5.death_flag() {
                // 一击打空血条：按满血计，撤下后重新生成
                self.overflow=true;
                chr.debug_flags.set_character_disabled(true);chr.debug_flags.set_force_unloaded(true);
                self.bound=None;self.respawn_at=Some(Instant::now()+Duration::from_secs(1));
                return Ok(());
            }
            chr.modules.data.hp=max;self.last_hp=max;
            chr.debug_flags.set_disabled_secondary_actions(true);
            chr.debug_flags.set_disabled_movement(true);
            return Ok(());
        }
        if let Some(at)=self.requested {
            if let Some(created)=world.debug_chr_creator.last_created_chr {
                let chr=unsafe {created.as_ref()};
                let (handle,address)=(chr.field_ins_handle,std::ptr::from_ref(chr) as usize);
                if chr.npc_param_id==NPC&&chr.modules.data.hp>0
                    &&matches!(unsafe {chr.chr_set_entry.as_ref()}.chr_load_status,ChrLoadStatus::Active|ChrLoadStatus::ReadyForActivation)
                    &&let Some(chr)=world.chr_ins_by_handle_mut(&handle).filter(|c|std::ptr::from_ref(*c) as usize==address) {
                    chr.debug_flags.set_disabled_secondary_actions(true);
                    chr.debug_flags.set_disabled_movement(true);
                    chr.modules.data.hp=chr.modules.data.max_hp;self.last_hp=chr.modules.data.max_hp;
                    self.bound=Some((handle,address));self.requested=None;world.debug_chr_creator.last_created_chr=None;
                    super::log(format!("Damage dummy bound: handle={handle}"));
                    return Ok(());
                }
            }
            if at.elapsed()>Duration::from_secs(15) {self.requested=None;self.failed=true;return Err("测伤假人生成超时".into());}
            return Ok(());
        }
        if self.failed||self.respawn_at.is_some_and(|at|Instant::now()<at)||world.debug_chr_creator.spawn {return Ok(());}
        let params=unsafe {SoloParamRepository::instance()}.map_err(|_|"敌人参数未加载")?;
        if params.get::<NpcParam>(NPC as u32).is_none()||params.get::<NpcThinkParam>(THINK as u32).is_none() {self.failed=true;return Err("缺少测伤假人的敌人参数".into());}
        let player=world.main_player.as_ref().ok_or("等待角色加载")?;
        let physics=&player.chr_ins.modules.physics;
        if !physics.standing_on_solid_ground||physics.is_falling {return Ok(());}
        // 放在玩家前方 5 米、偏右 2.5 米，面朝玩家（前方 = (−sin yaw, −cos yaw)）
        let me=[physics.position.0-player.block_position.x,physics.position.1-player.block_position.y,physics.position.2-player.block_position.z];
        let (sin,cos)=physics.orientation.to_euler_angles().1.sin_cos();
        let at=[me[0]-sin*5.0+cos*2.5,me[1]+0.1,me[2]-cos*5.0-sin*2.5];
        let yaw=super::enemies::facing(at,me);
        let origin=[physics.position.0-me[0],physics.position.1-me[1],physics.position.2-me[2]];
        let init=&mut world.debug_chr_creator.init_data;
        unsafe {
            let bytes=std::ptr::from_mut(init).cast::<u8>();
            bytes.add(0x10).cast::<F32Vector4>().write(F32Vector4(0.0,yaw,0.0,0.0));
            bytes.add(0xc8).write(0);
        }
        world.debug_chr_creator.last_created_chr=None;
        world.spawn_debug_character(&ChrDebugSpawnRequest{chr_id:MODEL,npc_param_id:NPC,npc_think_param_id:THINK,chara_init_param_id:0,event_entity_id:0,talk_id:-1,
            pos_x:origin[0]+at[0],pos_y:origin[1]+at[1],pos_z:origin[2]+at[2]});
        self.requested=Some(Instant::now());self.respawn_at=None;
        Ok(())
    }
    pub fn retire(&self,world:&mut WorldChrMan) {
        if self.requested.is_some()&&world.debug_chr_creator.init_data.npc_param_id==NPC {world.debug_chr_creator.spawn=false;}
        if let Some((handle,address))=self.bound
            &&let Some(chr)=world.chr_ins_by_handle_mut(&handle).filter(|c|std::ptr::from_ref(*c) as usize==address) {
            chr.debug_flags.set_character_disabled(true);chr.debug_flags.set_force_unloaded(true);
        }
        if self.requested.is_some()&&let Some(created)=world.debug_chr_creator.last_created_chr {
            let chr=unsafe {created.as_ref()};
            if chr.npc_param_id==NPC {
                let handle=chr.field_ins_handle;
                if let Some(chr)=world.chr_ins_by_handle_mut(&handle){chr.debug_flags.set_character_disabled(true);chr.debug_flags.set_force_unloaded(true);}
            }
        }
    }
}

/// 营地左侧的统计面板（画在前景层，不拦截输入）
pub fn draw_hud(ui:&hudhook::imgui::Ui,view:&View) {
    use crate::fancy_text::{Effect,Layer,Style,draw,measure};
    use hudhook::imgui::ImColor32;
    let size=ui.io().display_size;let s=(size[1]/1080.0).max(0.6);
    let (x,y)=(size[0]*0.025,size[1]*0.24);
    let (gold,white,dim,warn)=([232,196,120],[236,228,212],[160,150,132],[232,140,96]);
    let number=|v:u64|{let s=v.to_string();let mut out=String::new();for (i,c) in s.chars().enumerate(){if i>0&&(s.len()-i)%3==0{out.push(',');}out.push(c);}out};
    let mut lines:Vec<(String,[u8;3],f32)>=vec![("测伤假人".into(),gold,26.0)];
    if !view.ready {lines.push(("正在布置假人……".into(),dim,19.0));}
    match (view.active,view.finished) {
        (Some(round),_)=>{
            lines.push((format!("DPS  {:.1}",round.dps()),white,30.0));
            lines.push((format!("本轮 {} 伤害 · {} 击 · {:.1} 秒",number(round.total),round.hits,round.seconds),white,19.0));
            lines.push((format!("最近一击 {} · 最高 {}",number(u64::from(view.last_hit)),number(u64::from(round.biggest))),dim,19.0));
            lines.push(("停手 4 秒后结算本轮".into(),dim,17.0));
        },
        (None,Some(round))=>{
            lines.push((format!("上一轮 DPS  {:.1}",round.dps()),white,30.0));
            lines.push((format!("{} 伤害 · {} 击 · {:.1} 秒",number(round.total),round.hits,round.seconds.max(1.0)),white,19.0));
            lines.push((format!("最高一击 {}",number(u64::from(round.biggest))),dim,19.0));
            lines.push(("再次攻击开始新的一轮".into(),dim,17.0));
        },
        (None,None)=>lines.push(("攻击假人开始计算伤害".into(),dim,19.0)),
    }
    if view.overflow {lines.push(("有一击超过假人生命上限，按满血计".into(),warn,17.0));}
    let height:f32=lines.iter().map(|(_,_,k)|k*s+6.0*s).sum::<f32>()+20.0*s;
    let width=lines.iter().map(|(t,_,k)|measure(t,k*s)).fold(0.0f32,f32::max)+32.0*s;
    let back=ui.get_foreground_draw_list();
    back.add_rect([x,y],[x+width,y+height],ImColor32::from_rgba(10,9,8,170)).filled(true).rounding(6.0*s).build();
    back.add_rect([x,y],[x+width,y+height],ImColor32::from_rgba(180,150,90,120)).rounding(6.0*s).build();
    drop(back);
    let mut line_y=y+10.0*s;
    for (text,color,k) in &lines {
        draw(Layer::Foreground,[x+16.0*s,line_y],k*s,text,&Style{color:*color,accent:*color,effect:Effect::Plain,glow:false},0.0,1.0);
        line_y+=k*s+6.0*s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rounds_start_on_the_first_hit_and_settle_after_four_idle_seconds() {
        let mut meter=Meter::default();
        meter.hit(100,10.0);meter.hit(300,11.5);meter.hit(0,12.0);
        let round=meter.active(13.0).unwrap();
        assert_eq!((round.total,round.hits,round.biggest),(400,2,300));
        assert!((round.dps()-400.0/3.0).abs()<1e-9);
        meter.tick(15.0);assert!(meter.active(15.0).is_some()&&meter.finished.is_none());
        meter.tick(15.6);
        let done=meter.finished.unwrap();assert!(meter.active(16.0).is_none());
        assert_eq!((done.total,done.hits),(400,2));assert!((done.seconds-1.5).abs()<1e-9);
        assert!((done.dps()-400.0/1.5).abs()<1e-9);
        meter.hit(50,20.0);
        assert_eq!(meter.active(20.0).unwrap().total,50);assert_eq!(meter.finished.unwrap().total,400);
    }
    #[test]
    fn a_single_hit_counts_over_at_least_one_second() {
        let mut meter=Meter::default();meter.hit(900,0.0);meter.tick(5.0);
        assert_eq!(meter.finished.unwrap().dps(),900.0);
    }
}
