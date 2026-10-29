// 同品质的重复词条叠加：词条是装备的常驻效果（9850000 + 种类×10 + 品质），游戏对同一编号只保留一份。
// 数一数左右手当前武器（双手持握时收在背上的那把不算）与 4 件防具上同一个词条效果出现几次，多出来的每一件挂一份
// 数值相同、编号不同的复制效果（scene-builder AffixStackAssets：9880000 + 份数×1000 + 偏移），件数变少时撤下。
use eldenring::cs::{ChrAsmArmStyle,ChrIns,ChrInsExt,EquipParamProtector,EquipParamWeapon,PlayerGameData,SoloParamRepository,SpEffectParam};
use fromsoftware_shared::FromStatic;

const AFFIX_FIRST:i32=9850000;
const AFFIX_SPAN:i32=210;
const FIRST:i32=9880000;
const SPAN:i32=1000;
const COPIES:usize=5;
fn affix(id:i32)->bool {(AFFIX_FIRST..AFFIX_FIRST+AFFIX_SPAN).contains(&id)}
fn ours(id:i32)->bool {(FIRST+SPAN..FIRST+SPAN*(COPIES as i32+1)).contains(&id)}
fn copy(effect:i32,n:usize)->i32 {FIRST+n as i32*SPAN+effect-AFFIX_FIRST}
/// 身上的词条效果（含重复）→ 需要挂的复制效果
fn wanted(effects:&[i32])->Vec<i32> {
    let mut counts=std::collections::BTreeMap::new();
    for &effect in effects.iter().filter(|e|affix(**e)) {*counts.entry(effect).or_insert(0usize)+=1;}
    counts.into_iter().flat_map(|(effect,count)|(1..count.min(COPIES+1)).map(move |n|copy(effect,n))).collect()
}
/// 左右手当前武器与 4 件防具的常驻效果
fn equipped(data:&PlayerGameData,params:&SoloParamRepository)->Vec<i32> {
    let asm=&data.equipment.chr_asm;
    let slots=[asm.equipment.active_right_weapon_slot() as usize,asm.equipment.active_left_weapon_slot() as usize];
    let held=[asm.equipment.arm_style!=ChrAsmArmStyle::LeftBothHands,asm.equipment.arm_style!=ChrAsmArmStyle::RightBothHands];
    let mut effects=Vec::new();
    for hand in 0..2 {
        let Some(id)=asm.equipment_param_ids.get(slots[hand]).copied().filter(|id|*id>0&&held[hand]) else {continue};
        let Some(weapon)=params.get::<EquipParamWeapon>(id as u32).or_else(||params.get::<EquipParamWeapon>(id as u32/100*100)) else {continue};
        effects.extend([weapon.resident_sp_effect_id(),weapon.resident_sp_effect_id1(),weapon.resident_sp_effect_id2()]);
    }
    for slot in 12..=15 {
        let Some(id)=asm.equipment_param_ids.get(slot).copied().filter(|id|*id>0) else {continue};
        let Some(armor)=params.get::<EquipParamProtector>(id as u32) else {continue};
        effects.extend([armor.resident_sp_effect_id(),armor.resident_sp_effect_id2(),armor.resident_sp_effect_id3()]);
    }
    effects
}
/// 每帧：按身上的重复词条挂上或撤下复制效果（参数缺失时不挂）
pub fn sync(chr:&mut ChrIns,data:&PlayerGameData) {
    if chr.modules.data.hp<=0||chr.chr_flags1c5.death_flag() {return;}
    let Ok(params)=(unsafe {SoloParamRepository::instance()}) else{return};
    let want=wanted(&equipped(data,params));
    let present:Vec<i32>=chr.special_effect.entries().map(|e|e.param_id).filter(|id|ours(*id)).collect();
    for &id in present.iter().filter(|id|!want.contains(id)) {chr.remove_speffect(id);}
    for &id in want.iter().filter(|id|!present.contains(id)) {
        if params.get::<SpEffectParam>(id as u32).is_some() {chr.apply_speffect(id,false);}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn each_extra_piece_with_the_same_affix_adds_one_copy() {
        let attack_rare=9850000+4*10+1;let hp_common=9850000;
        assert!(super::wanted(&[attack_rare,hp_common,-1,0]).is_empty(),"one of each: nothing extra");
        assert_eq!(super::wanted(&[attack_rare,attack_rare]),vec![9881041]);
        assert_eq!(super::wanted(&[attack_rare;3]),vec![9881041,9882041]);
        assert_eq!(super::wanted(&[attack_rare,attack_rare+1]),Vec::<i32>::new(),"different qualities already stack natively");
        assert_eq!(super::wanted(&[hp_common;9]).len(),super::COPIES,"capped at the generated copies");
        assert!(super::wanted(&[7000,7000,12345,12345]).is_empty(),"only tower affix effects");
        assert!(super::ours(9881041)&&super::ours(9885209)&&!super::ours(9880041)&&!super::ours(9886000));
    }
}
