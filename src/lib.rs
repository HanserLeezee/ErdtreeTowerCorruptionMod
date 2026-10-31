use serde::{Deserialize, Serialize};

pub mod coordinator;
pub mod party_rewards;
pub mod party_link;
pub mod party_game;
pub mod party_law;
pub mod party_vote;
pub mod party_roster;
pub mod party_handshake;
pub mod party_events;
pub mod map_drawing;
pub mod contracts;
pub mod act_gifts;
pub mod elite;
pub mod descent;
pub mod fallen_oaths;
pub mod torches;
pub mod encounter;
pub mod events;
pub mod regional_events;
pub mod stormveil;
pub mod rewards;
pub mod affixes;
pub mod talismans;
pub mod boss_affixes;
pub mod embers;
pub mod scaling;
pub mod law;
pub mod debug;
pub mod journal;
pub mod legacy;
pub mod titles;
pub mod auras;
pub mod themes;

pub mod music;
pub mod options;
pub mod dlc;
pub mod stat_summary;

pub type RuleResult<T> = Result<T, String>;

/// 六个职业，沿用原版出身的名字与编号（存档里按名字记录）：
/// 流浪骑士→守誓骑士、勇者→狂战士、武士→影刃、观星者→星辉术士、预言家→祷誓者、密使→召唤师。
/// 职业重做前的剑士、盗贼、囚犯、一贫如洗已经移除，它们的旧存档不再继续（见 Run::retired_save）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Class {
    Vagabond,
    Samurai,
    Astrologer,
    Prophet,
    Hero,
    Confessor,
}

impl Class {
    /// 选职业界面的顺序
    pub const PLAYABLE:[Self;6]=[Self::Vagabond,Self::Hero,Self::Samurai,Self::Astrologer,Self::Prophet,Self::Confessor];
    pub fn sorcerer(self)->bool {self==Self::Astrologer}
    pub fn cleric(self)->bool {matches!(self,Self::Prophet|Self::Confessor)}
    /// 原版出身编号（CharaInitParam 3000+编号）
    pub fn archetype(self)->u8 {match self {
        Self::Vagabond=>0,Self::Hero=>2,Self::Astrologer=>4,Self::Prophet=>5,Self::Confessor=>6,Self::Samurai=>7,
    }}
    pub fn from_archetype(archetype:u8)->Option<Self> {Self::PLAYABLE.into_iter().find(|c|c.archetype()==archetype)}
    /// 职业擅长的武器类型（武器编号 ÷ 1000000）：武器奖励约七成从这些类型里出
    pub fn weapon_types(self)->&'static [u32] {match self {
        Self::Vagabond=>&[2,3,18,31,32],          // 直剑、大剑、斧枪、中盾、大盾
        Self::Hero=>&[4,11,12,14,15,23],          // 特大剑、槌、大槌、斧、大斧、特大武器
        Self::Samurai=>&[1,7,8,9,40,41],          // 匕首、曲剑、大曲剑、刀、弓、长弓
        Self::Astrologer=>&[33,1,5],              // 法杖、匕首、刺剑
        Self::Prophet=>&[34,16,17],               // 圣印记、矛、大矛
        Self::Confessor=>&[1,2,30,31],            // 匕首、直剑、小盾、中盾
    }}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    Monster,
    Elite,
    Boss,
    Camp,
    Shop,
    Event,
    Chest,
}

