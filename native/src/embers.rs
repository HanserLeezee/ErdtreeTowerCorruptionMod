use std::time::{Duration,Instant};
use eldenring::cs::{ChrIns,ChrInsExt,FieldInsHandle,SoloParamRepository,SpEffectParam,WorldChrMan};
use fromsoftware_shared::FromStatic;
use erdtree_tower::{Phase,Run,embers::{self,Ember}};

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
fn player_ids()->impl Iterator<Item=i32> {[EXECUTION,REVENGE].into_iter().chain((1..=embers::FRENZY_STACKS).map(frenzy_effect))}
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
    if held(run).contains(&Ember::Execution) {want.push(EXECUTION);}
    if fight.revenge_active() {want.push(REVENGE);}
    if fight.frenzy>0 {want.push(frenzy_effect(fight.frenzy));}
    let present:Vec<_>=chr.special_effect.entries().map(|e|e.param_id).collect();
    for id in player_ids() {if present.contains(&id)&&!want.contains(&id){chr.remove_speffect(id);}}
    if want.is_empty()||!ready(&want) {return;}
    for id in want {if !present.contains(&id){chr.apply_speffect(id,false);}}
}
fn distance(a:[f32;3],b:[f32;3])->f32 {((a[0]-b[0]).powi(2)+(a[1]-b[1]).powi(2)+(a[2]-b[2]).powi(2)).sqrt()}
struct Foe {handle:FieldInsHandle,hp:i32,max:i32,pos:[f32;3],alive:bool,boss:bool,hit_at:Option<Instant>}
/// 一场战斗内的余烬状态：按敌人生命变化判断命中与倒下，按玩家生命变化判断受伤
#[derive(Default)]
pub struct Fight {
    key:Option<(u64,u8,usize)>,foes:Vec<Foe>,player_hp:Option<i32>,flasks:Option<u8>,
    lifesteal:i64,rose:bool,dragon_ready:Option<Instant>,revenge_until:Option<Instant>,pub frenzy:u8,feasts:u8,missing:bool,
    /// 上一帧在场的召唤物位置（灵骸迸裂在它消失时引爆）
    spirit:Option<[f32;3]>,
}
impl Fight {
    pub fn active(&self)->bool {self.key.is_some()}
    pub fn revenge_active(&self)->bool {self.revenge_until.is_some_and(|until|Instant::now()<until)}
    /// 返回 true 表示触发了噬神之蛇的饱食，需要补一次红瓶
    pub fn tick(&mut self,world:&mut WorldChrMan,key:(u64,u8,usize),held:&[Ember],foes:&[(FieldInsHandle,bool)],authority:bool,flasks:Option<u8>)->bool {
        if self.key!=Some(key) {*self=Self{key:Some(key),..Default::default()};}
        let drank=matches!((self.flasks,flasks),(Some(before),Some(after)) if after<before);self.flasks=flasks;
        if held.is_empty() {return false;}
        let has=|e:Ember|held.contains(&e);
        let now=Instant::now();
        let Some(player)=world.main_player.as_ref() else{return false};
        if player.chr_ins.modules.data.hp<=0||player.chr_ins.chr_flags1c5.death_flag() {return false;}
        let data=&player.chr_ins.modules.data;
        let (hp,max_hp,stamina,max_stamina)=(data.hp,data.max_hp,data.stamina,data.max_stamina);
        let p=player.chr_ins.modules.physics.position;let here=[p.0,p.1,p.2];
        let taken=self.player_hp.map_or(0,|before|(before-hp).max(0));self.player_hp=Some(hp);
        // 敌人：命中与倒下
        let (mut hits,mut deaths)=(Vec::new(),Vec::new());
        for (handle,boss) in foes {
            let Some(chr)=world.chr_ins_by_handle(handle) else{continue};
            let (foe_hp,foe_max)=(chr.modules.data.hp,chr.modules.data.max_hp);
            let q=chr.modules.physics.position;let pos=[q.0,q.1,q.2];
            let alive=foe_hp>0&&!chr.chr_flags1c5.death_flag();
            let Some(index)=self.foes.iter().position(|f|f.handle==*handle) else {
                self.foes.push(Foe{handle:*handle,hp:foe_hp,max:foe_max,pos,alive,boss:*boss,hit_at:None});continue;
            };
            let foe=&mut self.foes[index];
            let drop=foe.hp-foe_hp;
            if foe.alive&&!alive {deaths.push(index);}
            else if alive&&drop>0&&i64::from(drop)*1000>=i64::from(foe_max.max(1))*HIT_PERMILLE&&foe.hit_at.is_none_or(|t|now.duration_since(t)>=HIT_INTERVAL) {
                hits.push((index,drop));foe.hit_at=Some(now);
            }
            (foe.hp,foe.max,foe.pos,foe.alive)=(foe_hp,foe_max,pos,alive);
        }
        // 召唤物：只看本机玩家的骨灰
        let spirit=world.summon_buddy_chr_set.characters().find(|c|c.modules.data.hp>0&&!c.chr_flags1c5.death_flag())
            .map(|c|{let q=c.modules.physics.position;(c.modules.data.hp,c.modules.data.max_hp,[q.0,q.1,q.2])});
        let vanished=self.spirit.filter(|_|spirit.is_none());self.spirit=spirit.map(|(_,_,pos)|pos);
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
        let feast=has(Ember::Feast)&&!deaths.is_empty()&&self.feasts<embers::FEAST_PER_FIGHT;
        if feast {self.feasts+=1;}
        // 敌人侧：只由房主（或单人）计算
        if !authority {return feast;}
        let effects:Vec<i32>=[(Ember::Bloodlord,BLOOD),(Ember::Frost,FROST)].into_iter().filter(|(e,_)|has(*e)).map(|(_,id)|id).collect();
        if !ready(&[BLOOD,FROST,FLASH]) {if !self.missing{super::log("TOWER_EMBER missing ember effects; update regulation.bin");self.missing=true;}}
        else if !effects.is_empty() {
            for (index,_) in &hits {if let Some(chr)=world.chr_ins_by_handle_mut(&self.foes[*index].handle) {for id in &effects {chr.apply_speffect(*id,false);}}}
        }
        let mut wounds:Vec<(usize,i32)>=Vec::new();
        if has(Ember::Deathflame) {
            for &dead in &deaths {
                let origin=self.foes[dead].pos;
                for (index,foe) in self.foes.iter().enumerate() {
                    if foe.alive&&distance(foe.pos,origin)<=embers::DEATHFLAME_RADIUS {wounds.push((index,embers::enemy_damage(foe.max,embers::DEATHFLAME_PERCENT,foe.boss)));}
                }
            }
        }
        if has(Ember::Echo)&&drank {
            for (index,foe) in self.foes.iter().enumerate() {
                if foe.alive&&distance(foe.pos,here)<=embers::ECHO_RADIUS {wounds.push((index,embers::enemy_damage(foe.max,embers::ECHO_PERCENT,foe.boss)));}
            }
        }
        if has(Ember::SpiritBurst) && let Some(origin)=vanished {
            for (index,foe) in self.foes.iter().enumerate() {
                if foe.alive&&distance(foe.pos,origin)<=embers::SPIRIT_BURST_RADIUS {wounds.push((index,embers::enemy_damage(foe.max,embers::SPIRIT_BURST_PERCENT,foe.boss)));}
            }
            super::log("TOWER_EMBER spirit burst");
        }
        if has(Ember::Thorns)&&taken>0 {
            let nearest=self.foes.iter().enumerate().filter(|(_,f)|f.alive).min_by(|(_,a),(_,b)|distance(a.pos,here).total_cmp(&distance(b.pos,here)));
            if let Some((index,foe))=nearest {wounds.push((index,embers::thorns_damage(taken,foe.max,foe.boss)));}
        }
        for (index,damage) in wounds {
            if damage<=0 {continue;}
            let foe=&mut self.foes[index];
            let Some(chr)=world.chr_ins_by_handle_mut(&foe.handle) else{continue};
            if chr.modules.data.hp<=0 {continue;}
            // 不由余烬直接击杀，避免绕过游戏的死亡流程
            chr.modules.data.hp=(chr.modules.data.hp-damage).max(1);foe.hp=chr.modules.data.hp;
            if ready(&[FLASH]) {chr.apply_speffect(FLASH,false);}
        }
        feast
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn player_effects_cover_every_frenzy_stack() {
        let ids:Vec<_>=player_ids().collect();
        assert_eq!(ids.len(),2+usize::from(embers::FRENZY_STACKS));
        assert_eq!(frenzy_effect(embers::FRENZY_STACKS),FIRST+38);
        assert!(!ids.contains(&BLOOD)&&!ids.contains(&FLASH));
    }
}
