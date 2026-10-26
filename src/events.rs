use crate::{BlessingKind, NodeKind, Reward, RuleResult, Run};

pub struct Story {
    pub title: &'static str,
    pub text: &'static str,
    pub choices: [&'static str; 2],
}
/// 旧旅程只抽前 6 个故事；event_rules 为 1 的新旅程加入赌运事件
pub const LEGACY_STORIES:usize=6;
pub const DICE:usize=6;pub const MYSTERY:usize=7;pub const REFORGE:usize=8;
pub const DICE_STAKES:[u32;2]=[1000,3000];
pub const BOX_PRICES:[u32;2]=[2000,5000];
pub const REFORGE_PRICES:[u32;2]=[2000,6000];
pub const STORIES: [Story; 9] = [
    Story {
        title: "鲜血祭坛",
        text: "祭坛上的指痕尚未干涸。一个声音从石缝中传来：‘黄金早已离去，鲜血仍能作价。’两枚护符静候你的选择。",
        choices: [
            "损失最大生命的 15%，获得鲜血脉动",
            "损失最大生命的 20%，获得蓄势重击",
        ],
    },
    Story {
        title: "流浪祈祷者",
        text: "祈祷者拢紧破旧的斗篷：‘我已走不动下一段路。若你愿分些卢恩，这枚护符便替我陪你走下去。’",
        choices: ["支付 1200 卢恩，获得生命护符", "支付 1200 卢恩，获得耐力护符"],
    },
    Story {
        title: "月光泉",
        text: "月光在泉中分成红与蓝。守泉人的石像刻着一句话：‘投入黄金，只取一捧；贪求两者的人，终会一无所得。’",
        choices: [
            "支付 800 卢恩，恢复 40% 最大生命",
            "支付 500 卢恩，恢复 60% 最大专注",
        ],
    },
    Story {
        title: "遗弃行囊",
        text: "一只行囊压在碎石下，主人已不知去向。纸条上写着：‘后来的人，请取走一份，另一份留给再后来的人。’",
        choices: ["取得 1200 卢恩", "补充 2 瓶圣杯瓶"],
    },
    Story {
        title: "禁书抄写员",
        text: "抄写员合上禁书，露出藏在书脊中的护符：‘知识未必救你。但这两条道路，总有一条能让敌人先倒下。’",
        choices: [
            "支付 1000 卢恩，获得魔法护符",
            "支付 1000 卢恩，获得火焰护符",
        ],
    },
    Story {
        title: "受伤的铁匠",
        text: "铁匠按住伤口，熄灭的炉旁还有余温：‘帮我撑过这一刻，钱袋归你。或者付些报酬，我还能再落几锤。’",
        choices: [
            "损失最大生命的 25%，取得 2000 卢恩",
            "支付 1200 卢恩，右手武器升至背包最高等效等级，再普通 +3 / 失色 +1",
        ],
    },
    Story {
        title: "骰子赌徒",
        text: "赌徒把两枚骨骰在掌心摇得作响：‘押上卢恩，赢了翻倍，输了归我。骰子落下之前，谁也不知道结果。’",
        choices: ["押 1000 卢恩\n赢了拿回 2000，输了全归赌徒", "押 3000 卢恩\n赢了拿回 6000，输了全归赌徒"],
    },
    Story {
        title: "盲盒商人",
        text: "商人拍了拍蒙着黑布的木箱：‘里面是一件武器或防具，品质全凭运气。贵一些的箱子，运气也会好一些。’",
        choices: [
            "支付 2000 卢恩\n随机一件武器或防具\n\n稀有 65%\n史诗 30%\n传说 5%",
            "支付 5000 卢恩\n随机一件武器或防具\n\n稀有 15%\n史诗 55%\n传说 30%",
        ],
    },
    Story {
        title: "词条重铸台",
        text: "一座仍有余温的熔炉。铭文写着：‘将词条交还火焰，火焰会还你另一种形状。’",
        choices: [
            "支付 2000 卢恩\n重铸右手的词条武器\n换成同一把武器的另一种词条组合\n强化等级与战灰保留\n\n1/5 的概率品质 +1",
            "支付 6000 卢恩\n精炼右手的词条武器\n换成另一种词条组合，品质必定 +1\n强化等级与战灰保留\n\n传说品质的武器不能精炼",
        ],
    },
];
/// 骰子赌徒：本节点的结果由种子决定，与押注多少无关
pub fn dice_wins(run:&Run,node:usize)->bool {crate::mix(run.reward_seed()^(node as u64).wrapping_mul(7919)^0xd1ce)%2==0}
/// 事件结算后的提示（赌运事件需要读取结果）
pub fn story_outcome(run:&Run,choice:usize)->String {
    let (Ok(index),Ok(node))=(run.story_index(),run.active_node(NodeKind::Event)) else {return outcome(0,2).into();};
    if choice==2 {return outcome(index,2).into();}
    match index {
        DICE=>if dice_wins(run,node) {format!("骰子落定，你赢了 {} 卢恩。",DICE_STAKES[choice])}else{format!("骰子落定，你输掉了 {} 卢恩。",DICE_STAKES[choice])},
        MYSTERY=>"商人掀开黑布，把箱子里的装备交给你。".into(),
        REFORGE=>if choice==0 {"火焰退去，武器换了一种形状。".into()}else{"火焰烧得更旺，武器带着新的光泽回到你手中。".into()},
        _=>outcome(index,choice).into(),
    }
}

pub fn outcome(story:usize,choice:usize)->&'static str {
    if choice==2{return "你没有接受交易，沿着来路继续前行。";}
    match story {
        0=>"鲜血渗入石槽，一枚护符落入掌心。可在装备菜单自行佩戴。",
        1=>"祈祷者收下卢恩，将护符交给你：愿你走到我未曾抵达的地方。",
        2=>"投入的卢恩沉入泉底，温热的泉水抚平了旅途的疲惫。",
        3=>if choice==0{"你收起钱袋，把补给留给下一位旅人。"}else{"你补充了圣杯瓶，将钱袋放回行囊。"},
        4=>"抄写员收好卢恩，把护符递给你。它的力量需要佩戴后才会生效。",
        _=>if choice==0{"铁匠终于稳住呼吸，将装着卢恩的钱袋推到你身边。"}else{"最后一声锤响落下，铁匠将强化后的武器交还给你。"},
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct StoryEffect {
    pub hp_cost_percent: u32,
    pub heal_percent: u32,
    pub fp_percent: u32,
    pub refill: bool,
    /// 盲盒开出的装备
    pub item: Option<u32>,
    /// 词条重铸：由运行时读取右手武器并替换
    pub reforge: bool,
    /// 精炼重铸：品质必定 +1
    pub reforge_upgrade: bool,
}

impl Run {
    pub fn story_index(&self) -> RuleResult<usize> {
        let id = self.active_node(NodeKind::Event)?;
        let count=if self.event_rules>=1 {STORIES.len()}else{LEGACY_STORIES};
        Ok((crate::mix(self.seed) as usize + id) % count)
    }
    pub fn resolve_story(&mut self, choice: usize) -> RuleResult<StoryEffect> {
        let id = self.active_node(NodeKind::Event)?;
        if choice > 2 {
            return Err("无效事件选项".into());
        }
        let mut next = self.clone();
        let mut effect = StoryEffect::default();
        if choice < 2 {
            let mut price = 0;
            match self.story_index()? {
                0 => {
                    effect.hp_cost_percent = if choice == 0 { 15 } else { 20 };
                    next.grant(Reward::Blessing(if choice == 0 {
                        BlessingKind::Bleed
                    } else {
                        BlessingKind::Charge
                    }))?;
                }
                1 => {
                    price = 1200;
                    next.grant(Reward::Blessing(if choice == 0 {
                        BlessingKind::Vigor
                    } else {
                        BlessingKind::Endurance
                    }))?;
                }
                2 => {
                    if choice == 0 {
                        price = 800;
                        effect.heal_percent = 40;
                        if next.hp == next.max_hp {
                            return Err("生命已满".into());
                        }
                    } else {
                        price = 500;
                        effect.fp_percent = 60;
                    }
                }
                3 => {
                    if choice == 0 {
                        next.grant(Reward::Gold(1200))?;
                    } else {
                        if next.flasks == next.flask_limit {
                            return Err("圣杯瓶已满".into());
                        }
                        effect.refill = true;
                        next.flasks = (next.flasks + 2).min(next.flask_limit);
                    }
                }
                DICE => {
                    let stake=DICE_STAKES[choice];
                    if next.gold<stake {return Err("卢恩不足，事件未消耗".into());}
                    if crate::events::dice_wins(self,id) {next.gold=next.gold.saturating_add(stake);}else{price=stake;}
                }
                MYSTERY => {
                    price=BOX_PRICES[choice];
                    let item=crate::affixes::mystery_box(self,id,choice==1).ok_or("盲盒里没有可用的装备")?;
                    next.grant(Reward::Item(item))?;effect.item=Some(item);
                }
                REFORGE => {price=REFORGE_PRICES[choice];effect.reforge=true;effect.reforge_upgrade=choice==1;}
                4 => {
                    price = 1000;
                    next.grant(Reward::Blessing(if choice == 0 {
                        BlessingKind::Sorcery
                    } else {
                        BlessingKind::Flame
                    }))?;
                }
                _ => {
                    if choice == 0 {
                        effect.hp_cost_percent = 25;
                        next.grant(Reward::Gold(2000))?;
                    } else {
                        price = 1200;
                        next.grant(Reward::UpgradeWeapon)?;
                    }
                }
            }
            if next.gold < price {
                return Err("卢恩不足，事件未消耗".into());
            }
            next.gold -= price;
            let cost = next
                .max_hp
                .saturating_mul(effect.hp_cost_percent)
                .div_ceil(100);
            if next.hp <= cost {
                return Err("生命不足，事件不会致死".into());
            }
            next.hp = (next.hp - cost)
                .saturating_add(next.max_hp * effect.heal_percent / 100)
                .min(next.max_hp);
        }
        next.finish(id);
        *self = next;
        Ok(effect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Class, Phase};
    fn story(index: usize) -> Run {
        for seed in 0..1000 {
            let mut r = Run::new(seed, Class::Samurai);
            r.nodes[0].kind = NodeKind::Event;
            r.phase = Phase::Node { id: 0 };
            r.gold = 8000;
            r.hp = 50;
            r.flasks = 2;
            if r.story_index().unwrap() == index {
                return r;
            }
        }
        panic!("missing story");
    }
    #[test]
    fn events_use_rune_prices_and_failed_payment_leaves_no_reward() {
        let mut r=story(1);r.gold=1199;let old=r.clone();
        assert!(r.resolve_story(0).is_err());assert_eq!(r,old);
        r.gold=1200;r.resolve_story(0).unwrap();assert_eq!(r.gold,0);
        let mut r=story(3);r.gold=0;r.resolve_story(0).unwrap();assert_eq!(r.gold,1200);
        assert!(STORIES.iter().all(|s|s.choices.iter().all(|c|!c.contains("塔币"))));
    }
    #[test]
    fn all_story_choices_resolve_once_and_leave_is_free() {
        for index in 0..STORIES.len() {
            for choice in 0..=2 {
                let mut r = story(index);
                let old = r.clone();
                r.resolve_story(choice).unwrap();
                assert_eq!(r.phase, Phase::Map);
                assert_eq!(r.visited, vec![0]);
                assert!(r.resolve_story(choice).is_err());
                if choice == 2 {
                    assert_eq!((r.gold, r.hp, r.flasks), (old.gold, old.hp, old.flasks));
                }
            }
        }
    }
    #[test]
    fn failed_payment_and_lethal_sacrifice_are_atomic() {
        let mut r = story(1);
        r.gold = 0;
        let old = r.clone();
        assert!(r.resolve_story(0).is_err());
        assert_eq!(r, old);
        let mut r = story(0);
        r.hp = 15;
        let old = r.clone();
        assert!(r.resolve_story(0).is_err());
        assert_eq!(r, old);
    }

    #[test]
    fn capped_blessing_and_full_resources_preserve_event() {
        let mut r = story(0);
        r.blessings.push(crate::Blessing {
            kind: BlessingKind::Bleed,
            level: 3,
        });
        let old = r.clone();
        assert!(r.resolve_story(0).is_err());
        assert_eq!(r, old);
        let mut r = story(2);
        r.hp = r.max_hp;
        let old = r.clone();
        assert!(r.resolve_story(0).is_err());
        assert_eq!(r, old);
        let mut r = story(3);
        r.flasks = r.flask_limit;
        let old = r.clone();
        assert!(r.resolve_story(1).is_err());
        assert_eq!(r, old);
    }
}
#[cfg(test)]
mod gamble_tests {
    use crate::{Class,NodeKind,Phase,Run,affixes,events::*};
    #[test]
    fn new_runs_draw_the_gamble_stories_and_old_runs_do_not() {
        let mut seen=std::collections::HashSet::new();
        for seed in 0..300 {
            let mut run=Run::new(seed,Class::Samurai);
            for (id,_) in run.nodes.clone().iter().enumerate().filter(|(_,n)|n.kind==NodeKind::Event) {
                run.phase=Phase::Node{id};seen.insert(run.story_index().unwrap());
                let mut old=run.clone();old.event_rules=0;assert!(old.story_index().unwrap()<LEGACY_STORIES);
            }
        }
        assert_eq!(seen.len(),STORIES.len());
        let mut json:serde_json::Value=serde_json::from_str(&Run::new(1,Class::Samurai).encode().unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("event_rules");
        assert_eq!(Run::decode(&json.to_string()).unwrap().event_rules,0);
    }
    #[test]
    fn dice_doubles_or_loses_the_stake_and_is_fixed_per_node() {
        let (mut wins,mut losses)=(0,0);
        for seed in 0..400 {
            let mut run=Run::new(seed,Class::Samurai);
            let Some(id)=run.nodes.iter().position(|n|n.kind==NodeKind::Event) else{continue};
            run.debug_jump(id).unwrap();run.enter(id).unwrap();
            if run.story_index()!=Ok(DICE) {continue;}
            run.gold=5000;
            for choice in 0..2 {
                let mut r=run.clone();let message=story_outcome(&r,choice);r.resolve_story(choice).unwrap();
                if dice_wins(&run,id) {assert_eq!(r.gold,5000+DICE_STAKES[choice]);assert!(message.contains("赢"));wins+=1;}
                else {assert_eq!(r.gold,5000-DICE_STAKES[choice]);assert!(message.contains("输"));losses+=1;}
            }
            let mut poor=run.clone();poor.gold=999;assert!(poor.resolve_story(0).is_err());
        }
        assert!(wins>0&&losses>0);
    }
    #[test]
    fn mystery_boxes_pay_out_equipment_and_reforge_picks_a_sibling() {
        let mut tiers=[[0u32;4];2];let mut kinds=std::collections::HashSet::new();
        for seed in 0..600 {for premium in [false,true] {
            let run=Run::new(seed,Class::Samurai);
            let item=affixes::mystery_box(&run,5,premium).unwrap();
            assert!(crate::rewards::lookup(item).is_some());kinds.insert(crate::rewards::lookup(item).unwrap().kind==crate::rewards::ItemKind::Armor);
            tiers[usize::from(premium)][usize::from(affixes::tier(item).unwrap())]+=1;
            let mut claimed=run.clone();claimed.claimed_items.push(item);
            assert_ne!(affixes::mystery_box(&claimed,5,premium),Some(item),"owned items are skipped");
            let reforged=affixes::reforge(&run,5,item,false).unwrap();
            match affixes::reforge(&run,5,item,true) {
                Some(up)=>assert_eq!(affixes::tier(up).unwrap(),affixes::tier(item).unwrap()+1),
                None=>assert_eq!(affixes::tier(item),Some(3),"only legendary items cannot be refined"),
            }
            assert_eq!(affixes::base(reforged),affixes::base(item));assert_ne!(reforged,item);
            assert!(affixes::tier(reforged).unwrap()>=affixes::tier(item).unwrap()&&affixes::tier(reforged).unwrap()<=affixes::tier(item).unwrap()+1);
        }}
        assert_eq!(kinds.len(),2,"boxes hold both weapons and armor");
        // 普通盲盒 稀有 65／史诗 30／传说 5，高级盲盒 15／55／30（600 次抽取，允许抽样误差）
        let share=|count:u32|count*100/600;
        assert!(tiers[0][0]==0&&tiers[1][0]==0,"no common boxes: {tiers:?}");
        assert!(matches!((share(tiers[0][1]),share(tiers[0][3])),(58..=72,1..=9)),"basic box tiers {tiers:?}");
        assert!(matches!((share(tiers[1][1]),share(tiers[1][3])),(9..=21,24..=36)),"premium box tiers {tiers:?}");
        for seed in 0..400 {
            let mut run=Run::new(seed,Class::Samurai);
            let Some(id)=run.nodes.iter().position(|n|n.kind==NodeKind::Event) else{continue};
            run.debug_jump(id).unwrap();run.enter(id).unwrap();run.gold=6000;
            match run.story_index().unwrap() {
                MYSTERY=>{let mut r=run.clone();let effect=r.resolve_story(1).unwrap();assert_eq!(r.gold,1000);assert!(r.claimed_items.contains(&effect.item.unwrap()));},
                REFORGE=>{
                    let mut r=run.clone();let effect=r.resolve_story(0).unwrap();assert!(effect.reforge&&!effect.reforge_upgrade);assert_eq!(r.gold,6000-REFORGE_PRICES[0]);
                    let mut refine=run.clone();let effect=refine.resolve_story(1).unwrap();assert!(effect.reforge_upgrade);assert_eq!(refine.gold,0);
                    let mut leave=run.clone();assert!(!leave.resolve_story(2).unwrap().reforge);assert_eq!(leave.gold,6000);
                },
                _=>{},
            }
        }
    }
}