impl NodeKind {
    pub fn combat(self) -> bool {
        matches!(self, Self::Monster | Self::Elite | Self::Boss)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Monster => "怪物",
            Self::Elite => "精英",
            Self::Boss => "Boss",
            Self::Camp => "篝火",
            Self::Shop => "商店",
            Self::Event => "事件",
            Self::Chest => "宝箱",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub kind: NodeKind,
    pub layer: usize,
    pub lane: usize,
    pub next: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlessingKind {
    Guard,
    Charge,
    Dodge,
    Bleed,
    Sorcery,
    Flame,
    Vigor,
    Endurance,
}

impl BlessingKind {
    pub const ALL: [Self; 8] = [
        Self::Guard,
        Self::Charge,
        Self::Dodge,
        Self::Bleed,
        Self::Sorcery,
        Self::Flame,
        Self::Vigor,
        Self::Endurance,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Guard => "铁壁反击",
            Self::Charge => "蓄势重击",
            Self::Dodge => "闪避锋芒",
            Self::Bleed => "鲜血脉动",
            Self::Sorcery => "魔力连携",
            Self::Flame => "余烬之火",
            Self::Vigor => "生命赐福",
            Self::Endurance => "耐力赐福",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blessing {
    pub kind: BlessingKind,
    pub level: u8,
}

/// 强化石出现在普通战、精英战奖励里的概率（百分比）
pub const STONE_CHANCE:u64=15;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reward {
    Blessing(BlessingKind),
    UpgradeWeapon,
    Gold(u32),
    Item(u32),
    TalismanBuff(u32),
    SacredTear,
    FlaskCapacity,
    MemorySlot,
    StarStream,
    Ember(embers::Ember),
    /// 强化石（营地强化凭证 7500100）：在营地铁匠或商店强化一次武器（也可升阶护符、调灵骨灰）
    UpgradeStone,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Map,
    Combat { id: usize },
    Node { id: usize },
    Reward { id: usize, offers: Vec<Reward> },
    Dead,
    Won,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampChoice {
    Refill,
    RefillBlue,
    RefillRedOnly,
    RefillBlueOnly,
    UpgradeWeapon,
    UpgradeBlessing(usize),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct CombatCheckpoint {
    id: usize,
    hp: u32,
    flasks: u8,
}

/// 商店价格在第三幕最后一层的百分比（见 Run::shop_position_percent）
pub const SHOP_PRICE_END:u32=200;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    schema: u32,
    #[serde(default)]
    pub affix_equipment: bool,
    #[serde(default)]
    pub affix_rules: u8,
    #[serde(default)]
    pub boss_reward_rules: u8,
    /// 职业规则：固定为 1（职业重做后的六个职业）。职业重做前的旧存档为 0，已不再支持，见 Run::retired_save
    #[serde(default)]
    pub class_rules: u8,
    #[serde(default)]
    pub talisman_affixes: bool,
    #[serde(default)]
    pub ruby_spells: bool,
    #[serde(default)]
    pub boss_affixes: bool,
    /// Boss 词条规则：1 起可以抽到新增的 3 种词条（吸血、荆棘、迅猛）；旧存档为 0，只用最早的 9 种
    #[serde(default)]
    pub boss_affix_rules: u8,
    #[serde(default)]
    pub scaling_rules: u8,
    #[serde(default,skip_serializing_if="no_law")]
    pub law: law::Law,
    /// 职业传承选用的开局护符（0：旧存档，随身的祖灵角不计入）
    #[serde(default,skip_serializing_if="no_start_talisman")]
    pub start_talisman: u32,
    /// 新旅程：战斗奖励里会出现余烬
    #[serde(default)]
    pub ember_rewards: bool,
    #[serde(default,skip_serializing_if="Vec::is_empty")]
    pub embers: Vec<embers::Ember>,
    /// 余烬规则：1 起按品质抽（embers::offer_by_quality）；旧存档为 0，等概率
    #[serde(default)]
    pub ember_rules: u8,
    /// 新旅程：普通战与精英战的奖励里会出现强化石
    #[serde(default)]
    pub stone_rewards: bool,
    /// 用过调试菜单的旅程：不计入律法纪录，誓约进度不再校验
    #[serde(default,skip_serializing_if="std::ops::Not::not")]
    pub debug: bool,
    /// 2：问号房的每个事件按权重单独抽取，加入骰子赌徒、盲盒商人与词条重铸台三个房间事件（0、1 为旧分配）
    /// 3：九个问号房事件权重相同（2 为试炼 4、其余 2）
    #[serde(default)]
    pub event_rules: u8,
    /// 堕落模式：可沿地图连线向上或向下进入当前关卡相邻、尚未去过的关卡
    #[serde(default,skip_serializing_if="std::ops::Not::not")]
    pub wander: bool,
    /// [HansFix] 堕落值：前几幕继承来的部分（每过一幕减半；本幕的从路线推算，见 descent_path）
    #[serde(default,skip_serializing_if="is_zero")]
    pub descents_before: u32,
    /// [HansFix] 在红色关卡领取、词条被扭曲的装备
    #[serde(default,skip_serializing_if="Vec::is_empty")]
    pub cursed: Vec<descent::Cursed>,
    /// [HansFix] 在堕落首领处立下的新誓约（各自带随机词条）
    #[serde(default,skip_serializing_if="Vec::is_empty")]
    pub fallen_oaths: Vec<fallen_oaths::FallenOath>,
    /// [HansFix] 堕落模式：红篝火、堕落挑战与堕落首领（开启堕落模式的旅程才有）
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub fallen: Option<descent::Fallen>,
    /// [HansFix] 本局拿到的火把（每把一个火把位）
    #[serde(default,skip_serializing_if="Vec::is_empty")]
    pub torches: Vec<torches::Torch>,
    #[serde(default,skip_serializing_if="is_zero")]
    pub torch_rolls: u32,
    #[serde(default="first_act")]
    pub act:u8,
    pub seed: u64,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub initial_seed: Option<u64>,
    #[serde(default)]
    pub completed_nodes: u32,
    #[serde(default)]
    pub completed_battles: u32,
    #[serde(default)]
    pub reached_layer: usize,
    #[serde(default,skip_serializing_if="Option::is_none")]
    personal_reward_seed: Option<u64>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub casting_tools: Option<[bool;2]>,
    /// 上一场战斗胜利时背包里武器的最高强化（0–50，与铁匠同一换算）：决定奖励里骨灰的最高等级
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub weapon_peak: Option<u8>,
    /// 持有过的骨灰最高等级（0–10，只升不降）：战斗胜利时按背包更新，营地调灵后立即更新。
    /// 决定奖励与商店里骨灰的最高等级（+1）；旧存档没有这一项时沿用武器换算
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub spirit_peak: Option<u8>,
    pub class: Class,
    pub nodes: Vec<Node>,
    pub phase: Phase,
    pub visited: Vec<usize>,
    pub hp: u32,
    pub max_hp: u32,
    pub flasks: u8,
    #[serde(default)]
    pub physick_empty: bool,
    pub flask_limit: u8,
    #[serde(default)]
    pub flask_bonus: u8,
    #[serde(default)]
    pub memory_bonus: u8,
    #[serde(default)]
    pub star_stream: bool,
    pub gold: u32,
    pub weapon_level: u8,
    pub blessings: Vec<Blessing>,
    #[serde(default)]
    pub delivered_talismans: Vec<u32>,
    #[serde(default)]
    pub claimed_items: Vec<u32>,
    #[serde(default)]
    pub regional_event: regional_events::Progress,
    #[serde(default)]
    pub contract: Option<contracts::State>,
    #[serde(default)]
    pub larval_gift: act_gifts::LarvalGift,
    #[serde(default)]
    pub elite_buffs: Vec<u32>,
    shop_purchased: u8,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub shop_stock: Option<ShopStock>,
    checkpoint: Option<CombatCheckpoint>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShopStock {pub node:usize,pub items:Vec<[i32;4]>}

fn first_act()->u8 {1}
fn no_law(law:&law::Law)->bool {law.is_empty()}
fn is_zero(n:&u32)->bool {*n==0}
fn no_start_talisman(id:&u32)->bool {*id==0}
pub fn mix(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

fn legacy_graph(seed: u64) -> Vec<Node> {
    (0..36)
        .map(|id| {
            let layer = id / 3;
            let lane = id % 3;
            let kind = match layer {
                0 => NodeKind::Monster,
                3 => [NodeKind::Shop, NodeKind::Chest, NodeKind::Event][lane],
                10 => NodeKind::Camp,
                11 => NodeKind::Boss,
                _ => [
                    NodeKind::Monster,
                    NodeKind::Camp,
                    NodeKind::Shop,
                    NodeKind::Event,
                    NodeKind::Chest,
                    NodeKind::Monster,
                    NodeKind::Elite,
                ][(mix(seed ^ id as u64) % 7) as usize],
            };
            let next = if layer == 11 {
                vec![]
            } else {
                (0..3)
                    .filter(|target| {
                        let right = layer < 2 || mix(seed ^ (layer as u64 * 997)) & 1 == 0;
                        *target == lane || (right && *target == lane + 1) || (!right && lane == *target + 1)
                    })
                    .map(|target| (layer + 1) * 3 + target)
                    .collect()
            };
            Node {
                kind,
                layer,
                lane,
                next,
            }
        })
        .collect()
}

#[cfg(test)]
mod fallen_oath_page {
    use crate::{Class,Phase,Run,fallen_oaths::{Card,FallenOath,NEW}};
    fn at_third_page(seed:u64)->Run {
        let mut run=Run::new(seed,Class::Samurai);run.set_wander(true).unwrap();
        run.debug_fallen_start().unwrap();let boss=run.fallen_id().unwrap();
        run.debug_jump(boss).unwrap();run.enter(boss).unwrap();run.win_combat().unwrap();
        run.claim_reward(0).unwrap();run.decline_reward().unwrap();assert!(run.fallen_oath_pending());run
    }
    #[test]
    fn third_page_offers_new_oaths_with_two_curses_and_one_blessing_and_can_be_skipped() {
        for seed in 0..40u64 {
            let run=at_third_page(seed);
            let cards=run.fallen_oath_cards();assert_eq!(cards.len(),3);
            for card in &cards {
                let Card::New(o)=card else{panic!("no deepen card before holding one")};
                assert!(NEW.contains(&o.kind)&&o.valid()&&o.level==1);
                let lines=o.lines();assert_eq!(lines.len(),4);assert_eq!(lines.iter().filter(|(_,good)|!good).count(),2);
                assert_eq!(o.curse_effects().len(),3);
            }
            let mut skipped=run.clone();skipped.decline_reward().unwrap();
            assert_eq!(skipped.act,2);assert!(skipped.fallen_oaths.is_empty());assert!(!skipped.fallen_oath_pending());
            // 深化：已有一份时多一张深化卡，选它等级 +1，词条不变
            let mut run=run;run.fallen_oaths.push(FallenOath::roll(NEW[0],seed));
            let cards=run.fallen_oath_cards();assert!(matches!(cards.last(),Some(Card::Deepen(0))));
            assert!(cards[..cards.len()-1].iter().all(|c|matches!(c,Card::New(o) if o.kind!=NEW[0])));
            let before=run.fallen_oaths[0];run.choose_fallen_oath(cards.len()-1).unwrap();
            assert_eq!(run.fallen_oaths[0].level,2);assert_eq!((run.fallen_oaths[0].minus,run.fallen_oaths[0].plus),(before.minus,before.plus));
            assert_eq!(run.phase,Phase::Map);assert_eq!(run.act,2);
            assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
        }
    }
    #[test]
    fn curse_texts_and_rows_follow_the_level() {
        use crate::fallen_oaths::{effect,text};
        assert_eq!(text(0,1,false,0),"最大生命 -8%");assert_eq!(text(0,3,true,0),"最大生命 +12%");
        assert_eq!(text(1,2,false,1),"受到火焰伤害 +12%");assert_eq!(text(1,1,true,3),"受到神圣伤害 -10%");
        assert_eq!(text(3,1,false,2),"精力消耗 +8%");assert_eq!(text(5,3,false,2),"战技伤害 -15%");
        assert_eq!(effect(0,1,false,0),9600200);assert_eq!(effect(5,3,true,3),9600257);
        // 属性伤害：每种属性单独的行
        assert_eq!(effect(1,1,false,0),9600260);assert_eq!(effect(1,3,true,3),9600297);
        let elements:std::collections::BTreeSet<u8>=(0..64u64).map(|s|crate::fallen_oaths::FallenOath::roll(crate::contracts::Kind::Iron,s).element).collect();
        assert_eq!(elements.len(),4,"all four elements can be rolled");
    }
}

#[cfg(test)]
mod wander_mode {
    use super::*;
    fn clear(run:&mut Run,id:usize) {
        run.enter(id).unwrap();
        match run.nodes[id].kind {
            NodeKind::Monster|NodeKind::Elite|NodeKind::Boss=>{run.win_combat().unwrap();run.decline_reward().unwrap();},
            NodeKind::Camp=>run.leave_camp().unwrap(),NodeKind::Shop=>run.leave_shop().unwrap(),
            NodeKind::Chest=>run.finish_chest().unwrap(),NodeKind::Event=>run.resolve_event(false).unwrap(),
        }
    }
    fn neighbours(run:&Run,id:usize)->Vec<usize> {
        let mut v:Vec<usize>=run.nodes[id].next.iter().copied().chain((0..run.nodes.len()).filter(|&o|run.nodes[o].next.contains(&id))).filter(|&o|Some(o)!=run.fallen_id()).collect();
        v.sort_unstable();v.dedup();v
    }
    #[test]
    fn toggle_only_before_the_first_node_and_old_saves_default_off() {
        let mut run=Run::new(42,Class::Samurai);
        assert!(!run.wander);assert!(!run.encode().unwrap().contains("wander"));
        run.set_wander(true).unwrap();assert!(run.wander);
        assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
        let first=run.available()[0];clear(&mut run,first);
        assert!(run.set_wander(false).is_err());assert!(run.wander);
    }
    #[test]
    fn wandering_moves_up_and_down_but_never_revisits_and_saves_stay_valid() {
        for seed in 0..40u64 {
            let mut run=Run::new(seed,Class::Samurai);run.set_wander(true).unwrap();
            let mut went_down=false;let mut steps=0;
            while run.act==1&&run.phase==Phase::Map {
                let current=*run.visited.last().unwrap_or(&usize::MAX);
                let options=run.available();assert!(!options.is_empty());
                if Some(current)==run.red_camp() {break;}
                for &o in &options {assert!(!run.visited.contains(&o));}
                if current!=usize::MAX {
                    let near:Vec<usize>=neighbours(&run,current).into_iter().filter(|o|!run.visited.contains(o)).collect();
                    if near.is_empty() {assert!(options.iter().all(|&o|run.nodes[o].kind==NodeKind::Boss));}
                    else {assert_eq!(options,near);}
                }
                // Prefer non-boss nodes, and go down whenever possible, to explore the map.
                let pick=options.iter().copied().filter(|&o|run.nodes[o].kind!=NodeKind::Boss)
                    .min_by_key(|&o|run.nodes[o].layer).unwrap_or(options[0]);
                if current!=usize::MAX&&run.nodes[pick].layer<run.nodes[current].layer {went_down=true;}
                clear(&mut run,pick);steps+=1;
                if run.act==1 {assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);}
            }
            assert_eq!(run.act,2,"seed {seed}: act should end at the boss");
            assert!(run.wander,"mode persists into the next act");
            println!("WANDER_PROBE seed={seed} steps={steps} went_down={went_down}");
            assert!(steps>=1);
        }
    }
    #[test]
    fn descents_are_counted_marked_red_rewarded_like_elites_and_carry_into_the_next_act() {
        let mut checked_curse=0;
        for seed in 0..60u64 {
            let mut run=Run::new(seed,Class::Samurai);run.set_wander(true).unwrap();
            let mut expected=0u32;
            while run.act==1&&run.phase==Phase::Map {
                let options=run.available();
                let pick=options.iter().copied().filter(|&o|run.nodes[o].kind!=NodeKind::Boss)
                    .min_by_key(|&o|run.nodes[o].layer).unwrap_or(options[0]);
                let down=run.descent_choice(pick);
                if let Some(&from)=run.visited.last() {assert_eq!(down,!run.nodes[from].next.contains(&pick)&&run.nodes[pick].kind!=NodeKind::Boss);}
                run.enter(pick).unwrap();
                if down {expected+=1;assert_eq!(run.descent_level(pick),Some(expected));} else {assert_eq!(run.descent_level(pick),None);}
                assert_eq!(run.descents(),expected);
                match run.nodes[pick].kind {
                    NodeKind::Monster|NodeKind::Elite|NodeKind::Boss=>{
                        run.win_combat().unwrap();
                        let Phase::Reward{ref offers,..}=run.phase else{panic!()};let offers=offers.clone();
                        if down&&run.nodes[pick].kind!=NodeKind::Boss {
                            let page=run.elite_page(pick);
                            assert_eq!(offers.len(),descent::offer_count(expected).min(page.len()+1),"options grow with the descent count");
                            assert_eq!(offers[0],page[0],"the elite gear stays the first option");
                            if offers.len()>2 {assert_eq!(offers[2..],page[1..offers.len()-1]);}
                        }
                        assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
                        let cursed=offers.iter().position(|r|run.curse(pick,r).is_some());
                        if let Some(index)=cursed.filter(|_|down) {
                            let plan=run.curse(pick,&offers[index]).unwrap();
                            run.claim_reward(index).unwrap();
                            assert_eq!(run.cursed_item(plan.item),Some(&plan));checked_curse+=1;
                            let loaded=Run::decode(&run.encode().unwrap()).unwrap();assert_eq!(loaded,run);
                        } else {
                            assert!(!down||offers.iter().all(|r|run.curse(pick,r).is_none()));
                            run.decline_reward().unwrap();
                        }
                    },
                    NodeKind::Camp=>run.leave_camp().unwrap(),NodeKind::Shop=>run.leave_shop().unwrap(),
                    NodeKind::Chest=>run.finish_chest().unwrap(),NodeKind::Event=>run.resolve_event(false).unwrap(),
                }
                assert_eq!(run.descents(),if run.act==1 {expected} else {expected/2});
            }
            assert_eq!(run.act,2);assert_eq!(run.descents_before,expected/2);assert_eq!(run.descents(),expected/2,"half the fallen value carries into act 2");
            assert!(run.descent_path().is_empty());
        }
        assert!(checked_curse>0,"some red elite pages offer affixed gear");
    }
    #[test]
    fn red_page_size_follows_the_descent_count_and_casters_can_draw_spells() {
        assert_eq!((1..=12).map(descent::offer_count).collect::<Vec<_>>(),vec![1,1,1,2,2,2,3,3,3,4,4,4]);
        let mut spells=0;
        for seed in 0..80u64 {
            for class in [Class::Astrologer,Class::Prophet,Class::Samurai] {
                let run=Run::new(seed,class);
                if let Reward::Item(id)=descent::extra_offer(&run,3,&[]) {
                    let kind=rewards::lookup(id).unwrap().kind;
                    assert!(kind.allowed(class));
                    if matches!(kind,rewards::ItemKind::Sorcery|rewards::ItemKind::Incantation) {spells+=1;}
                }
            }
        }
        assert!(spells>0,"spells and incantations are in the red pool");
    }
    fn clear_node(run:&mut Run,id:usize) {
        match run.nodes[id].kind {
            NodeKind::Monster|NodeKind::Elite|NodeKind::Boss=>{run.win_combat().unwrap();run.decline_reward().unwrap();},
            NodeKind::Camp=>run.leave_camp().unwrap(),NodeKind::Shop=>run.leave_shop().unwrap(),
            NodeKind::Chest=>run.finish_chest().unwrap(),NodeKind::Event=>run.resolve_event(false).unwrap(),
        }
    }
    #[test]
    fn fallen_mode_map_has_camps_a_red_camp_and_a_locked_fallen_boss() {
        for seed in 0..60u64 {
            let mut run=Run::new(seed,Class::Samurai);let plain=run.nodes.clone();
            run.set_wander(true).unwrap();
            let fallen=run.fallen_id().unwrap();assert_eq!(fallen,plain.len());
            let top=run.nodes.iter().map(|n|n.layer).max().unwrap();
            assert!(run.nodes[..fallen].iter().filter(|n|n.layer+1==top).all(|n|n.kind==NodeKind::Camp));
            assert_eq!(run.nodes[fallen].kind,NodeKind::Boss);assert_eq!(run.nodes[fallen].next,run.start_nodes());
            assert!(run.red_camp().is_some_and(|c|run.nodes[c].layer+1==top));
            assert_eq!(run.floors(),top+1);
            assert!(!run.available().contains(&fallen));
            assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
            run.set_wander(false).unwrap();assert_eq!(run.nodes,plain);assert!(run.fallen.is_none());
        }
    }
    #[test]
    fn red_camp_starts_the_fallen_challenge_down_to_the_fallen_boss_with_two_reward_pages() {
        let mut done=0;let mut shops=0;
        for seed in 0..40u64 {
            let mut run=Run::new(seed,Class::Samurai);run.set_wander(true).unwrap();
            let camp=run.red_camp().unwrap();let fallen=run.fallen_id().unwrap();
            // 沿连线一路向上走到红篝火（BFS 找一条路）
            let mut prev=vec![usize::MAX;run.nodes.len()];let mut queue:std::collections::VecDeque<usize>=run.start_nodes().into();
            for &s in &run.start_nodes() {prev[s]=s;}
            while let Some(n)=queue.pop_front() {for &m in &run.nodes[n].next {if prev[m]==usize::MAX {prev[m]=n;queue.push_back(m);}}}
            if prev[camp]==usize::MAX {continue;}
            let mut path=vec![camp];while prev[*path.last().unwrap()]!=*path.last().unwrap() {path.push(prev[*path.last().unwrap()]);}
            path.reverse();
            for &id in &path {assert!(run.available().contains(&id),"seed {seed}");run.enter(id).unwrap();clear_node(&mut run,id);}
            let before=run.descents();
            let back=path[path.len()-2];
            assert!(run.available().contains(&back),"red camp offers the visited neighbour");
            let kinds_layer0:Vec<_>=run.start_nodes().iter().map(|&i|run.nodes[i].kind).collect();
            let back_kind=run.nodes[back].kind;
            run.enter(back).unwrap();
            assert!(run.fallen_active());assert_eq!(run.visited,vec![camp]);
            assert_eq!(run.nodes[back].kind,back_kind,"seed {seed}: the chosen entry keeps its kind (a shop stays a shop)");
            if back_kind==NodeKind::Shop {assert!(run.red_shop(back),"an entry shop is a fallen shop");}
            assert_eq!(run.start_nodes().iter().map(|&i|run.nodes[i].kind).collect::<Vec<_>>(),kinds_layer0);
            assert_eq!(run.descents(),before+1);
            assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
            clear_node(&mut run,back);
            let mut guard=0;
            while run.act==1 {
                guard+=1;assert!(guard<40);
                let options=run.available();assert!(!options.is_empty());
                let cur=*run.visited.last().unwrap();
                for &o in &options {assert!(o==fallen||run.nodes[o].next.contains(&cur),"only downward");}
                let pick=options[0];run.enter(pick).unwrap();
                if pick==fallen {
                    let level=run.descent_level(fallen).unwrap();assert!(level>before+1);
                    run.win_combat().unwrap();
                    for page in 0..2 {
                        let Phase::Reward{ref offers,..}=run.phase else{panic!()};let offers=offers.clone();
                        assert_eq!(offers.len(),3);
                        for r in &offers {let c=run.curse(fallen,r).unwrap();
                            assert_eq!(c.swaps.iter().map(|s|descent::factor(s.1)).filter(|f|*f==2).count(),2);
                            assert_eq!(c.swaps.iter().filter(|s|descent::factor(s.1)==-2).count(),1);}
                        assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
                        run.claim_reward(0).unwrap();
                        if page==0 {assert!(matches!(run.phase,Phase::Reward{..}));assert_eq!(run.fallen.as_ref().unwrap().page,1);}
                    }
                    // 第三页：堕落誓约
                    assert!(run.fallen_oath_pending());assert_eq!(run.act,1);
                    let cards=run.fallen_oath_cards();assert_eq!(cards.len(),3);
                    assert!(matches!(run.phase,Phase::Reward{ref offers,..} if offers.is_empty()));
                    assert!(run.claim_reward(0).is_err());
                    assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
                    let erdtree_tower_card=cards[0];
                    run.choose_fallen_oath(0).unwrap();
                    let fallen_oaths::Card::New(chosen)=erdtree_tower_card else{panic!()};
                    assert_eq!(run.fallen_oaths,vec![chosen]);assert!(chosen.valid());
                    assert_eq!(run.act,2,"the fallen boss clears the act");assert!(run.cursed.len()>=2);
                    assert!(run.fallen.as_ref().is_some_and(|f|!f.active&&f.page==0));
                    assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
                    done+=1;
                } else {
                    if run.nodes[pick].kind==NodeKind::Shop {
                        assert!(run.red_shop(pick));
                        if run.claimed_items.contains(&160030000) {} else {
                            run.keep_shop_gear(pick,160030000).unwrap();assert!(run.cursed_item(160030000).is_some());
                        }
                        let bonus=run.grant_shop_bonus(pick).unwrap();
                        assert_eq!(run.cursed_item(bonus).unwrap().swaps.iter().filter(|s|descent::factor(s.1)==-2).count(),1);
                        assert!(run.grant_shop_bonus(pick).is_err(),"once per shop");
                        assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
                        shops+=1;
                    }
                    clear_node(&mut run,pick);assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
                }
            }
        }
        assert!(done>10,"{done}");assert!(shops>0,"fallen shops were visited");
    }
    #[test]
    fn upward_only_runs_never_descend_and_tampered_curses_are_rejected() {
        let mut run=Run::new(5,Class::Samurai);
        let first=run.available()[0];run.enter(first).unwrap();assert_eq!(run.descents(),0);
        let mut bad=Run::new(5,Class::Samurai);bad.claimed_items.push(160010000);
        bad.cursed.push(descent::Cursed{item:160010000,swaps:vec![(9850190,9850191)]});
        assert!(Run::decode(&bad.encode().unwrap()).is_err());
    }
    #[test]
    fn surrounded_player_is_sent_to_the_boss() {
        let mut run=Run::new(7,Class::Samurai);run.set_wander(true).unwrap();
        let start=run.available()[0];
        // Pretend every neighbour of the start node has already been visited.
        run.visited=vec![start];
        let route=run.wander_route(&run.visited.iter().copied().chain(neighbours(&run,start)).collect::<Vec<_>>(),start);
        assert!(!route.is_empty());assert!(route.iter().all(|&id|run.nodes[id].kind==NodeKind::Boss));
    }
    #[test]
    fn every_act_map_fits_the_party_report_limit() {
        let most=(0..500u64).map(|seed|camp_graph(seed).len()).max().unwrap();
        println!("WANDER_PROBE most_nodes={most}");
        assert!(most<=crate::party_game::MAX_VISITED,"{most}");
    }
}

#[cfg(test)]
mod map_compatibility {
    use super::*;
    #[test]
    fn old_act_keeps_twelve_floors_then_advances_to_sixteen() {
        for schema in 1..=3 {
            let mut run=Run::new(42,Class::Samurai);run.schema=schema;
            run.nodes=match schema {1=>legacy_graph(42),2=>graph(42),_=>balanced_graph(42)};
            let original=run.nodes.clone();
            for step in 0..12 {
                run=Run::decode(&run.encode().unwrap()).unwrap();
                assert_eq!(run.nodes,original);assert_eq!(run.visited.len(),step);
                let id=run.available()[0];run.enter(id).unwrap();
                match run.nodes[id].kind {
                    NodeKind::Monster|NodeKind::Elite|NodeKind::Boss=>{run.win_combat().unwrap();run.decline_reward().unwrap();},
                    NodeKind::Camp=>run.leave_camp().unwrap(),NodeKind::Shop=>run.leave_shop().unwrap(),
                    NodeKind::Chest=>run.finish_chest().unwrap(),NodeKind::Event=>run.resolve_event(false).unwrap(),
                }
            }
            assert_eq!(run.act,2);assert_eq!(run.floors(),16);assert_eq!(run.schema,6);
            assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
        }
    }
    #[test]
    fn legacy_map_roundtrips_without_rewriting_routes() {
        let mut run=Run::new(42,Class::Samurai);run.schema=1;run.nodes=legacy_graph(run.seed);
        assert_eq!(run.available(),vec![0]);
        run.enter(0).unwrap();run.win_combat().unwrap();
        assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
        run.decline_reward().unwrap();assert_eq!(run.available(),vec![3,4]);
        assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
    }
    #[test]
    fn increased_camps_preserve_old_saves_and_spacing() {
        let mut old_count=0;let mut new_count=0;
        for seed in 0..1000 {
            let mut old=Run::new(seed,Class::Samurai);old.schema=5;old.nodes=spaced_graph(seed);
            assert_eq!(Run::decode(&old.encode().unwrap()).unwrap(),old);
            let new=Run::new(seed,Class::Samurai);
            assert_eq!(Run::decode(&new.encode().unwrap()).unwrap(),new);
            old_count+=old.nodes.iter().filter(|n|n.kind==NodeKind::Camp&&n.layer<14).count();
            new_count+=new.nodes.iter().filter(|n|n.kind==NodeKind::Camp&&n.layer<14).count();
            for (id,node) in new.nodes.iter().enumerate() {
                assert_eq!(node.next,old.nodes[id].next);
                if matches!(node.kind,NodeKind::Camp|NodeKind::Chest){assert!(node.next.iter().all(|&next|!matches!(new.nodes[next].kind,NodeKind::Camp|NodeKind::Chest)));}
                if node.layer==14{assert_eq!(node.kind,NodeKind::Camp);}
            }
        }
        assert!(new_count*100>old_count*150,"camps: old={old_count}, new={new_count}");
        println!("Random camps over 1000 seeds: old={old_count}, new={new_count}");
    }
    #[test]
    fn spaced_routes_keep_links_and_boss_camp_and_old_saves() {
        for seed in 0..1000 {
            let mut old=Run::new(seed,Class::Samurai);old.schema=4;old.nodes=expedition_graph(seed);
            assert_eq!(Run::decode(&old.encode().unwrap()).unwrap(),old);
            let mut run=Run::new(seed,Class::Samurai);
            for act in 1..=3 {
                assert_eq!(run.act,act);run.validate().unwrap();
                assert_eq!(Run::decode(&run.encode().unwrap()).unwrap(),run);
                let original=expedition_graph(run.seed);
                for (id,node) in run.nodes.iter().enumerate() {
                    assert_eq!(node.next,original[id].next);
                    if node.layer==14 {assert_eq!(node.kind,NodeKind::Camp);}
                    if matches!(node.kind,NodeKind::Chest|NodeKind::Camp) {
                        for &next in &node.next {assert!(!matches!(run.nodes[next].kind,NodeKind::Chest|NodeKind::Camp));}
                    }
                }
                if act<3 {let boss=run.nodes.len()-1;run.finish(boss);}
            }
        }
    }
    #[test]
    fn previous_routes_remain_stable_and_new_routes_reduce_services() {
        let service=|n:&&Node|matches!(n.kind,NodeKind::Camp|NodeKind::Shop|NodeKind::Chest);
        let mut old_count=0;let mut new_count=0;
        for seed in 0..200 {
            let mut old=Run::new(seed,Class::Samurai);old.schema=2;old.nodes=graph(seed);
            assert_eq!(Run::decode(&old.encode().unwrap()).unwrap(),old);
            let mut new=Run::new(seed,Class::Samurai);new.schema=3;new.nodes=balanced_graph(seed);assert_eq!(Run::decode(&new.encode().unwrap()).unwrap(),new);
            old_count+=old.nodes.iter().filter(service).count();new_count+=new.nodes.iter().filter(service).count();
            assert!(new.nodes.iter().filter(|n|n.layer==10).all(|n|n.kind==NodeKind::Camp));
        }
        assert!(new_count*100<old_count*85);
        assert!(new_count*100>old_count*60);
    }
}

fn graph(seed:u64)->Vec<Node> {
    let mut nodes=legacy_graph(seed);
    nodes.truncate(34);
    nodes[33].lane=1;
    let lanes=|layer:usize|->Vec<usize> {if layer==0||layer==2||layer<10&&mix(seed^layer as u64^319)%3==0 {vec![0,1,2]}else{vec![0,2]}};
    for layer in 0..10 {
        let from=lanes(layer);let to=lanes(layer+1);
        let right=mix(seed^(layer as u64*997))&1==0;
        let lane=if right {(mix(seed^layer as u64^71)%2) as usize}else{1+(mix(seed^layer as u64^71)%2) as usize};
        for node in &mut nodes[layer*3..layer*3+3] {
            let targets=if let Some(rank)=from.iter().position(|&l|l==node.lane) {
                match (from.len(),to.len()) {
                    (3,2)=>match rank {0=>vec![0],1=>vec![0,1],_=>vec![1]},
                    (2,3)=>if rank==0 {vec![0,1]}else{vec![1,2]},
                    _=>{let mut v=vec![rank];if rank==lane.min(from.len()-1) {let next=if right{rank+1}else{rank.saturating_sub(1)};if next<to.len()&&!v.contains(&next){v.push(next);}}v},
                }
            }else{vec![node.lane.min(to.len()-1)]};
            node.next=targets.into_iter().map(|rank|(layer+1)*3+to[rank]).collect();node.next.sort_unstable();
        }
        if from.len()==2 {nodes[layer*3+1].next=vec![*nodes[layer*3].next.last().unwrap()];}
    }
    for node in &mut nodes[30..33] {node.next=vec![33];}
    nodes
}

fn expedition_graph(seed:u64)->Vec<Node> {
    let mut nodes=Vec::new();let mut rows=Vec::new();
    let pool=[NodeKind::Monster,NodeKind::Monster,NodeKind::Monster,NodeKind::Monster,NodeKind::Elite,NodeKind::Event,NodeKind::Event,NodeKind::Camp,NodeKind::Shop,NodeKind::Chest];
    for layer in 0..16 {
        let count=if layer==15 {1}else{4+(mix(seed^layer as u64^319)%2) as usize};
        let start=nodes.len();rows.push((start,count));
        for lane in 0..count {
            let kind=match layer {0=>NodeKind::Monster,14=>NodeKind::Camp,15=>NodeKind::Boss,_=>pool[(mix(seed^nodes.len() as u64)%pool.len() as u64) as usize]};
            nodes.push(Node{kind,layer,lane,next:vec![]});
        }
    }
    for layer in 0..15 {
        let (start,n)=rows[layer];let (next,m)=rows[layer+1];
        for i in 0..n {
            nodes[start+i].next=(0..m).filter(|&j|i*m<(j+1)*n&&j*n<(i+1)*m).map(|j|next+j).collect();
        }
        if n==m {
            let lane=(mix(seed^layer as u64^997)%(n-1) as u64) as usize;
            nodes[start+lane].next.push(next+lane+1);
        }
    }
    nodes
}

fn balanced_graph(seed:u64)->Vec<Node> {
    let mut nodes=graph(seed);
    let pool=[NodeKind::Monster,NodeKind::Monster,NodeKind::Monster,NodeKind::Monster,NodeKind::Elite,NodeKind::Event,NodeKind::Event,NodeKind::Camp,NodeKind::Shop,NodeKind::Chest];
    for (id,node) in nodes.iter_mut().enumerate().filter(|(_,n)|(1..10).contains(&n.layer)) {
        node.kind=if node.layer==3 {[NodeKind::Shop,NodeKind::Monster,NodeKind::Event][node.lane]}else{pool[(mix(seed^id as u64)%pool.len() as u64) as usize]};
    }
    nodes
}

fn spaced_graph(seed:u64)->Vec<Node> {
    space_rest_nodes(expedition_graph(seed))
}

pub(crate) fn camp_graph(seed:u64)->Vec<Node> {
    let mut nodes=expedition_graph(seed);
    for (id,node) in nodes.iter_mut().enumerate() {
        if (1..14).contains(&node.layer)&&mix(seed^id as u64)%10==0 {node.kind=NodeKind::Camp;}
    }
    space_rest_nodes(nodes)
}

pub(crate) fn space_rest_nodes(mut nodes:Vec<Node>)->Vec<Node> {
    let rest=|kind|matches!(kind,NodeKind::Chest|NodeKind::Camp);
    for id in (0..nodes.len()).rev() {
        if rest(nodes[id].kind)&&nodes[id].next.iter().any(|&next|rest(nodes[next].kind)) {
            nodes[id].kind=NodeKind::Monster;
        }
    }
    nodes
}

impl Run {
    pub fn floors(&self)->usize {self.nodes.last().map_or(0,|n|n.layer+1)}
    pub fn new(seed: u64, class: Class) -> Self {
        Self {
            schema: 6,
            affix_equipment: true,
            affix_rules: 5,
            boss_reward_rules: 1,
            class_rules: 1,
            talisman_affixes: true,
            ruby_spells: false,
            boss_affixes: true,
            boss_affix_rules: 1,
            scaling_rules: 1,
            law: law::Law::default(),
            start_talisman: 0,
            ember_rewards: true,
            ember_rules: 1,
            stone_rewards: true,
            embers: vec![],
            debug: false,
            event_rules: 3,
            wander: false,
            descents_before: 0,
            cursed: Vec::new(),
            fallen_oaths: Vec::new(),
            fallen: None,
            torches: Vec::new(),
            torch_rolls: 0,
            act:1,
            seed,
            initial_seed: Some(seed),
            completed_nodes: 0,
            completed_battles: 0,
            reached_layer: 0,
            personal_reward_seed: None,
            casting_tools: None,
            weapon_peak: None,
            spirit_peak: Some(0),
            class,
            nodes: camp_graph(seed),
            phase: Phase::Map,
            visited: vec![],
            hp: 100,
            max_hp: 100,
            flasks: 10,
            flask_limit: 10,
            flask_bonus: 0,
            memory_bonus: 0,
            star_stream: false,
            gold: 0,
            weapon_level: 0,
            blessings: vec![],
            delivered_talismans: vec![],
            claimed_items: vec![],
            physick_empty: false,
            regional_event: Default::default(),
            contract: None,
            larval_gift: Default::default(),
            elite_buffs: vec![],
            shop_purchased: 0,
            shop_stock: None,
            checkpoint: None,
        }
    }

    pub fn new_personal(seed:u64,class:Class,reward_seed:u64)->Self {
        let mut run=Self::new(seed,class);
        run.personal_reward_seed=Some(reward_seed);
        run
    }

    /// 开局时立下苦难律法；只能在第一幕起点、尚未行动时设置
    pub fn set_law(&mut self,law:law::Law)->RuleResult<()> {
        if !law.valid() {return Err("苦难律法无效".into());}
        if self.act!=1||!self.visited.is_empty()||self.phase!=Phase::Map {return Err("苦难律法只能在开局时立下".into());}
        let full=self.flasks==self.flask_limit;
        self.law=law;self.flask_limit=self.full_flask_limit();
        self.flasks=if full {self.flask_limit}else{self.flasks.min(self.flask_limit)};
        Ok(())
    }
    /// 职业重做后的召唤师（群灵：胜利时召唤物还活着就不消耗骨灰；骨灰奖励更多）
    /// 旅程进度：(当前是第几层，从 0 起, 最后一层的序号)；三幕连续计数，还没进入节点时按本幕第 1 层算
    pub fn journey_progress(&self)->(u32,u32) {
        let per_act=self.nodes.iter().map(|n|n.layer+1).max().unwrap_or(1) as u32;
        let step=(u32::from(self.act.clamp(1,3))-1)*per_act+(self.reached_layer as u32).saturating_sub(1).min(per_act-1);
        (step,(3*per_act-1).max(1))
    }
    /// 商店价格随旅程位置浮动的百分比：第一幕第 1 层 100%，按层线性涨到第三幕最后一层 SHOP_PRICE_END
    pub fn shop_position_percent(&self)->u32 {
        let (step,total)=self.journey_progress();
        (100*(total-step)+SHOP_PRICE_END*step+total/2)/total
    }
    /// 职业重做前的旧存档（旧职业，或 class_rules 为 0）：不再继续，由大赐福备份后按没有存档处理
    pub fn retired_save(text:&str)->bool {
        let Ok(value)=serde_json::from_str::<serde_json::Value>(text) else{return false;};
        let class=value.get("class").cloned().map(serde_json::from_value::<Class>);
        matches!(class,Some(Err(_)))||value.get("class_rules").and_then(|v|v.as_u64()).unwrap_or(0)==0
    }
    pub fn summoner(&self)->bool {self.class==Class::Confessor}
    pub fn full_flask_limit(&self)->u8 {let cut=self.law.flask_cut();10+self.flask_bonus*2-cut[0]-cut[1]}

    pub fn reward_seed(&self)->u64 {
        self.personal_reward_seed.map_or(self.seed,|seed|mix(seed ^ (u64::from(self.act)<<48) ^ 0x726577617264))
    }

    pub fn align_party_start(&mut self,seed:u64)->RuleResult<()> {
        self.validate()?;
        if self.seed==seed {return Ok(());}
        if self.act!=1||!self.visited.is_empty()||self.phase!=Phase::Map {
            return Err("已有旅程不能替换路线，请从大赐福开始共同旅程".into());
        }
        let mut next=self.clone();next.seed=seed;next.initial_seed=Some(seed);next.schema=6;next.nodes=camp_graph(seed);
        next.validate()?;*self=next;Ok(())
    }

    pub fn available(&self) -> Vec<usize> {
        if self.phase != Phase::Map || self.contract_pending() {
            return vec![];
        }
        self.next_route()
    }

    fn next_route(&self) -> Vec<usize> {
        match self.visited.last() {
            Some(&id) if self.fallen_active() => self.fallen_route(&self.visited,id),
            Some(&id) if self.wander => {
                let mut route=self.wander_route(&self.visited,id);
                // 红篝火：选择相邻、走过的关卡即开启堕落挑战
                if Some(id)==self.red_camp() {
                    route.extend((0..self.nodes.len()).filter(|&o|self.nodes[o].next.contains(&id)&&self.visited.contains(&o)&&Some(o)!=self.fallen_id()));
                    route.sort_unstable();route.dedup();
                }
                route
            },
            Some(id) => self.nodes[*id].next.clone(),
            None => self.start_nodes(),
        }
    }

    /// 堕落：当前关卡沿连线上下相邻、且尚未去过的关卡；四周都去过时只能挑战首领
    fn wander_route(&self,visited:&[usize],id:usize)->Vec<usize> {
        let fallen=self.fallen_id();
        let mut route:Vec<usize>=self.nodes[id].next.iter().copied()
            .chain((0..self.nodes.len()).filter(|&other|self.nodes[other].next.contains(&id)))
            .filter(|other|!visited.contains(other)&&Some(*other)!=fallen).collect();
        route.sort_unstable();route.dedup();
        if route.is_empty() {
            route=(0..self.nodes.len()).filter(|&other|self.nodes[other].kind==NodeKind::Boss&&!visited.contains(&other)&&Some(other)!=fallen).collect();
        }
        route
    }
    /// [HansFix] 堕落挑战：只能沿连线向下进入没去过的关卡；起点那排向下就是堕落首领
    pub(crate) fn fallen_route(&self,visited:&[usize],id:usize)->Vec<usize> {
        if Some(id)==self.fallen_id() {return vec![];}
        let route:Vec<usize>=(0..self.nodes.len()).filter(|&o|self.nodes[o].next.contains(&id)&&!visited.contains(&o)).collect();
        if route.is_empty() {self.fallen_id().filter(|f|!visited.contains(f)).into_iter().collect()} else {route}
    }
    /// 堕落首领的节点编号（堕落模式地图的最后一个节点）
    pub fn fallen_id(&self)->Option<usize> {self.fallen.as_ref().map(|_|self.nodes.len()-1)}
    pub fn fallen_active(&self)->bool {self.fallen.as_ref().is_some_and(|f|f.active)}
    pub fn red_camp(&self)->Option<usize> {self.fallen.as_ref().and_then(|_|descent::red_camp(&self.nodes,self.seed))}
    /// 这一幕的地图（堕落模式下按当前状态加上篝火、堕落首领与洗牌）
    pub(crate) fn expected_nodes(&self)->Vec<Node> {
        let base=camp_graph(self.seed);
        match &self.fallen {Some(f)=>descent::fallen_graph(base,self.seed,f.active,f.entry),None=>base}
    }
    /// 红篝火处选择了走过的相邻关卡：开启堕落挑战
    pub(crate) fn begin_fallen(&mut self,entry:usize)->RuleResult<()> {
        let camp=self.red_camp().ok_or("没有红篝火")?;
        self.descents_before+=self.descent_path().len() as u32;
        if let Some(f)=self.fallen.as_mut() {f.active=true;f.page=0;f.entry=Some(entry);}
        self.nodes=self.expected_nodes();
        self.visited=vec![camp];
        Ok(())
    }

    /// [HansFix] 堕落：从 from 进入 to 是否为“向下”（不是沿连线向上；四周走完后去首领不算）
    pub fn is_descent(&self,from:usize,to:usize)->bool {
        self.wander&&self.nodes.get(from).is_some_and(|n|!n.next.contains(&to))&&self.nodes.get(to).is_some_and(|n|n.kind!=NodeKind::Boss||Some(to)==self.fallen_id())
    }
    /// 本幕已走路线（含正在进行的关卡）里向下进入的关卡，按先后
    pub fn descent_path(&self)->Vec<usize> {
        let mut path=self.visited.clone();
        if let Phase::Combat{id}|Phase::Node{id}|Phase::Reward{id,..}=self.phase {if path.last()!=Some(&id) {path.push(id);}}
        path.windows(2).filter(|w|self.is_descent(w[0],w[1])).map(|w|w[1]).collect()
    }
    /// 当前堕落值（继承的一半 + 本幕向下次数）
    pub fn descents(&self)->u32 {self.descents_before+self.descent_path().len() as u32}
    /// 红色关卡：是第几次向下（整局计数，从 1 开始）
    pub fn descent_level(&self,id:usize)->Option<u32> {
        self.descent_path().iter().position(|&n|n==id).map(|i|self.descents_before+i as u32+1)
    }
    /// 地图上可选的关卡里，哪些是向下（红色高亮）
    pub fn descent_choice(&self,id:usize)->bool {self.visited.last().is_some_and(|&from|self.is_descent(from,id))}

    /// 开启或关闭堕落模式；只能在第一幕起点、尚未行动时切换
    pub fn set_wander(&mut self,on:bool)->RuleResult<()> {
        if self.act!=1||!self.visited.is_empty()||self.phase!=Phase::Map {return Err("堕落模式只能在开局、选择第一个关卡前切换".into());}
        self.wander=on;self.apply_fallen_mode();Ok(())
    }
    /// 联机客机跟随房主的堕落设置（由房主的进入指令携带）
    pub fn follow_wander(&mut self,on:bool) {
        self.wander=on;
        if self.act==1&&self.visited.is_empty()&&self.schema==6 {self.apply_fallen_mode();}
    }
    /// 堕落模式开关后换上对应的地图（只在第一幕开局时调用）
    fn apply_fallen_mode(&mut self) {
        if self.schema!=6 {return;}
        self.fallen=self.wander.then(descent::Fallen::default);
        self.nodes=self.expected_nodes();
    }

    pub fn start_nodes(&self)->Vec<usize> {if self.schema==1 {vec![0]}else if self.schema<4 {vec![0,1,2]}else{self.nodes.iter().enumerate().take_while(|(_,n)|n.layer==0).map(|(id,_)|id).collect()}}

    pub fn enter(&mut self, id: usize) -> RuleResult<()> {
        if !self.available().contains(&id) {
            return Err("节点不可达".into());
        }
        if !self.fallen_active()&&self.visited.last().is_some_and(|&l|Some(l)==self.red_camp())&&self.visited.contains(&id) {self.begin_fallen(id)?;}
        self.reached_layer=self.reached_layer.max(self.nodes[id].layer+1);
        self.shop_purchased = 0;self.shop_stock=None;
        if self.nodes[id].kind==NodeKind::Event {
            let draw=self.event_draw(id);
            self.regional_event.scarabs=draw==events::Draw::Scarabs;
            self.regional_event.locked=draw==events::Draw::Locked;
            self.regional_event.trial=draw==events::Draw::Trial;
            self.regional_event.recovery=draw==events::Draw::Spring;

        }
        if self.nodes[id].kind.combat() {
            self.checkpoint = Some(CombatCheckpoint {
                id,
                hp: self.hp,
                flasks: self.flasks,
            });
            self.phase = Phase::Combat { id };
        } else {
            self.phase = Phase::Node { id };
        }
        Ok(())
    }

    fn active_node(&self, kind: NodeKind) -> RuleResult<usize> {
        match self.phase {
            Phase::Node { id } if self.nodes.get(id).is_some_and(|n| n.kind == kind) => Ok(id),
            _ => Err("当前节点不允许此操作".into()),
        }
    }

    fn finish(&mut self, id: usize) {
        self.completed_nodes=self.completed_nodes.saturating_add(1);
        if matches!(self.nodes[id].kind,NodeKind::Monster|NodeKind::Elite|NodeKind::Boss){self.completed_battles=self.completed_battles.saturating_add(1);}
        self.regional_event=Default::default();
        self.visited.push(id);
        self.checkpoint = None;
        self.shop_purchased = 0;self.shop_stock=None;
        if self.nodes[id].kind==NodeKind::Boss && self.act<3 {
            // [HansFix] 堕落值：进入下一幕只继承一半（向下取整）
            self.descents_before=(self.descents_before+self.descent_path().len() as u32)/2;
            self.act+=1;
            self.reached_layer=0;
            self.hp=self.max_hp;
            self.flasks=self.flask_limit;
            self.seed=mix(self.seed^u64::from(self.act));
            if let Some(old)=&self.contract {self.contract=Some(old.next_act(self.reward_seed(),self.class,self.act));}
            self.schema=6;
            self.nodes=camp_graph(self.seed);
            if self.fallen.is_some() {self.fallen=Some(descent::Fallen::default());self.nodes=self.expected_nodes();}
            self.visited.clear();
            self.phase=Phase::Map;
            return;
        }
        self.phase = if self.nodes[id].kind == NodeKind::Boss {
            Phase::Won
        } else {
            Phase::Map
        };
    }

    pub fn use_flask(&mut self) -> RuleResult<()> {
        if !matches!(self.phase, Phase::Combat { .. }) || self.flasks == 0 {
            return Err("当前无法使用圣杯瓶".into());
        }
        self.flasks -= 1;
        Ok(())
    }

    // The game bridge owns healing and reports actual health; the rule engine never invents it.
    pub fn observe_hp(&mut self, hp: u32) -> RuleResult<()> {
        if !matches!(self.phase, Phase::Combat { .. }) || hp > self.max_hp {
            return Err("无效生命状态".into());
        }
        self.hp = hp;
        if hp == 0 {
            self.phase = Phase::Dead;
            self.checkpoint = None;
        }
        Ok(())
    }

    pub fn camp(&mut self, choice: CampChoice) -> RuleResult<()> {
        let id = self.active_node(NodeKind::Camp)?;
        match choice {
            CampChoice::Refill | CampChoice::RefillBlue | CampChoice::RefillRedOnly | CampChoice::RefillBlueOnly => {
                if self.flasks >= self.flask_limit && !self.physick_empty {
                    return Err("圣杯瓶已满".into());
                }
                self.flasks = self.flasks.saturating_add(3).min(self.flask_limit);
                self.physick_empty=false;
            }
            CampChoice::UpgradeWeapon => {
                if self.weapon_level >= 25 {
                    return Err("武器已达到上限".into());
                }
                self.weapon_level += 1;
            }
            CampChoice::UpgradeBlessing(index) => {
                let blessing = self.blessings.get_mut(index).ok_or("请选择已有赐福")?;
                if blessing.level >= 3 {
                    return Err("赐福已达到上限".into());
                }
                blessing.level += 1;
            }
        }
        self.finish(id);
        Ok(())
    }

    pub fn leave_camp(&mut self) -> RuleResult<()> {
        let id = self.active_node(NodeKind::Camp)?;
        self.finish(id);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn offers_for_test(&self,id:usize)->Vec<Reward> {self.offers(id)}
    fn offers(&self, id: usize) -> Vec<Reward> {
        // [HansFix] 堕落首领：两页三件“两个词条 ×2、一个 ×(-2)”的传说装备
        if Some(id)==self.fallen_id() {let page=self.fallen.as_ref().map_or(0,|f|f.page);return if page>=2 {vec![]} else {descent::fallen_offers(self,id,page)};}
        // [HansFix] 红色战斗关卡：精英结算，选项数量随向下次数增加
        if let Some(level)=self.red_combat(id) {return self.descent_offers(id,level);}
        let mut offers=rewards::offers(self,id);
        if !self.law.is_empty() {
            for reward in &mut offers {
                if self.law.lone_wolf()&&matches!(reward,Reward::Item(item) if rewards::lookup(*item).is_some_and(|i|i.kind==rewards::ItemKind::Spirit)) {*reward=Reward::Gold(1500);}
                if let Reward::Gold(amount)=reward {*amount=self.law.runes(*amount);}
            }
            if self.law.stingy()&&self.nodes.get(id).is_some_and(|n|n.kind.combat()) {offers.truncate(2);}
        }
        // 强化石：普通战与精英战有 STONE_CHANCE% 的概率把中间那个奖励换成强化石（余烬占最后一个）
        if self.stone_rewards&&self.nodes.get(id).is_some_and(|n|matches!(n.kind,NodeKind::Monster|NodeKind::Elite))
            &&mix(self.reward_seed()^(id as u64).wrapping_mul(7919)^0x5701_e5a1)%100<STONE_CHANCE {
            let middle=offers.len()/2;if let Some(slot)=offers.get_mut(middle) {*slot=Reward::UpgradeStone;}
        }
        if let Some(ember)=embers::reward(self,id) {if let Some(last)=offers.last_mut() {*last=Reward::Ember(ember);}}
        offers
    }
    fn legacy_offers(&self, id: usize) -> Vec<Reward> {
        let start = (mix(self.seed ^ (id as u64 + 100)) % 8) as usize;
        let mut offers = Vec::new();
        for offset in 0..8 {
            let kind = BlessingKind::ALL[(start + offset) % 8];
            if !self
                .blessings
                .iter()
                .any(|b| b.kind == kind && b.level == 3)
            {
                offers.push(Reward::Blessing(kind));
                if offers.len() == 2 {
                    break;
                }
            }
        }
        while offers.len() < 2 {
            offers.push(Reward::Gold(75 + offers.len() as u32 * 25));
        }
        offers.push(if self.weapon_level < 25 {
            Reward::UpgradeWeapon
        } else {
            Reward::Gold(100)
        });
        offers
    }

    pub fn win_combat(&mut self) -> RuleResult<()> {
        let Phase::Combat { id } = self.phase else {
            return Err("当前没有进行中的战斗".into());
        };
        let kind = self.nodes.get(id).ok_or("无效节点")?.kind;
        if !kind.combat() || self.hp == 0 {
            return Err("战斗状态无效".into());
        }
        self.checkpoint = None;
        self.phase = Phase::Reward {
            id,
            offers: self.offers(id),
        };
        Ok(())
    }

    fn grant(&mut self, reward: Reward) -> RuleResult<()> {
        match reward {
            Reward::FlaskCapacity=>{
                if self.flask_bonus>=4{return Err("圣杯瓶容量已达上限".into());}
                self.flask_bonus+=1;self.flask_limit=self.full_flask_limit();
            },
            Reward::MemorySlot=>{
                if self.memory_bonus>=6{return Err("记忆栏位已达上限".into());}
                self.memory_bonus+=1;
            },
            Reward::StarStream=>{
                if self.star_stream{return Err("已获得星流引导".into());}
                self.star_stream=true;
            },
            Reward::SacredTear|Reward::UpgradeStone=>{},
            Reward::Ember(ember)=>{
                if !self.ember_rewards||self.embers.contains(&ember){return Err("余烬已获得或无效".into());}
                self.embers.push(ember);
            },
            Reward::TalismanBuff(id)=>{
                if !elite::BUFFS.contains(&id)||self.elite_buffs.contains(&id){return Err("护符效果已获得或无效".into());}
                self.elite_buffs.push(id);
            },
            Reward::Blessing(kind) => {
                if let Some(b) = self.blessings.iter_mut().find(|b| b.kind == kind) {
                    if b.level >= 3 {
                        return Err("赐福已达到上限".into());
                    }
                    b.level += 1;
                } else {
                    self.blessings.push(Blessing { kind, level: 1 });
                }
            }
            Reward::UpgradeWeapon => {
                if self.weapon_level >= 25 {
                    return Err("武器已达到上限".into());
                }
                self.weapon_level += 1;
            }
            Reward::Gold(amount) => self.gold = self.gold.saturating_add(amount),
            Reward::Item(id)=>{
                let item=rewards::lookup(id).ok_or("未知奖励物品")?;
                let boss=matches!(self.phase,Phase::Reward{id:node,..} if self.nodes[node].kind==NodeKind::Boss)&&elite::LEGENDARY.contains(&affixes::base(id));
                if (!item.kind.allowed(self.class)&&!boss)||(self.claimed_items.contains(&id)&&!(boss&&item.kind==rewards::ItemKind::Weapon)) {return Err("该奖励不可领取".into());}
                if !self.claimed_items.contains(&id){self.claimed_items.push(id);}
            },
        }
        Ok(())
    }

    pub fn claim_reward(&mut self, index: usize) -> RuleResult<()> {
        let Phase::Reward { id, ref offers } = self.phase else {
            return Err("当前没有待领取奖励".into());
        };
        let reward = offers.get(index).ok_or("无效奖励选项")?.clone();
        let curse=self.curse(id,&reward);
        self.grant(reward)?;
        if let Some(curse)=curse {self.cursed.retain(|c|c.item!=curse.item);self.cursed.push(curse);}
        if self.fallen_second_page(id) {return Ok(());}
        self.finish(id);
        Ok(())
    }

    /// [HansFix] 红色战斗关卡（向下进入的普通或精英战斗）：第几次向下
    /// [HansFix] 堕落关卡（红色战斗关卡或堕落首领）结算卢恩：这一场拿到 earned，不足以往堕落关卡的最高值时返回要补的差额，并更新最高值
    pub fn fallen_rune_floor(&mut self,id:usize,earned:u32)->u32 {
        if self.red_combat(id).is_none()&&Some(id)!=self.fallen_id() {return 0;}
        let Some(fallen)=self.fallen.as_mut() else{return 0};
        let top_up=fallen.best_runes.saturating_sub(earned);
        fallen.best_runes=fallen.best_runes.max(earned);
        top_up
    }
    fn red_combat(&self,id:usize)->Option<u32> {
        self.nodes.get(id).filter(|n|matches!(n.kind,NodeKind::Monster|NodeKind::Elite))?;self.descent_level(id)
    }
    /// 精英结算的奖励页（不带向下规则），红色关卡在它的基础上增减选项
    fn elite_page(&self,id:usize)->Vec<Reward> {
        let mut elite=self.clone();elite.nodes[id].kind=NodeKind::Elite;elite.wander=false;elite.offers(id)
    }
    /// 红色关卡奖励：精英奖励的装备、额外一件（含魔法与祷告）、护符效果、圣杯滴露，按向下次数取前 1–4 个
    fn descent_offers(&self,id:usize,level:u32)->Vec<Reward> {
        let base=self.elite_page(id);
        let mut offers=Vec::new();
        offers.extend(base.first().cloned());
        offers.push(descent::extra_offer(self,id,&base));
        offers.extend(base.iter().skip(1).cloned());
        offers.truncate(descent::offer_count(level).min(base.len()+1));
        offers
    }
    /// [HansFix] 红色战斗关卡的奖励里，这件装备的词条会怎样被扭曲
    pub fn curse(&self,node:usize,reward:&Reward)->Option<descent::Cursed> {
        let Reward::Item(item)=*reward else{return None};
        if Some(node)==self.fallen_id() {return descent::super_plan(self.reward_seed(),node,item);}
        self.red_combat(node)?;
        descent::plan(self.reward_seed(),node,item)
    }
    /// [HansFix] 堕落商店：向下进入的商店（堕落挑战里的商店都是）
    pub fn red_shop(&self,id:usize)->bool {self.nodes.get(id).is_some_and(|n|n.kind==NodeKind::Shop)&&self.descent_level(id).is_some()}
    /// 堕落商店里这件装备的堕落方式（与红色关卡奖励相同：一个词条 ×2、一个 ×-1）
    pub fn shop_curse(&self,node:usize,item:u32)->Option<descent::Cursed> {
        if !self.red_shop(node) {return None;}
        descent::plan(self.reward_seed(),node,item)
    }
    /// 在堕落商店买下的装备记入本局（之后按堕落后的效果生效）
    pub fn keep_shop_gear(&mut self,node:usize,item:u32)->RuleResult<()> {
        let curse=self.shop_curse(node,item).ok_or("这件装备不是堕落商店的商品")?;
        if !self.claimed_items.contains(&item) {self.claimed_items.push(item);}
        self.cursed.retain(|c|c.item!=item);self.cursed.push(curse);Ok(())
    }
    /// 在这家堕落商店消费满额：送一件“两个词条 ×2、一个 ×(-2)”的传说装备（每家一次）
    pub fn grant_shop_bonus(&mut self,node:usize)->RuleResult<u32> {
        if !self.red_shop(node) {return Err("只有堕落商店有满额奖励".into());}
        if self.fallen.as_ref().is_none_or(|f|f.bonus.contains(&node)) {return Err("这家商店的满额奖励已经领过".into());}
        let (item,curse)=descent::shop_bonus(self,node).ok_or("没有可送的传说装备")?;
        self.claimed_items.push(item);self.cursed.retain(|c|c.item!=item);self.cursed.push(curse);
        if let Some(f)=self.fallen.as_mut() {f.bonus.push(node);}
        Ok(item)
    }
    /// 已领取的扭曲装备
    pub fn cursed_item(&self,item:u32)->Option<&descent::Cursed> {self.cursed.iter().find(|c|c.item==item)}

    pub fn decline_reward(&mut self) -> RuleResult<()> {
        let Phase::Reward { id, .. } = self.phase else {
            return Err("当前没有待领取奖励".into());
        };
        if self.fallen_second_page(id) {return Ok(());}
        self.finish(id);
        Ok(())
    }
    /// 堕落首领的第一页奖励领完：换到第二页，而不是结束关卡
    fn fallen_second_page(&mut self,id:usize)->bool {
        if Some(id)!=self.fallen_id() {return false;}
        let page=self.fallen.as_ref().map_or(9,|f|f.page);
        // 第三页：新誓约（没有可选的就直接结束）
        if page>1||page==1&&self.fallen_oath_cards().is_empty() {return false;}
        if let Some(f)=self.fallen.as_mut() {f.page=page+1;}
        self.phase=Phase::Reward{id,offers:self.offers(id)};
        true
    }
    /// [HansFix] 堕落首领的第三页（新誓约）是否正在等待选择
    pub fn fallen_oath_pending(&self)->bool {
        matches!(self.phase,Phase::Reward{id,..} if Some(id)==self.fallen_id())&&self.fallen.as_ref().is_some_and(|f|f.page==2)
    }
    /// 第三页的卡：最多三份没立过的新誓约，已立的新誓约未满级时再加一张深化卡
    pub fn fallen_oath_cards(&self)->Vec<fallen_oaths::Card> {
        let seed=mix(self.reward_seed()^u64::from(self.act)^0x0a7f_a11e);
        let mut pool:Vec<_>=fallen_oaths::NEW.into_iter().filter(|k|!self.fallen_oaths.iter().any(|o|o.kind==*k)).collect();
        pool.sort_by_key(|k|mix(seed^k.index() as u64));
        let mut cards:Vec<_>=pool.into_iter().take(3).map(|k|fallen_oaths::Card::New(fallen_oaths::FallenOath::roll(k,seed^(k.index() as u64)<<16))).collect();
        if let Some((i,_))=self.fallen_oaths.iter().enumerate().filter(|(_,o)|o.level<3).max_by_key(|(i,o)|(o.level,*i)) {cards.push(fallen_oaths::Card::Deepen(i));}
        cards
    }
    /// 第三页选一张卡（新誓约或深化），然后结束堕落首领关卡
    pub fn choose_fallen_oath(&mut self,index:usize)->RuleResult<()> {
        if !self.fallen_oath_pending() {return Err("现在不是选择堕落誓约的时候".into());}
        let card=*self.fallen_oath_cards().get(index).ok_or("无效誓约选项")?;
        match card {
            fallen_oaths::Card::New(oath)=>self.fallen_oaths.push(oath),
            fallen_oaths::Card::Deepen(i)=>{let o=self.fallen_oaths.get_mut(i).ok_or("誓约不存在")?;o.level=(o.level+1).min(3);},
        }
        let Phase::Reward{id,..}=self.phase else {return Err("当前没有待领取奖励".into());};
        self.finish(id);Ok(())
    }

    pub fn finish_chest(&mut self) -> RuleResult<()> {
        let id = self.active_node(NodeKind::Chest)?;
        self.finish(id);
        Ok(())
    }

    pub fn open_chest(&mut self) -> RuleResult<()> {
        let id = self.active_node(NodeKind::Chest)?;
        self.phase = Phase::Reward {
            id,
            offers: self.offers(id),
        };
        Ok(())
    }

    pub fn shop_offer(&self, index: usize) -> RuleResult<(Reward, u32, bool)> {
        self.active_node(NodeKind::Shop)?;
        let stock = [
            (Reward::Blessing(BlessingKind::Vigor), 75),
            (Reward::UpgradeWeapon, 100),
            (Reward::Blessing(BlessingKind::Endurance), 75),
        ];
        let (reward, price) = stock.get(index).ok_or("无效商品")?;
        Ok((
            reward.clone(),
            *price,
            self.shop_purchased & (1 << index) != 0,
        ))
    }

    pub fn buy(&mut self, index: usize) -> RuleResult<()> {
        let (reward, price, purchased) = self.shop_offer(index)?;
        if purchased {
            return Err("商品已售出".into());
        }
        if self.gold < price {
            return Err("卢恩不足".into());
        }
        self.grant(reward)?;
        self.gold -= price;
        self.shop_purchased |= 1 << index;
        Ok(())
    }

    pub fn leave_shop(&mut self) -> RuleResult<()> {
        let id = self.active_node(NodeKind::Shop)?;
        self.finish(id);
        Ok(())
    }

    pub fn event_cost(&self) -> u32 {
        self.max_hp.saturating_mul(15).div_ceil(100)
    }

    pub fn resolve_event(&mut self, accept: bool) -> RuleResult<()> {
        let id = self.active_node(NodeKind::Event)?;
        if accept {
            let cost = self.event_cost();
            if self.hp <= cost || self.max_hp <= cost {
                return Err("生命不足，无法献祭".into());
            }
            self.grant(Reward::Blessing(BlessingKind::Bleed))?;
            self.max_hp -= cost;
            self.hp = (self.hp - cost).min(self.max_hp);
        }
        self.finish(id);
        Ok(())
    }

    pub fn encode(&self) -> RuleResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }
    pub fn resume_from_hub(&mut self) -> RuleResult<()> {
        self.validate()?;
        if matches!(self.phase,Phase::Dead|Phase::Won){return Err("已结束的旅程不能继续".into());}
        if let Phase::Combat { .. } = self.phase {
            self.checkpoint=None;
            self.phase=Phase::Map;
        }
        Ok(())
    }
    pub fn end_on_death(&mut self) {
        self.hp=0;self.phase=Phase::Dead;self.checkpoint=None;
    }

    pub fn decode(json: &str) -> RuleResult<Self> {
        if json.len() > 128 * 1024 {
            return Err("存档过大".into());
        }
        let mut run: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if run.ruby_spells {
            run.ruby_spells=false;
            if let Phase::Reward{id,..}=run.phase {run.phase=Phase::Reward{id,offers:run.offers(id)};}
            if run.shop_stock.as_ref().is_some_and(|s|s.items.iter().any(|[kind,id,..]|*kind==3&&rewards::RETIRED_SPELLS.contains(&(0x40000000|*id as u32)))) {run.shop_stock=None;}
        }
        if run.flask_bonus<=4&&run.flask_limit==5+run.flask_bonus*2 {
            if run.flasks>run.flask_limit||run.checkpoint.as_ref().is_some_and(|c|c.flasks>run.flask_limit) {return Err("旧存档圣杯瓶数量无效".into());}
            run.flask_limit=run.full_flask_limit();
        }
        run.validate()?;
        if run.boss_reward_rules==0 {
            run.boss_reward_rules=1;
            if let Phase::Reward{id,..}=run.phase {run.phase=Phase::Reward{id,offers:run.offers(id)};}
        }
        if let Phase::Reward{id,ref offers}=run.phase {
            if *offers==rewards::before_tool_rate(&run,id)||*offers==rewards::before_whetblades(&run,id)||*offers==rewards::before_claimed_bases(&run,id)||*offers==rewards::before_spirit_levels(&run,id)||*offers==rewards::before_currency_1500(&run,id)||*offers==rewards::before_chest_boosts(&run,id)||*offers==rewards::before_bosses(&run,id)||*offers==rewards::before_elites(&run,id)||*offers==rewards::before_base_talismans(&run,id)||*offers==rewards::before_spirit_weight(&run,id)||*offers==rewards::previous_offers(&run,id)||*offers==run.legacy_offers(id)||*offers==rewards::before_spirits(&run,id)||*offers==rewards::before_expansion(&run,id) {
                run.phase=Phase::Reward{id,offers:run.offers(id)};
            }
        }
        if let Some(checkpoint) = &run.checkpoint {
            run.hp = checkpoint.hp;
            run.flasks = checkpoint.flasks;
        }
        Ok(run)
    }

    pub fn validate(&self) -> RuleResult<()> {
        let fail = || Err("存档状态不合法或版本不受支持".into());
        if ![1,2,3,4,5,6].contains(&self.schema)||self.affix_rules>5||self.boss_reward_rules>1||self.class_rules!=1||self.boss_affix_rules>1||self.ember_rules>1||self.scaling_rules>1
            || self.contract.as_ref().is_some_and(|c|!c.valid(self.act)&&!(self.debug&&c.act==self.act))
            || !self.larval_gift.valid(self.act)
            || self.elite_buffs.iter().any(|id|!elite::BUFFS.contains(id))
            || self.elite_buffs.iter().collect::<std::collections::BTreeSet<_>>().len()!=self.elite_buffs.len()
            || !self.valid_regional_event()
            || !(1..=3).contains(&self.act)
            || self.nodes != match self.schema {1=>legacy_graph(self.seed),2=>graph(self.seed),3=>balanced_graph(self.seed),4=>expedition_graph(self.seed),5=>spaced_graph(self.seed),_=>self.expected_nodes()}
            || self.max_hp == 0
            || self.max_hp > 100
            || self.hp > self.max_hp
            || self.flask_bonus>4 || self.memory_bonus>6
            || !self.law.valid()
            || !self.valid_start_talisman()
            || self.event_rules>3
            || (!self.ember_rewards&&!self.embers.is_empty())
            || self.embers.iter().enumerate().any(|(i,e)|self.embers[..i].contains(e))
            || self.flask_limit != self.full_flask_limit()
            || self.flasks > self.flask_limit
            || self.weapon_level > 25
            || self.weapon_peak.is_some_and(|peak|peak>50)
            || self.spirit_peak.is_some_and(|peak|peak>10)
            || self.shop_purchased > 7
            || self.blessings.len() > 8
            || self.delivered_talismans.len() > 24
            || self.visited.len() > if self.wander {self.nodes.len()} else {self.floors()}
        {
            return fail();
        }
        for (i, b) in self.blessings.iter().enumerate() {
            if !(1..=3).contains(&b.level)
                || self.blessings[..i].iter().any(|other| other.kind == b.kind)
            {
                return fail();
            }
        }
        for (i,id) in self.delivered_talismans.iter().enumerate() {
            if *id==0 || *id>999999 || self.delivered_talismans[..i].contains(id) {return fail();}
        }
        if self.claimed_items.len()>rewards::CATALOG.len()+affixes::CATALOG.len() {return fail();}
        if self.cursed.len()>self.claimed_items.len()||self.cursed.iter().any(|c|!self.claimed_items.contains(&c.item)||c.swaps.is_empty()
            ||c.swaps.len()>3||c.swaps.iter().any(|&(from,to)|!descent::effects(c.item).contains(&from)||(to!=descent::doubled(from)&&to!=descent::negated(from)&&to!=descent::negated2(from)))) {return fail();}
        // 旧版（r4 及以前）的堕落存档没有 fallen，照旧继续，只是这局没有红篝火与堕落首领
        if (self.fallen.is_some()&&!(self.wander&&self.schema==6))||self.fallen.as_ref().is_some_and(|f|f.page>2) {return fail();}
        if self.fallen_oaths.iter().enumerate().any(|(i,o)|!o.valid()||self.fallen_oaths[..i].iter().any(|p|p.kind==o.kind)) {return fail();}
        for (i,id) in self.claimed_items.iter().enumerate() {
            if !(rewards::lookup(*id).is_some_and(|item|item.kind.allowed(self.class)||elite::LEGENDARY.contains(&affixes::base(*id)))||rewards::RETIRED_SPELLS.contains(id))||self.claimed_items[..i].contains(id) {return fail();}
        }
        let mut route = if self.fallen_active() {self.red_camp().into_iter().collect()} else {self.start_nodes()};
        for (i,id) in self.visited.iter().enumerate() {
            if !route.contains(id) {
                return fail();
            }
            route = if self.fallen_active() {self.fallen_route(&self.visited[..=i],*id)} else if self.wander {self.wander_route(&self.visited[..=i],*id)} else {self.nodes[*id].next.clone()};
        }
        let current_id = match &self.phase {
            Phase::Combat { id } => {
                if !self.nodes.get(*id).is_some_and(|n| n.kind.combat()) {
                    return fail();
                }
                let Some(c) = &self.checkpoint else {
                    return fail();
                };
                if c.id != *id || c.hp == 0 || c.hp > self.max_hp || c.flasks > self.flask_limit {
                    return fail();
                }
                Some(*id)
            }
            Phase::Node { id } => {
                if self.nodes.get(*id).is_none_or(|n| n.kind.combat()) {
                    return fail();
                }
                Some(*id)
            }
            Phase::Reward { id, offers } => {
                if !self
                    .nodes
                    .get(*id)
                    .is_some_and(|n| n.kind.combat() || n.kind == NodeKind::Chest)
                    || (*offers != self.offers(*id) && !(self.red_combat(*id).is_some()&&*offers==self.elite_page(*id)) && *offers != rewards::before_tool_rate(self,*id) && *offers != rewards::before_whetblades(self,*id) && *offers != rewards::before_claimed_bases(self,*id) && *offers != rewards::before_spirit_levels(self,*id) && *offers != rewards::before_currency_1500(self,*id) && *offers != rewards::before_chest_boosts(self,*id) && *offers != rewards::before_bosses(self,*id) && *offers != rewards::before_elites(self,*id) && *offers != rewards::before_base_talismans(self,*id) && *offers != self.legacy_offers(*id) && *offers != rewards::previous_offers(self,*id) && *offers != rewards::before_spirits(self,*id) && *offers != rewards::before_expansion(self,*id) && *offers != rewards::before_spirit_weight(self,*id))
                {
                    return fail();
                }
                Some(*id)
            }
            Phase::Won => {
                let ended=if self.wander {self.visited.last().is_some_and(|&id|self.nodes[id].kind==NodeKind::Boss)}
                    else {self.visited.len()==self.floors()&&route.is_empty()};
                if !ended || self.hp == 0 {
                    return fail();
                }
                None
            }
            Phase::Dead => {
                if self.hp != 0 {
                    return fail();
                }
                None
            }
            Phase::Map => {
                if route.is_empty() {
                    return fail();
                }
                None
            }
        };
        if current_id.is_some_and(|id| !route.contains(&id))
            || (!matches!(self.phase, Phase::Combat { .. }) && self.checkpoint.is_some())
            || (self.hp == 0 && self.phase != Phase::Dead)
            || (self.shop_purchased != 0
                && !matches!(self.phase, Phase::Node { id } if self.nodes[id].kind == NodeKind::Shop))
            || self.shop_stock.as_ref().is_some_and(|stock|stock.items.len()>64
                || !matches!(self.phase, Phase::Node { id } if id==stock.node&&self.nodes[id].kind == NodeKind::Shop))
        {
            return fail();
        }
        Ok(())
    }
}
