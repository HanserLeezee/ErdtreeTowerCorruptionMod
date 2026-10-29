// 第三幕「双王共命」：双王组合（enemies::DUAL_PAIRS）同时上场。两只照常死亡（死亡动画、尸体），
// 一只死后 WINDOW 秒内另一只也死，战斗胜利；超时则在尸体的位置重新生成死掉的那只，生成后生命设为 REVIVE_PERCENT%。
// 重新生成之后规则照旧：谁先死，另一只就要在 WINDOW 秒内跟上。两只开场生命为 START_PERCENT%。
// 重新生成由运行时把那只敌人的战斗状态重置为“待生成”（runtime::poll_group 负责生成与绑定）。
// 联机用地图里的预置实体，死了不能重新生成：联机时不复活（相当于普通的双 Boss 战）。屏幕上不显示规则提示。
use std::time::{Duration,Instant};
use eldenring::cs::{ChrInsExt,FieldInsHandle,WorldChrMan};

pub const WINDOW:Duration=Duration::from_secs(25);
pub const REVIVE_PERCENT:i64=50;
pub const START_PERCENT:i64=70;
/// 开场生命在看到 Boss 1 秒后再调整（成长效果先挂上）
const SETTLE:Duration=Duration::from_secs(1);

#[derive(Default)]
struct Twin {name:&'static str,handle:Option<FieldInsHandle>,seen:Option<Instant>,started:bool,fell:Option<(Instant,[f32;3])>,reviving:bool}
#[derive(Default)]
pub struct Fight {key:Option<(u64,u8,usize)>,twins:[Twin;2],finished:bool,revives:u32}
/// 该复活的那只：另一只还活着、它已经死了 WINDOW 秒以上
fn due(fell:[Option<Instant>;2],alive:[bool;2],now:Instant)->Option<usize> {
    (0..2).find(|&i|alive[1-i]&&fell[i].is_some_and(|at|now.duration_since(at)>=WINDOW))
}
impl Fight {
    pub fn active(&self)->bool {self.key.is_some()&&!self.finished}
    /// foes：两只 Boss 的 (名字, 当前句柄)。返回需要在尸体位置重新生成的那只（下标与尸体在场景里的位置）
    pub fn tick(&mut self,world:&mut WorldChrMan,key:(u64,u8,usize),foes:[(&'static str,Option<FieldInsHandle>);2],authority:bool,revive:bool)->Option<(usize,[f32;3])> {
        if self.key!=Some(key) {
            *self=Self{key:Some(key),..Default::default()};
            for (twin,(name,_)) in self.twins.iter_mut().zip(foes) {twin.name=name;}
            super::log(format!("TOWER_DUAL begin node={} bosses={}、{}",key.2,foes[0].0,foes[1].0));
        }
        if self.finished {return None;}
        let now=Instant::now();
        let mut alive=[false;2];
        // 复生期间另一只若已倒下，它的倒计时从复生完成时重新算
        let mut restart=None;
        for (i,(_,handle)) in foes.into_iter().enumerate() {
            let twin=&mut self.twins[i];
            if handle!=twin.handle {twin.handle=handle;twin.seen=handle.map(|_|now);}
            let Some(chr)=handle.and_then(|h|world.chr_ins_by_handle_mut(&h)) else{continue};
            let (hp,max)=(chr.modules.data.hp,chr.modules.data.max_hp);
            if chr.chr_flags1c5.death_flag()||hp<=0 {
                if twin.fell.is_none() {
                    let p=chr.modules.physics.position;twin.fell=Some((now,[p.0,p.1,p.2]));
                    super::log(format!("TOWER_DUAL {} fell; partner has {} s",twin.name,WINDOW.as_secs()));
                }
                continue;
            }
            alive[i]=true;
            if twin.reviving {
                // 重新生成的那只：生命设为一半，挂上重生的光效
                if authority&&max>0 {chr.modules.data.hp=(i64::from(max)*REVIVE_PERCENT/100).max(1) as i32;chr.apply_speffect(super::boss_affixes::REBORN,false);}
                twin.reviving=false;twin.fell=None;twin.started=true;
                super::log(format!("TOWER_DUAL {} revived at {}/{max}",twin.name,chr.modules.data.hp));
                restart=Some(1-i);
            } else if twin.fell.is_some() {twin.fell=None;}
            if authority&&!twin.started&&twin.seen.is_some_and(|at|now.duration_since(at)>=SETTLE)&&max>0 {
                chr.modules.data.hp=hp.min((i64::from(max)*START_PERCENT/100) as i32).max(1);twin.started=true;
            }
        }
        if let Some(other)=restart && let Some((at,_))=&mut self.twins[other].fell {*at=now;}
        let fell=[self.twins[0].fell.map(|(t,_)|t),self.twins[1].fell.map(|(t,_)|t)];
        if fell[0].is_some()&&fell[1].is_some()&&!self.twins.iter().any(|t|t.reviving) {
            let gap=fell[0].unwrap().max(fell[1].unwrap())-fell[0].unwrap().min(fell[1].unwrap());
            super::log(format!("TOWER_DUAL both fell {:.1} s apart after {} revives",gap.as_secs_f32(),self.revives));
            self.finished=true;return None;
        }
        if !authority||!revive {return None;}
        let index=due(fell,alive,now)?;
        let twin=&mut self.twins[index];
        if twin.reviving {return None;}
        twin.reviving=true;self.revives+=1;
        let at=twin.fell.map(|(_,p)|p)?;
        super::log(format!("TOWER_DUAL {} rises again at {at:?}",twin.name));
        Some((index,at))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_dead_twin_rises_only_when_its_partner_outlives_the_window() {
        let now=std::time::Instant::now();let late=now+super::WINDOW;
        assert_eq!(super::due([Some(now),None],[false,true],late),Some(0));
        assert_eq!(super::due([None,Some(now)],[true,false],late),Some(1));
        assert_eq!(super::due([Some(now),None],[false,true],late-std::time::Duration::from_millis(1)),None,"still inside the window");
        assert_eq!(super::due([Some(now),Some(now)],[false,false],late),None,"both fell: victory");
        assert_eq!(super::due([None,None],[true,true],late),None);
    }
}

/// 卡关保险：Boss 房的 Boss 连续 STUCK_SECONDS 秒停在正好 1 点生命，视为被转阶段的锁血卡住（原版由地图事件接手，爬塔没有这些事件），
/// 直接让它死并记日志。正常战斗里生命正好停在 1 很少见。只由房主（或单人）执行
pub const STUCK_SECONDS:u64=10;
#[derive(Default)]
pub struct LockGuard {key:Option<(u64,u8,usize)>,since:Vec<(FieldInsHandle,Instant)>,released:Vec<FieldInsHandle>}
fn stuck(since:Option<Instant>,now:Instant)->bool {since.is_some_and(|at|now.duration_since(at)>=Duration::from_secs(STUCK_SECONDS))}
impl LockGuard {
    pub fn tick(&mut self,world:&mut WorldChrMan,key:(u64,u8,usize),bosses:&[(FieldInsHandle,&'static str)],authority:bool) {
        if self.key!=Some(key) {*self=Self{key:Some(key),..Default::default()};}
        if !authority {return;}
        let now=Instant::now();
        for (handle,name) in bosses {
            let Some(chr)=world.chr_ins_by_handle_mut(handle) else{continue};
            let locked=chr.modules.data.hp==1&&!chr.chr_flags1c5.death_flag();
            let index=self.since.iter().position(|(h,_)|h==handle);
            match (locked,index) {
                (false,Some(i))=>{self.since.remove(i);}
                (true,None)=>self.since.push((*handle,now)),
                (true,Some(i)) if stuck(Some(self.since[i].1),now)=>{
                    self.since.remove(i);
                    if self.released.contains(handle) {
                        // 设成 0 也被锁回 1：藏起来并标记死亡，让这场战斗按胜利结束
                        chr.debug_flags.set_character_disabled(true);chr.chr_flags1c5.set_death_flag(true);
                        super::log(format!("TOWER_BOSS_GUARD {name} still locked at 1 HP; hidden and counted as defeated"));
                    } else {
                        chr.modules.data.hp=0;self.released.push(*handle);
                        super::log(format!("TOWER_BOSS_GUARD {name} sat at 1 HP for {STUCK_SECONDS} s; released (likely a phase lock waiting for a map event)"));
                    }
                }
                _=>{}
            }
        }
    }
}
#[cfg(test)]
mod guard_tests {
    #[test]
    fn only_a_long_stay_at_one_hp_counts_as_stuck() {
        let now=std::time::Instant::now();
        assert!(!super::stuck(None,now));
        assert!(!super::stuck(Some(now),now+std::time::Duration::from_secs(super::STUCK_SECONDS-1)));
        assert!(super::stuck(Some(now),now+std::time::Duration::from_secs(super::STUCK_SECONDS)));
    }
}
