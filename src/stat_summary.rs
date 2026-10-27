//! 构筑面板的「数值加成」：把角色身上正在生效的特效（词条装备、护符、誓约、余烬、律法、职业被动、临时增益……）
//! 按项连乘后的倍率，换成“物理攻击力 +12%”“受到火伤害 -8%”这样的文字。倍率由 DLL 从 SpEffect 参数里读出来
//! （native/src/stats.rs），这里只负责项目表、合并与格式，方便测试。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sense {
    /// 越高越好（攻击力、生命上限、卢恩获取……）
    Gain,
    /// 越低越好（受到的伤害、消耗、受到的削韧）
    Cost,
}

#[derive(Clone, Copy, Debug)]
pub struct Stat {
    pub group: &'static str,
    pub label: &'static str,
    pub sense: Sense,
}

const fn gain(group: &'static str, label: &'static str) -> Stat {
    Stat { group, label, sense: Sense::Gain }
}
const fn cost(group: &'static str, label: &'static str) -> Stat {
    Stat { group, label, sense: Sense::Cost }
}

/// 显示顺序；DLL 按同样的顺序给出每一项的总倍率
pub const STATS: [Stat; 25] = [
    gain("攻击", "物理攻击力"),
    gain("攻击", "魔力攻击力"),
    gain("攻击", "火攻击力"),
    gain("攻击", "雷攻击力"),
    gain("攻击", "圣攻击力"),
    cost("防御", "受到物理伤害"),
    cost("防御", "受到斩击伤害"),
    cost("防御", "受到打击伤害"),
    cost("防御", "受到突刺伤害"),
    cost("防御", "受到魔力伤害"),
    cost("防御", "受到火伤害"),
    cost("防御", "受到雷伤害"),
    cost("防御", "受到圣伤害"),
    cost("防御", "受到的削韧"),
    gain("属性", "生命上限"),
    gain("属性", "专注上限"),
    gain("属性", "精力上限"),
    cost("消耗", "精力消耗"),
    cost("消耗", "战技专注消耗"),
    cost("消耗", "魔法专注消耗"),
    cost("消耗", "祷告专注消耗"),
    gain("回复", "红露滴回复量"),
    gain("回复", "蓝露滴回复量"),
    gain("其他", "卢恩获取"),
    gain("其他", "物品掉落率"),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub group: &'static str,
    pub label: &'static str,
    /// “+12%”“-8.5%”
    pub value: String,
    /// 对玩家有利（金色）还是不利（红色）
    pub good: bool,
}

/// 每一项的总倍率（与 STATS 对齐）→ 有变化的项；不足 0.05% 的不显示
pub fn lines(products: &[f32]) -> Vec<Line> {
    STATS
        .iter()
        .zip(products)
        .filter_map(|(stat, product)| {
            let percent = (product - 1.0) * 100.0;
            if !percent.is_finite() || percent.abs() < 0.05 {
                return None;
            }
            let good = match stat.sense {
                Sense::Gain => percent > 0.0,
                Sense::Cost => percent < 0.0,
            };
            Some(Line { group: stat.group, label: stat.label, value: format_percent(percent), good })
        })
        .collect()
}

fn format_percent(percent: f32) -> String {
    let rounded = (percent * 10.0).round() / 10.0;
    let text = if rounded.fract() == 0.0 { format!("{:.0}", rounded.abs()) } else { format!("{:.1}", rounded.abs()) };
    format!("{}{}%", if rounded < 0.0 { "-" } else { "+" }, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neutral() -> Vec<f32> {
        vec![1.0; STATS.len()]
    }

    #[test]
    fn neutral_multipliers_show_nothing() {
        assert!(lines(&neutral()).is_empty());
        let mut tiny = neutral();
        tiny[0] = 1.0004;
        assert!(lines(&tiny).is_empty());
    }

    #[test]
    fn changes_are_signed_rounded_and_judged_by_direction() {
        let mut products = neutral();
        products[0] = 1.12 * 1.05; // 物理攻击力：两个加成连乘
        products[10] = 0.9; // 受到火伤害 -10%：有利
        products[17] = 1.15; // 精力消耗 +15%：不利
        products[23] = 1.0333; // 卢恩获取
        let out = lines(&products);
        assert_eq!(out.len(), 4);
        assert_eq!((out[0].label, out[0].value.as_str(), out[0].good), ("物理攻击力", "+17.6%", true));
        assert_eq!((out[1].label, out[1].value.as_str(), out[1].good), ("受到火伤害", "-10%", true));
        assert_eq!((out[2].label, out[2].value.as_str(), out[2].good), ("精力消耗", "+15%", false));
        assert_eq!(out[3].value, "+3.3%");
    }

    #[test]
    fn broken_values_are_ignored() {
        let mut products = neutral();
        products[1] = f32::NAN;
        products[2] = f32::INFINITY;
        assert!(lines(&products).is_empty());
    }
}
