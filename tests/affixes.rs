use erdtree_tower::{affixes,Class,NodeKind,Phase,Reward,Run,rewards::{self,ItemKind}};

#[test]
fn generated_equipment_has_stable_identity_and_no_base_collision() {
    assert!(affixes::CATALOG.len()>3000);
    let mut ids=std::collections::HashSet::new();
    for item in affixes::CATALOG.iter() {
        assert!(ids.insert(item.id));assert!(!rewards::CATALOG.iter().any(|i|i.id==item.id));
        let base=affixes::base(item.id);
        assert!(rewards::lookup(base).is_some_and(|i|i.kind==item.kind));
        assert!(affixes::tier(item.id).is_some());
    }
}

#[test]
fn affixed_rewards_are_deterministic_claimable_and_survive_reload() {
    let mut seen_armor=false;
    for seed in 0..100 {
        let mut run=Run::new(seed,Class::Samurai);
        let node=run.start_nodes()[0];run.enter(node).unwrap();run.win_combat().unwrap();
        let Phase::Reward{ref offers,..}=run.phase else {panic!()};
        let raw=rewards::offers(&run,node);assert_eq!(offers.len(),raw.len());
        for (index,(shown,pool)) in offers.iter().zip(&raw).enumerate() {assert!(shown==pool||index==raw.len()-1&&matches!(shown,Reward::Ember(_)),"only the last slot may become an ember");}
        for reward in offers {if let Reward::Item(id)=reward {
            let kind=rewards::lookup(*id).unwrap().kind;
            if matches!(kind,ItemKind::Weapon|ItemKind::Armor){assert!(affixes::tier(*id).is_some());}
            seen_armor|=kind==ItemKind::Armor;
        }}
        assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
        run.claim_reward(0).unwrap();assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
    }
    assert!(seen_armor);
}

#[test]
fn old_run_without_affix_field_keeps_pending_rewards() {
    let mut run=Run::new(19,Class::Samurai);run.affix_equipment=false;
    let node=run.start_nodes()[0];run.enter(node).unwrap();run.win_combat().unwrap();
    let mut value:serde_json::Value=serde_json::from_str(&run.encode().unwrap()).unwrap();
    value.as_object_mut().unwrap().remove("affix_equipment");
    assert_eq!(Run::decode(&value.to_string()).unwrap(),run);
}

#[test]
fn boss_weapon_keeps_legendary_quality_and_base_pool_identity() {
    for seed in 0..60 {
        let run=Run::new(seed,Class::Samurai);
        let node=run.nodes.iter().position(|n|n.kind==NodeKind::Boss).unwrap();
        for r in rewards::offers(&run,node) {if let Reward::Item(id)=r {
            assert!(erdtree_tower::elite::LEGENDARY.contains(&affixes::base(id)));
            if id>>28==0 {assert_eq!(affixes::tier(id),Some(3));}
        }}
    }
}

#[test]
fn armor_types_are_independent_of_rarity_and_all_can_be_obtained() {
    let armor=rewards::CATALOG.iter().find(|i|i.kind==ItemKind::Armor).unwrap();
    let mut types=std::collections::HashSet::new();
    for seed in 0..1000 {
        let run=Run::new(seed,Class::Samurai);
        let id=affixes::roll(&run,armor.id,13,true).unwrap();
        assert_eq!(affixes::base(id),armor.id);
        types.insert(affixes::profile(id).unwrap());
    }
    assert_eq!(types.len(),4);
}

#[test]
fn previous_affix_journey_keeps_the_same_equipment_ids() {
    let mut run=Run::new(19,Class::Samurai);run.affix_rules=0;
    let node=run.start_nodes()[0];run.enter(node).unwrap();run.win_combat().unwrap();
    let mut value:serde_json::Value=serde_json::from_str(&run.encode().unwrap()).unwrap();
    value.as_object_mut().unwrap().remove("affix_rules");
    assert_eq!(Run::decode(&value.to_string()).unwrap(),run);
    let Phase::Reward{offers,..}=run.phase else {panic!()};
    for reward in offers {if let Reward::Item(id)=reward {assert!(affixes::profile(id).is_none());}}
}

#[test]
fn quality_follows_the_prefix_with_source_floors_and_old_runs_keep_vanilla_floors() {
    use erdtree_tower::NodeKind;
    // 亵渎圣剑：原版传说，规则 3 起也会以普通、稀有、史诗出现
    let mut tiers=std::collections::BTreeSet::new();
    for seed in 0..400 {let run=Run::new(seed,Class::Samurai);tiers.insert(affixes::tier(affixes::roll(&run,3140000,seed.wrapping_mul(7919)^0x51,true).unwrap()).unwrap());}
    assert_eq!(tiers.into_iter().collect::<Vec<_>>(),vec![0,1,2,3]);
    for seed in 0..60 {
        let run=Run::new(seed,Class::Samurai);
        for (id,node) in run.nodes.iter().enumerate() {
            let floor=match node.kind {NodeKind::Boss=>3,NodeKind::Elite=>2,_=>continue};
            for reward in rewards::offers(&run,id) {if let Reward::Item(item)=reward {if let Some(tier)=affixes::tier(item) {
                assert!(tier>=floor,"{:?} reward {item} tier {tier}",node.kind);
            }}}
        }
        assert!(affixes::tier(affixes::roll_min(&run,3140000,seed,true,3).unwrap())==Some(3));
        let mut old=run.clone();old.affix_rules=2;
        assert_eq!(affixes::tier(affixes::roll(&old,3140000,seed,true).unwrap()),Some(3),"old runs keep the vanilla rarity floor");
        assert_eq!(Run::decode(&run.encode().unwrap()).unwrap().affix_rules,4);
    }
}

#[test]
fn new_runs_use_the_relaxed_prefix_odds_and_old_runs_keep_theirs() {
    assert_eq!((0..100).map(affixes::prefix_tier).fold([0;4],|mut n,t|{n[usize::from(t)]+=1;n}),[25,35,28,12]);
    let count=|rules:u8|{
        let mut n=[0u32;4];
        for seed in 0..4000u64 {let mut run=Run::new(seed,Class::Samurai);run.affix_rules=rules;n[usize::from(affixes::tier(affixes::roll(&run,3140000,seed*7919^0x51,true).unwrap()).unwrap())]+=1;}
        n
    };
    let new=count(4);let old=count(3);
    for (tier,(share,tolerance)) in [(25,4),(35,4),(28,4),(12,3)].into_iter().enumerate() {assert!((new[tier] as i32*100/4000-share).abs()<=tolerance,"rules 4 tier {tier}: {new:?}");}
    for (tier,(share,tolerance)) in [(45,4),(30,4),(18,4),(7,3)].into_iter().enumerate() {assert!((old[tier] as i32*100/4000-share).abs()<=tolerance,"rules 3 tier {tier}: {old:?}");}
}
