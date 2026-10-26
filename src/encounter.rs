#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observation {
    Alive,
    Dead,
    Missing,
    DifferentEntity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Waiting,
    Victory,
    PlayerDied,
    LostEntity,
    SpawnTimedOut,
}

#[derive(Clone, Debug)]
pub struct Encounter<H> {
    handle: Option<H>,
    terminal: Option<Outcome>,
}

impl<H: Copy + Eq> Default for Encounter<H> {
    fn default() -> Self {
        Self {
            handle: None,
            terminal: None,
        }
    }
}

impl<H: Copy + Eq> Encounter<H> {
    pub fn bind(&mut self, handle: H, observed_alive: bool) -> bool {
        if self.handle.is_some() || self.terminal.is_some() || !observed_alive {
            return false;
        }
        self.handle = Some(handle);
        true
    }

    pub fn handle(&self) -> Option<H> {
        self.handle
    }

    pub fn poll(
        &mut self,
        player_dead: bool,
        observation: Observation,
        spawn_expired: bool,
    ) -> Outcome {
        if let Some(terminal) = self.terminal {
            return terminal;
        }
        let outcome = if player_dead {
            Outcome::PlayerDied
        } else if self.handle.is_none() {
            if spawn_expired {
                Outcome::SpawnTimedOut
            } else {
                Outcome::Waiting
            }
        } else {
            match observation {
                Observation::Alive => Outcome::Waiting,
                Observation::Dead => Outcome::Victory,
                Observation::Missing | Observation::DifferentEntity => Outcome::LostEntity,
            }
        };
        if outcome != Outcome::Waiting {
            self.terminal = Some(outcome);
        }
        outcome
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {Weak,Strong,Boss}
/// 单人同时在场的敌人上限（Boss 组除外）
pub const SOLO_CAP:usize=2;
/// 联机与 Boss 组一起出；其余单人战斗同时在场最多 SOLO_CAP 只，普通战清完弱怪后才出强怪。
/// previous 为本组中排在 index 之前的敌人状态，未生成的敌人记为 Waiting。
pub fn may_spawn(networked:bool,monster_node:bool,roles:&[Role],index:usize,previous:&[Outcome])->bool {
    if networked||roles.iter().all(|r|*r==Role::Boss) {return true;}
    if previous.iter().filter(|o|**o!=Outcome::Victory).count()>=SOLO_CAP {return false;}
    !(monster_node&&roles[index]!=Role::Weak&&roles[..index].iter().zip(previous).any(|(r,o)|*r==Role::Weak&&*o!=Outcome::Victory))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_a_live_bound_enemy_before_victory() {
        let mut fight = Encounter::<u64>::default();
        assert_eq!(
            fight.poll(false, Observation::Dead, false),
            Outcome::Waiting
        );
        assert!(!fight.bind(7, false));
        assert!(fight.bind(7, true));
        assert_eq!(
            fight.poll(false, Observation::Dead, false),
            Outcome::Victory
        );
    }

    #[test]
    fn missing_or_replaced_enemies_are_not_kills() {
        for observation in [Observation::Missing, Observation::DifferentEntity] {
            let mut fight = Encounter::<u64>::default();
            assert!(fight.bind(7, true));
            assert_eq!(fight.poll(false, observation, false), Outcome::LostEntity);
            assert_eq!(
                fight.poll(false, Observation::Dead, false),
                Outcome::LostEntity
            );
        }
    }

    #[test]
    fn simultaneous_death_is_not_victory() {
        let mut fight = Encounter::<u64>::default();
        fight.bind(7, true);
        assert_eq!(
            fight.poll(true, Observation::Dead, false),
            Outcome::PlayerDied
        );
    }

    #[test]
    fn late_binding_after_timeout_is_rejected() {
        let mut fight = Encounter::<u64>::default();
        assert_eq!(
            fight.poll(false, Observation::Alive, true),
            Outcome::SpawnTimedOut
        );
        assert!(!fight.bind(7, true));
    }

    #[test]
    fn binding_cannot_be_replaced_and_timeout_does_not_end_active_fight() {
        let mut fight = Encounter::<u64>::default();
        assert!(fight.bind(7, true));
        assert!(!fight.bind(8, true));
        assert_eq!(fight.handle(), Some(7));
        assert_eq!(
            fight.poll(false, Observation::Alive, true),
            Outcome::Waiting
        );
    }

    #[test]
    fn solo_waves_cap_and_boss_groups() {
        use super::{Role::*,may_spawn,Outcome::*};
        let duo=[Boss,Boss];
        assert!(may_spawn(false,false,&duo,1,&[Waiting]),"boss components spawn together");
        let monsters=[Weak,Weak,Strong,Strong];
        assert!(may_spawn(false,true,&monsters,1,&[Waiting]));
        assert!(!may_spawn(false,true,&monsters,2,&[Waiting,Waiting]),"cap of two");
        assert!(!may_spawn(false,true,&monsters,2,&[Victory,Waiting]),"strong wave waits for weak enemies");
        assert!(may_spawn(false,true,&monsters,2,&[Victory,Victory]));
        assert!(may_spawn(false,true,&monsters,3,&[Victory,Victory,Waiting]),"strong enemies arrive together");
        let elite=[Strong,Strong,Weak,Weak];
        assert!(may_spawn(false,false,&elite,1,&[Waiting]),"elite and escorts together");
        assert!(!may_spawn(false,false,&elite,2,&[Waiting,Waiting]));
        assert!(may_spawn(false,false,&elite,2,&[Victory,Waiting]),"next enemy fills a freed slot");
        assert!(may_spawn(true,true,&monsters,3,&[Waiting,Waiting,Waiting]),"party keeps group spawning");
    }
}
