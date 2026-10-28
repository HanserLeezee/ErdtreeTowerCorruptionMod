use crate::{NodeKind, Reward, RuleResult, Run};

/// 问号房里的赌运事件：进入事件房间后与 NPC 交谈，弹出事件卡片
pub struct Story {
    pub title: &'static str,
    pub text: &'static str,
    pub choices: [&'static str; 2],
}
pub const DICE:usize=0;pub const MYSTERY:usize=1;pub const REFORGE:usize=2;
pub const DICE_STAKES:[u32;2]=[1000,3000];
pub const BOX_PRICES:[u32;2]=[2000,5000];
pub const REFORGE_PRICES:[u32;2]=[2000,6000];
pub const STORIES: [Story; 3] = [
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
/// 问号房抽到的事件。event_rules 2 起每个事件按权重表单独抽取（3 起用 EVENT_WEIGHTS）；更早的旅程沿用 8 格分配，没有赌运事件
#[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)]
pub enum Draw {Trial,Chests,Altar,Locked,Scarabs,Spring,Dice,Mystery,Reforge}
/// 问号房事件权重（event_rules 3 起）：九个事件相同，各 1/9
pub const EVENT_WEIGHTS:[(Draw,u32);9]=[
    (Draw::Trial,1),(Draw::Chests,1),(Draw::Altar,1),(Draw::Locked,1),(Draw::Scarabs,1),(Draw::Spring,1),
    (Draw::Dice,1),(Draw::Mystery,1),(Draw::Reforge,1),
];
/// event_rules 2 的权重，合计 20（试炼 4，其余 2）：旧旅程读档沿用，房间不变
pub const EVENT_WEIGHTS_V2:[(Draw,u32);9]=[
    (Draw::Trial,4),(Draw::Chests,2),(Draw::Altar,2),(Draw::Locked,2),(Draw::Scarabs,2),(Draw::Spring,2),
    (Draw::Dice,2),(Draw::Mystery,2),(Draw::Reforge,2),
];
impl Run {
    /// 本旅程第 id 个节点（问号房）的事件，由种子与节点决定，读档不变
    pub fn event_draw(&self,id:usize)->Draw {
        if self.event_rules<2 {
            return match crate::mix(self.seed^id as u64^0x10cced)%8 {
                0=>Draw::Locked,3=>Draw::Scarabs,4|6|7=>Draw::Trial,5=>Draw::Spring,
                _=>if crate::regional_events::kind(self.seed,id)==1 {Draw::Altar}else{Draw::Chests},
            };
        }
        let weights=if self.event_rules<3 {EVENT_WEIGHTS_V2}else{EVENT_WEIGHTS};
        let total:u32=weights.iter().map(|(_,w)|w).sum();
        let mut roll=(crate::mix(self.seed^(id as u64).wrapping_mul(0x9e37)^0xe7e27)%total as u64) as u32;
        for (draw,weight) in weights {if roll<weight {return draw;} roll-=weight;}
        unreachable!()
    }
    /// 问号房里的赌运事件（STORIES 下标）
    pub fn gamble(&self,id:usize)->Option<usize> {
        if self.nodes.get(id).is_none_or(|n|n.kind!=NodeKind::Event) {return None;}
        match self.event_draw(id) {Draw::Dice=>Some(DICE),Draw::Mystery=>Some(MYSTERY),Draw::Reforge=>Some(REFORGE),_=>None}
    }
    /// 事件房间是贪欲祭坛还是三重宝匣（联机时赌运事件也改为二者之一，按种子决定；其余事件沿用原来的判定）
    pub fn altar(&self,id:usize)->bool {
        match self.event_draw(id) {Draw::Altar=>true,Draw::Chests=>false,_=>crate::regional_events::kind(self.seed,id)==1}
    }
}
/// 骰子赌徒：本节点的结果由种子决定，与押注多少无关
pub fn dice_wins(run:&Run,node:usize)->bool {crate::mix(run.reward_seed()^(node as u64).wrapping_mul(7919)^0xd1ce)%2==0}
/// 事件结算后的提示（赌运事件需要读取结果）
pub fn story_outcome(run:&Run,choice:usize)->String {
    let (Ok(index),Ok(node))=(run.story_index(),run.active_node(NodeKind::Event)) else {return String::new();};
    match index {
        DICE=>if dice_wins(run,node) {format!("骰子落定，你赢了 {} 卢恩。",DICE_STAKES[choice])}else{format!("骰子落定，你输掉了 {} 卢恩。",DICE_STAKES[choice])},
        MYSTERY=>"商人掀开黑布，把箱子里的装备交给你。".into(),
        _=>if choice==0 {"火焰退去，武器换了一种形状。".into()}else{"火焰烧得更旺，武器带着新的光泽回到你手中。".into()},
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
        self.gamble(id).ok_or_else(||"这里没有赌运事件".into())
    }
    /// 赌运事件选一项（0 或 1），每处只能用一次；选完留在房间，从传送门离开
    pub fn resolve_story(&mut self, choice: usize) -> RuleResult<StoryEffect> {
        let id = self.active_node(NodeKind::Event)?;
        let index=self.story_index()?;
        if choice > 1 {return Err("无效事件选项".into());}
        if self.regional_event.opened!=0 {return Err("这里的事件已经用过".into());}
        let mut next = self.clone();
        let mut effect = StoryEffect::default();
        let mut price = 0;
        match index {
            DICE => {
                let stake=DICE_STAKES[choice];
                if next.gold<stake {return Err("卢恩不足，事件未消耗".into());}
                if dice_wins(self,id) {next.gold=next.gold.saturating_add(stake);}else{price=stake;}
            }
            MYSTERY => {
                price=BOX_PRICES[choice];
                let item=crate::affixes::mystery_box(self,id,choice==1).ok_or("盲盒里没有可用的装备")?;
                next.grant(Reward::Item(item))?;effect.item=Some(item);
            }
            _ => {price=REFORGE_PRICES[choice];effect.reforge=true;effect.reforge_upgrade=choice==1;}
        }
        if next.gold < price {return Err("卢恩不足，事件未消耗".into());}
        next.gold -= price;
        next.regional_event.opened=1;
        *self = next;
        Ok(effect)
    }
}

#[cfg(test)]
mod gamble_tests {
    use crate::{Class,NodeKind,Run,affixes,events::*};
    #[test]
    fn every_event_has_its_own_weight_and_gambles_happen_in_the_room() {
        let total:u32=EVENT_WEIGHTS.iter().map(|(_,w)|w).sum();let total_v2:u32=EVENT_WEIGHTS_V2.iter().map(|(_,w)|w).sum();
        let mut counts=std::collections::HashMap::new();let mut counts_v2=std::collections::HashMap::new();let mut nodes=0u32;
        for seed in 0..3000 {
            let run=Run::new(seed,Class::Samurai);
            for id in (0..run.nodes.len()).filter(|id|run.nodes[*id].kind==NodeKind::Event) {
                nodes+=1;let draw=run.event_draw(id);*counts.entry(draw).or_insert(0u32)+=1;
                let mut v2=run.clone();v2.event_rules=2;*counts_v2.entry(v2.event_draw(id)).or_insert(0u32)+=1;
                let mut entered=run.clone();entered.debug_jump(id).unwrap();entered.enter(id).unwrap();
                let p=&entered.regional_event;
                assert_eq!((p.trial,p.locked,p.scarabs,p.recovery),(draw==Draw::Trial,draw==Draw::Locked,draw==Draw::Scarabs,draw==Draw::Spring));
                match draw {
                    Draw::Altar=>assert!(run.altar(id)),Draw::Chests=>assert!(!run.altar(id)),
                    // 赌运事件：用一次后留在房间（读档校验通过、不能再用），从传送门离开
                    Draw::Dice|Draw::Mystery|Draw::Reforge=>{
                        assert_eq!(entered.story_index(),Ok(run.gamble(id).unwrap()));
                        entered.gold=8000;
                        if entered.resolve_story(0).is_ok() {
                            assert_eq!(entered.phase,crate::Phase::Node{id});
                            let mut loaded=Run::decode(&entered.encode().unwrap()).unwrap();
                            assert!(loaded.resolve_story(0).is_err()&&loaded.resolve_story(1).is_err());
                            loaded.leave_regional_event().unwrap();assert_eq!(loaded.phase,crate::Phase::Map);
                        }
                    },
                    _=>assert!(entered.story_index().is_err()),
                }
                // 旧旅程沿用原来的 8 格分配，没有赌运事件
                let mut old=run.clone();old.event_rules=1;assert_eq!(old.gamble(id),None);
            }
        }
        // 每个事件的实际出现率接近各自权重（误差在 ±15% 以内）；event_rules 2 的旧旅程仍按旧权重
        for (weights,total,counts) in [(EVENT_WEIGHTS,total,&counts),(EVENT_WEIGHTS_V2,total_v2,&counts_v2)] {for (draw,weight) in weights {
            let expected=nodes as f64*weight as f64/total as f64;let seen=*counts.get(&draw).unwrap_or(&0) as f64;
            assert!((seen-expected).abs()<expected*0.15,"{draw:?}: {seen} vs {expected}");
        }}
        assert!(EVENT_WEIGHTS.iter().all(|(_,w)|*w==EVENT_WEIGHTS[0].1));
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
            match run.story_index().unwrap_or(usize::MAX) {
                MYSTERY=>{let mut r=run.clone();let effect=r.resolve_story(1).unwrap();assert_eq!(r.gold,1000);assert!(r.claimed_items.contains(&effect.item.unwrap()));},
                REFORGE=>{
                    let mut r=run.clone();let effect=r.resolve_story(0).unwrap();assert!(effect.reforge&&!effect.reforge_upgrade);assert_eq!(r.gold,6000-REFORGE_PRICES[0]);
                    let mut refine=run.clone();let effect=refine.resolve_story(1).unwrap();assert!(effect.reforge_upgrade);assert_eq!(refine.gold,0);
                    assert!(run.clone().resolve_story(2).is_err());
                },
                _=>{},
            }
        }
    }
}
