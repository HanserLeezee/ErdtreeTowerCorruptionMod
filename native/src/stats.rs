// 构筑面板的「数值加成」：角色身上正在生效的特效（词条装备与护符的常驻特效、誓约、余烬、律法、职业被动、临时增益）
// 按项连乘。项目表与显示格式在 erdtree_tower::stat_summary；这里只从 SpEffect 参数里读倍率。
// 带触发条件的（剩余生命高于/低于多少、只对某类攻击生效的 stateInfo）不算进常驻总数，只计个数。
use eldenring::{cs::{ChrIns,SoloParamRepository,SpEffectParam},param::SP_EFFECT_PARAM_ST};
use erdtree_tower::stat_summary::{Line,STATS,lines};
use fromsoftware_shared::FromStatic;

#[derive(Clone,Default)]
pub struct Summary {pub lines:Vec<Line>,pub conditional:usize}

pub fn collect(chr:&ChrIns)->Summary {
    let Ok(params)=(unsafe {SoloParamRepository::instance()}) else {return Summary::default();};
    let mut products=[1.0f32;STATS.len()];
    let mut seen=Vec::new();
    let mut conditional=0;
    for id in chr.special_effect.entries().map(|e|e.param_id) {
        if id<=0||seen.contains(&id) {continue;}
        seen.push(id);
        let Some(param)=params.get::<SpEffectParam>(id as u32) else {continue};
        // 0 表示免疫或“不使用”，负数与非有限值都不是正常倍率
        let rates=rates(param).map(|r|if r.is_finite()&&r>0.0 {r} else {1.0});
        if rates.iter().all(|r|(r-1.0).abs()<1e-6) {continue;}
        if param.condition_hp()>=0.0||param.condition_hp_rate()>=0.0||param.state_info()!=0 {conditional+=1;continue;}
        for (product,rate) in products.iter_mut().zip(rates) {*product*=rate;}
    }
    Summary{lines:lines(&products),conditional}
}

/// 与 STATS 的顺序一一对应；攻击力两组倍率都会乘到伤害上
fn rates(p:&SP_EFFECT_PARAM_ST)->[f32;STATS.len()] {[
    p.physics_attack_rate()*p.physics_attack_power_rate(),
    p.magic_attack_rate()*p.magic_attack_power_rate(),
    p.fire_attack_rate()*p.fire_attack_power_rate(),
    p.thunder_attack_rate()*p.thunder_attack_power_rate(),
    p.dark_attack_rate()*p.dark_attack_power_rate(),
    p.neutral_damage_cut_rate(),
    p.slash_damage_cut_rate(),
    p.blow_damage_cut_rate(),
    p.thrust_damage_cut_rate(),
    p.magic_damage_cut_rate(),
    p.fire_damage_cut_rate(),
    p.thunder_damage_cut_rate(),
    p.dark_damage_cut_rate(),
    p.toughness_damage_cut_rate(),
    p.max_hp_rate(),
    p.max_mp_rate(),
    p.max_stamina_rate(),
    p.consume_stamina_rate(),
    p.arts_consumption_rate(),
    p.magic_consumption_rate(),
    p.miracle_consumption_rate(),
    p.change_hp_estus_flask_correct_rate(),
    p.change_mp_estus_flask_correct_rate(),
    p.soul_rate(),
    p.item_drop_rate(),
]}
