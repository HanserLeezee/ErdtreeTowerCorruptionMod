use erdtree_tower::{Class,NodeKind,Reward,Run,rewards::{self,ItemKind}};

fn weapon_offers(class:Class)->(u32,u32) {
    let (mut favored,mut total)=(0,0);
    for seed in 0..300 {
        let mut run=Run::new(seed,class);run.ember_rewards=false;
        for (id,node) in run.nodes.iter().enumerate().filter(|(_,n)|n.kind==NodeKind::Monster) {
            let _=node;
            for reward in rewards::offers(&run,id) {
                if let Reward::Item(item)=reward && rewards::lookup(item).is_some_and(|i|i.kind==ItemKind::Weapon) {
                    total+=1;
                    if class.weapon_types().contains(&((erdtree_tower::affixes::base(item)&0x0fffffff)/1_000_000)) {favored+=1;}
                }
            }
        }
    }
    (favored,total)
}
#[test]
fn six_playable_classes_keep_their_original_archetypes() {
    assert_eq!(Class::PLAYABLE.len(),6);
    for class in Class::PLAYABLE {
        assert_eq!(Class::PLAYABLE.iter().filter(|c|c.archetype()==class.archetype()).count(),1);
        assert_eq!(Class::from_archetype(class.archetype()),Some(class));
    }
    assert_eq!(Class::Confessor.archetype(),6);
    // 职业重做前的剑士、盗贼、囚犯、一贫如洗已移除
    for archetype in [1,3,8,9] {assert_eq!(Class::from_archetype(archetype),None);}
    assert!(Class::PLAYABLE.iter().all(|c|!c.weapon_types().is_empty()));
    assert_eq!(Run::new(1,Class::Hero).class_rules,1);
}
#[test]
fn weapon_rewards_lean_towards_the_class() {
    for class in [Class::Vagabond,Class::Hero,Class::Samurai] {
        let (favored,total)=weapon_offers(class);
        assert!(favored as f64/total as f64>0.6,"{class:?} favored {favored}/{total}");
    }
}
#[test]
fn summoners_see_a_spirit_ash_on_every_ordinary_reward_page() {
    let (mut pages,mut with_spirit)=(0,0);
    for seed in 0..200 {
        let mut run=Run::new(seed,Class::Confessor);run.ember_rewards=false;
        assert!(run.summoner()&&erdtree_tower::embers::carries_spirit(&run));
        for (id,_) in run.nodes.iter().enumerate().filter(|(_,n)|n.kind==NodeKind::Monster) {
            pages+=1;
            if rewards::offers(&run,id).iter().any(|r|matches!(r,Reward::Item(i) if rewards::lookup(*i).is_some_and(|i|i.kind==ItemKind::Spirit))) {with_spirit+=1;}
        }
    }
    assert!(with_spirit*10>=pages*9,"{with_spirit}/{pages}");
}

#[test]
fn pre_rework_saves_are_retired_instead_of_loaded() {
    let run=Run::new(5,Class::Samurai);let text=run.encode().unwrap();
    assert!(!Run::retired_save(&text));
    let mut json:serde_json::Value=serde_json::from_str(&text).unwrap();
    json["class"]=serde_json::Value::String("Prisoner".into());
    assert!(Run::retired_save(&json.to_string())&&Run::decode(&json.to_string()).is_err());
    let mut old:serde_json::Value=serde_json::from_str(&text).unwrap();old.as_object_mut().unwrap().remove("class_rules");
    assert!(Run::retired_save(&old.to_string()));
    old["class_rules"]=serde_json::Value::from(0);assert!(Run::retired_save(&old.to_string())&&Run::decode(&old.to_string()).is_err());
    assert!(!Run::retired_save("not json"),"corrupted saves are still reported instead of retired");
}
#[test]
fn heritage_keeps_current_classes_and_drops_removed_ones() {
    use erdtree_tower::legacy::Legacy;
    let text=r#"{"classes":[["Samurai",{"xp":300,"talisman":2090}],["Wretch",{"xp":900,"talisman":0}],["Hero",{"xp":50,"talisman":0}]]}"#;
    let legacy=Legacy::decode(text);
    assert_eq!(legacy.classes.iter().map(|(c,_)|*c).collect::<Vec<_>>(),vec![Class::Samurai,Class::Hero]);
    assert_eq!((legacy.get(Class::Samurai).xp,legacy.get(Class::Samurai).talisman),(300,2090));
    assert_eq!(Legacy::decode(&legacy.encode()),legacy);
}
