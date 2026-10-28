use erdtree_tower::{Class,NodeKind,Phase,Reward,Run,elite,rewards};

#[test]
fn boss_guarantees_one_gold_and_rolls_the_other_two_normally() {
    let mut seen=std::collections::HashSet::new();
    let mut non_gold=0;
    for class in Class::ALL {for seed in 0..100 {
        let run=Run::new(seed,class);
        let node=run.nodes.iter().position(|n|n.kind==NodeKind::Boss).unwrap();
        let offers=rewards::offers(&run,node);
        assert_eq!(offers.len(),3);assert_eq!(offers,rewards::offers(&run,node));
        let mut ids=std::collections::HashSet::new();
        for (slot,reward) in offers.into_iter().enumerate() {
            let Reward::Item(id)=reward else {assert_ne!(slot,0);continue};
            let base=erdtree_tower::affixes::base(id);
            if slot==0 {assert!(elite::LEGENDARY.contains(&base));assert!(erdtree_tower::affixes::tier(id).is_none_or(|tier|tier==3));seen.insert(base);}
            else if erdtree_tower::affixes::tier(id).is_some_and(|tier|tier<3){non_gold+=1;}
            assert!(ids.insert(base));
        }
    }}
    assert_eq!(seen.len(),elite::LEGENDARY.len());
    assert!(non_gold>0,"Boss must not force all three cards to gold");
}

#[test]
fn collected_boss_pool_keeps_gold_weapon_even_without_casting_tools() {
    let mut run=Run::new(99,Class::Samurai);
    run.claimed_items=elite::LEGENDARY.to_vec();
    run.casting_tools=Some([false,false]);
    let node=run.nodes.iter().position(|n|n.kind==NodeKind::Boss).unwrap();
    let offers=rewards::offers(&run,node);
    assert_eq!(offers.len(),3);
    let Reward::Item(id)=offers[0] else {panic!("Expected guaranteed weapon")};
    assert_eq!(rewards::lookup(id).unwrap().kind,rewards::ItemKind::Weapon);
    assert_eq!(erdtree_tower::affixes::tier(id),Some(3));
}

#[test]
fn old_three_gold_pending_page_migrates_without_losing_progress() {
    let mut run=Run::new(123,Class::Samurai);
    run.boss_reward_rules=0;
    loop {
        match run.phase.clone() {
            Phase::Map=>{let node=run.available()[0];run.enter(node).unwrap();},
            Phase::Combat{..}=>run.win_combat().unwrap(),
            Phase::Reward{id,..} if run.nodes[id].kind==NodeKind::Boss=>break,
            Phase::Reward{..}=>run.decline_reward().unwrap(),
            Phase::Node{id}=>match run.nodes[id].kind {
                NodeKind::Camp=>run.leave_camp().unwrap(),NodeKind::Shop=>run.leave_shop().unwrap(),
                NodeKind::Chest=>run.finish_chest().unwrap(),NodeKind::Event=>run.resolve_event(false).unwrap(),_=>unreachable!(),
            },_=>panic!("unexpected phase"),
        }
    }
    let mut json=serde_json::to_value(&run).unwrap();json.as_object_mut().unwrap().remove("boss_reward_rules");
    let migrated=Run::decode(&json.to_string()).unwrap();
    assert_eq!(migrated.boss_reward_rules,1);
    assert_eq!(migrated.gold,run.gold);
    assert_eq!(migrated.claimed_items,run.claimed_items);
    assert_eq!(migrated.visited,run.visited);
    let Phase::Reward{id,ref offers}=migrated.phase else{panic!()};
    assert_eq!(*offers,rewards::offers(&migrated,id));
    assert_eq!(Run::decode(&migrated.encode().unwrap()).unwrap(),migrated);
}

#[test]
fn boss_rewards_can_be_claimed_and_saved_in_all_three_acts() {
    for seed in 0..30 {
        let mut run=Run::new(seed,Class::Samurai);
        while !matches!(run.phase,Phase::Won) {
            match run.phase.clone() {
                Phase::Map=>{let node=run.available()[0];run.enter(node).unwrap();},
                Phase::Combat{..}=>run.win_combat().unwrap(),
                Phase::Reward{id,offers}=>{
                    let index=if run.nodes[id].kind==NodeKind::Boss {
                        offers.iter().position(|r|matches!(r,Reward::Item(id) if *id>>28==4)).unwrap_or(0)
                    }else{0};
                    run.claim_reward(index).unwrap();
                    run=Run::decode(&run.encode().unwrap()).unwrap();
                },
                Phase::Node{id}=>match run.nodes[id].kind {
                    NodeKind::Camp=>run.leave_camp().unwrap(),
                    NodeKind::Shop=>run.leave_shop().unwrap(),
                    NodeKind::Chest=>run.finish_chest().unwrap(),
                    NodeKind::Event=>run.resolve_event(false).unwrap(),
                    _=>unreachable!(),
                },
                _=>panic!("Unexpected phase"),
            }
        }
    }
}
