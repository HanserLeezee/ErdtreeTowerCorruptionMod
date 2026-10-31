use erdtree_tower::{Class,Run,legacy::Legacy,weapon_training::{Training,Preference,pick}};

#[test]
fn training_spends_points_without_removing_class_experience() {
    let mut legacy=Legacy::default();
    legacy.get_mut(Class::Hero).xp=300;
    legacy.training.choose(Some(9)).unwrap();
    legacy.upgrade_training(9).unwrap();
    assert_eq!(legacy.training.level_for(9),1);
    assert_eq!(legacy.training_balance(),200);
    legacy.upgrade_training(9).unwrap();
    let before=legacy.clone();
    assert!(legacy.upgrade_training(9).is_err());
    assert_eq!(legacy,before);
    assert_eq!(legacy.get(Class::Hero).xp,300);
    assert_eq!(Legacy::decode(&legacy.encode()),legacy);
    assert_eq!(Legacy::decode(r#"{"classes":[["Hero",{"xp":300,"talisman":0}]]}"#).training,Training::default());
}

#[test]
fn preference_is_optional_capped_and_preserved_by_run_saves() {
    let mut training=Training::default();
    assert!(training.choose(Some(99)).is_err());
    training.choose(Some(67)).unwrap();
    assert!(training.preferences().is_empty());
    let mut legacy=Legacy::default();legacy.get_mut(Class::Hero).xp=5000;
    for _ in 0..5 {legacy.upgrade_training(67).unwrap();}
    assert!(legacy.upgrade_training(67).is_err());
    let mut run=Run::new(42,Class::Hero);run.weapon_preferences=legacy.training.preferences();
    assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
    run.weapon_preference=Some(Preference{kind:67,level:6});
    assert!(Run::decode(&run.encode().unwrap()).is_err());
    legacy.training.choose(None).unwrap();assert!(legacy.training.preferences().is_empty());
    assert_eq!(legacy.training.level_for(67),5);
}

#[test]
fn multiple_types_have_independent_levels_spending_and_toggles() {
    let mut legacy=Legacy::default();legacy.get_mut(Class::Hero).xp=1000;
    legacy.upgrade_training(3).unwrap();legacy.upgrade_training(3).unwrap();legacy.upgrade_training(9).unwrap();
    assert_eq!(legacy.training_balance(),600);
    assert_eq!(legacy.training.preferences(),vec![Preference{kind:3,level:2},Preference{kind:9,level:1}]);
    legacy.training.choose(Some(3)).unwrap();
    assert_eq!(legacy.training.preferences(),vec![Preference{kind:9,level:1}]);
    assert_eq!(legacy.training_balance(),600);
    legacy.training.choose(Some(3)).unwrap();
    assert_eq!(legacy.training.level_for(3),2);
    assert_eq!(Legacy::decode(&legacy.encode()),legacy);
    let prefs=vec![Preference{kind:3,level:5},Preference{kind:9,level:5}];
    let mut counts=[0;3];
    for roll in 0..1100 {counts[erdtree_tower::weapon_training::pick_many(&[3000000,9000000,33000000],roll,&prefs,None)]+=1;}
    assert_eq!(counts,[500,500,100]);
    let mut run=Run::new(42,Class::Hero);run.weapon_preferences=prefs;
    assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
    run.weapon_preferences.push(Preference{kind:3,level:1});assert!(run.validate().is_err());
}

#[test]
fn old_single_training_migrates_once_without_charging_again() {
    let old=r#"{"classes":[["Hero",{"xp":1000}]],"training":{"selected":9,"level":3}}"#;
    let legacy=Legacy::decode(old);
    assert_eq!(legacy.training.level_for(9),3);assert_eq!(legacy.training_balance(),350);
    assert_eq!(Legacy::decode(&legacy.encode()),legacy);
    let disabled=Legacy::decode(&old.replace("\"selected\":9","\"selected\":null"));
    assert_eq!(disabled.training_balance(),1000);
}

#[test]
fn full_training_has_fivefold_relative_weight_without_excluding_other_weapons() {
    let ids=[1000000,9000000,33000000];
    let pref=Preference{kind:9,level:5};
    let mut counts=[0;3];
    for roll in 0..700 {counts[pick(&ids,roll,pref,None)]+=1;}
    assert_eq!(counts,[100,500,100]);
    assert_eq!((1..=5).map(|level|Preference{kind:9,level}.weight(9000000)).collect::<Vec<_>>(),vec![180,260,340,420,500]);
    for class in Class::PLAYABLE {
        let mut counts=[0;3];
        for seed in 0..30000 {counts[pick(&ids,seed,pref,Some(class))]+=1;}
        assert!(counts.iter().all(|n|*n>0));
    }
}

#[test]
fn personal_runs_preserve_preferences_through_party_roundtrip() {
    use erdtree_tower::party_rewards::PartyRewards;
    let mut host=Run::new_personal(42,Class::Hero,1);
    let mut guest=Run::new_personal(42,Class::Hero,2);
    host.weapon_preferences=vec![Preference{kind:3,level:5},Preference{kind:9,level:2}];
    guest.weapon_preferences=vec![Preference{kind:9,level:3},Preference{kind:33,level:1}];
    let party=PartyRewards::new(1,10,vec![(10,host.clone()),(20,guest.clone())]).unwrap();
    assert_eq!(party.run(10).unwrap().preferences(),host.preferences());
    assert_eq!(party.run(20).unwrap().preferences(),guest.preferences());
    assert_eq!(Run::decode(&guest.encode().unwrap()).unwrap().preferences(),guest.preferences());
}

#[test]
fn catalog_types_are_covered_and_bonus_pools_keep_category_and_quality() {
    use erdtree_tower::{rewards::{CATALOG,ItemKind},weapon_training::{known,reward}};
    for item in CATALOG.iter().filter(|i|i.kind==ItemKind::Weapon) {assert!(known((item.id/1_000_000) as u8),"{}",item.name);}
    let mut run=Run::new(42,Class::Hero);run.weapon_preference=Some(Preference{kind:9,level:5});
    let ids=[9000000,1000000,2140000,3090000,0x40001068];
    for roll in 0..1000 {
        let old=ids[roll as usize%ids.len()];let new=reward(&run,&ids,roll);
        assert_eq!(old>>28,new>>28);
        assert_eq!(erdtree_tower::elite::LEGENDARY.contains(&old),erdtree_tower::elite::LEGENDARY.contains(&new));
    }
}

#[test]
fn preferred_rewards_can_be_claimed_and_resumed_without_rerolling() {
    use erdtree_tower::NodeKind;
    for class in Class::PLAYABLE {for kind in [3,9,33,67] {
        let mut run=Run::new(42,class);run.weapon_preferences=vec![Preference{kind,level:5},Preference{kind:1,level:2}];
        for _ in 0..8 {
            let node=run.available()[0];let room=run.nodes[node].kind;run.enter(node).unwrap();
            match room {
                NodeKind::Monster|NodeKind::Elite|NodeKind::Boss=>{
                    run.win_combat().unwrap();
                    assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
                    run.claim_reward(0).unwrap();
                },
                NodeKind::Camp=>run.leave_camp().unwrap(),NodeKind::Shop=>run.leave_shop().unwrap(),
                NodeKind::Chest=>run.finish_chest().unwrap(),NodeKind::Event=>run.resolve_event(false).unwrap(),
            }
        }
    }}
}
