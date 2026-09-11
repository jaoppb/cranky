use super::priority::RulePriority;
use crate::features::styling::domain::computed_style::ComputedStyle;

/// One declaration set that matched an element, tagged with its full cascade
/// priority. `fold_matches` sorts these into cascade order and applies them.
#[derive(Debug, Clone)]
pub struct MatchedRule {
    priority: RulePriority,
    style: ComputedStyle,
}

impl MatchedRule {
    #[must_use]
    pub const fn new(priority: RulePriority, style: ComputedStyle) -> Self {
        Self { priority, style }
    }
}

/// Applies the CSS cascade.
///
/// Sorts matches into ascending priority and folds them via
/// `ComputedStyle::merge_with`, so the highest-priority match — by
/// importance, then layer, then specificity, then source order — wins.
#[must_use]
pub fn fold_matches(mut matches: Vec<MatchedRule>) -> ComputedStyle {
    matches.sort_by_key(|m| m.priority);
    let mut result = ComputedStyle::default();
    for m in &matches {
        result.merge_with(&m.style);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::cascade::priority::{Importance, Layer};
    use crate::features::styling::domain::cascade::specificity::Specificity;
    use crate::shared::primitives::color::DrawingColor;

    fn style_with_color(hex: &str) -> ComputedStyle {
        let mut style = ComputedStyle::default();
        style.set_color(DrawingColor::parse(hex).unwrap());
        style
    }

    #[test]
    fn test_higher_priority_wins_regardless_of_vec_order() {
        let low = MatchedRule::new(
            RulePriority::new(
                Importance::Normal,
                Layer::BASE,
                Specificity::new(0, 1, 0),
                0,
                0,
            ),
            style_with_color("#111111"),
        );
        let high = MatchedRule::new(
            RulePriority::new(
                Importance::Normal,
                Layer::MODULE,
                Specificity::new(0, 0, 0),
                0,
                0,
            ),
            style_with_color("#eeeeee"),
        );
        // low pushed after high: fold must still resolve by priority, not vec order.
        let result = fold_matches(vec![high.clone(), low.clone()]);
        let expected = fold_matches(vec![low, high]);
        assert_eq!(result, expected);
        assert_eq!(result.color(), Some(&DrawingColor::parse("#eeeeee").unwrap()));
    }
}
