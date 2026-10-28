use std::{ffi::c_void,sync::atomic::{AtomicU64,AtomicUsize,Ordering}};
use eldenring::cs::{CharaInitParam,CalcCorrectGraph,SoloParamRepository};
use fromsoftware_shared::FromStatic;
use hudhook::mh::{MhHook,MH_EnableHook};

static ORIGINAL:AtomicUsize=AtomicUsize::new(0);
static CURVE:AtomicU64=AtomicU64::new(0);
type Cost=unsafe extern "C" fn(i32)->u64;

/// 阶梯式升级价格：按本局已经升了几级分档（不看角色的绝对等级，各职业初始等级不同）
/// (本局第几级起, 每级卢恩)
pub const STEPS:[(u32,u64);4]=[(1,1000),(11,2000),(21,3500),(31,5000)];
fn fixed_cost(next:i32,start:u32)->Option<u64> {
    if start==0||next<=start as i32 {return None;}
    let bought=(next-start as i32) as u32;
    STEPS.iter().rev().find(|(from,_)|bought>=*from).map(|(_,cost)|*cost)
}
unsafe extern "C" fn calculate(next:i32)->u64 {
    let curve=CURVE.load(Ordering::Acquire);
    let start=(curve>>32) as u32;
    if let Some(cost)=fixed_cost(next,start) {return cost;}
    let original:Cost=unsafe {std::mem::transmute(ORIGINAL.load(Ordering::Acquire))};
    unsafe {original(next)}
}
pub fn configure(archetype:u8)->Result<(),String> {
    let address=ORIGINAL.load(Ordering::Acquire);
    if address==0 {return Err("升级价格接口未加载，暂不开放加点".into());}
    let params=unsafe {SoloParamRepository::instance()}.map_err(|_|"升级参数未就绪")?;
    let start=params.get::<CharaInitParam>(3000+u32::from(archetype)).ok_or("职业升级参数不存在")?.soul_lv() as u32;
    if CURVE.load(Ordering::Acquire)>>32==u64::from(start) {return Ok(());}
    if params.get::<CalcCorrectGraph>(200).is_none(){return Err("升级费用曲线未加载".into());}
    let original:Cost=unsafe {std::mem::transmute(address)};
    let first=unsafe {original(start as i32+1)};
    if start==0||start>713||first==0||first>1_000_000 {return Err("初始升级价格无效".into());}
    CURVE.store((u64::from(start)<<32)|first,Ordering::Release);
    super::log(format!("Stepped level cost: class={archetype}, starting level={start}, steps={STEPS:?}"));
    Ok(())
}
pub unsafe fn install()->Result<(),String> {
    // GetLevelUpSoulCost (ESD env 105) and native leveling share this next-level cost function.
    let address=super::items::checked_function(0x687670,&[0x40,0x53,0x48,0x83,0xec,0x50,0x8b,0xd9,0x0f,0x29,0x74,0x24,0x40,0x48,0x8d,0x4c,0x24,0x20])? as *mut c_void;
    let hook=unsafe {MhHook::new(address,calculate as *mut c_void)}.map_err(|e|format!("Level cost hook: {e:?}"))?;
    ORIGINAL.store(hook.trampoline() as usize,Ordering::Release);
    if let Err(error)=unsafe {MH_EnableHook(address)}.ok() {
        ORIGINAL.store(0,Ordering::Release);
        return Err(format!("Level cost enable: {error:?}"));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_menu_single_and_multi_level_cost_follows_the_steps() {
        assert_eq!(fixed_cost(20,0),None);
        for start in [1,5,6,7,8,9,10] {
            assert_eq!(fixed_cost(start,start as u32),None);
            let cost=|bought:i32|fixed_cost(start+bought,start as u32).unwrap();
            assert_eq!([1,10,11,20,21,30,31,200].map(cost),[1000,1000,2000,2000,3500,3500,5000,5000]);
            for next in start+1..=713 {assert!(fixed_cost(next,start as u32).is_some());}
            // 一次升多级的合计：第 9～12 级跨过一个档位
            assert_eq!((9..=12).map(cost).sum::<u64>(),1000*2+2000*2);
            assert_eq!((1..=30).map(cost).sum::<u64>(),10_000+20_000+35_000);
        }
    }
}
