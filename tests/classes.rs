use erdtree_tower::{Class,NodeKind,Reward,Run,rewards::{self,ItemKind}};

fn weapon_offers(class:Class,rules:u8)->(u32,u32) {
    let (mut favored,mut total)=(0,0);
    for seed in 0..300 {
        let mut run=Run::new(seed,class);run.class_rules=rules;run.ember_rewards=false;
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
    for class in Class::ALL {assert_eq!(Class::ALL.iter().filter(|c|c.archetype()==class.archetype()).count(),1);}
    assert_eq!((Class::Confessor.archetype(),Class::Prisoner.archetype()),(6,8));
    assert!(Class::PLAYABLE.iter().all(|c|!c.weapon_types().is_empty()));
    assert!(!Class::Wretch.playable()&&Class::Confessor.playable());
    assert_eq!(Run::new(1,Class::Hero).class_rules,1);
}
#[test]
fn weapon_rewards_lean_towards_the_class() {
    for class in [Class::Vagabond,Class::Hero,Class::Samurai] {
        let (favored,total)=weapon_offers(class,1);
        let share=favored as f64/total as f64;
        assert!(share>0.6,"{class:?} favored {favored}/{total}");
        let (old_favored,old_total)=weapon_offers(class,0);
        assert!((old_favored as f64/old_total as f64)<share-0.25,"old saves keep the unbiased pool: {class:?} {old_favored}/{old_total}");
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
        let mut old=run.clone();old.class_rules=0;assert!(!old.summoner());
    }
    assert!(with_spirit*10>=pages*9,"{with_spirit}/{pages}");
}
