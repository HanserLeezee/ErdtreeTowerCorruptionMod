use eldenring::cs::{ItemCategory,PlayerGameData};
pub const TEAR:i32=8185;
/// 泪滴幼体的持有数量（背包 + 仓库）。它是重要物品，必须用 loot::held 读：items() 在无缝联机下读不到它，
/// 换幕发放时会以为没发到、每 2 秒再发一个
pub fn count(data:&PlayerGameData)->u64 {
    super::loot::held(data,|item|item.category()==ItemCategory::Goods&&item.param_id()==TEAR as u32)
}
