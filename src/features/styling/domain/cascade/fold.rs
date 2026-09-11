use super::declared_style::DeclaredStyle;
use super::priority::RulePriority;

/// One declaration set that matched an element, tagged with its full cascade
/// priority. `fold_matches` sorts these into cascade order and applies them.
#[derive(Debug, Clone)]
pub struct MatchedRule {
    priority: RulePriority,
    style: DeclaredStyle,
}

impl MatchedRule {
    #[must_use]
    pub const fn new(priority: RulePriority, style: DeclaredStyle) -> Self {
        Self { priority, style }
    }
}

/// Applies the CSS cascade, short of inheritance.
///
/// Sorts matches into ascending priority and folds them via
/// `DeclaredStyle::merge_with`, so the highest-priority match — by
/// importance, then layer, then specificity, then source order — wins.
/// Inheritance is resolved separately, by `DeclaredStyle::collapse`, once
/// the caller has a value to inherit from.
#[must_use]
pub fn fold_matches(mut matches: Vec<MatchedRule>) -> DeclaredStyle {
    matches.sort_by_key(|m| m.priority);
    let mut result = DeclaredStyle::default();
    for m in &matches {
        result.merge_with(&m.style);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::cascade::declared_lengths::DeclaredLengths;
    use crate::features::styling::domain::cascade::declared_style::InheritableKeywords;
    use crate::features::styling::domain::cascade::inherited::InheritedStyle;
    use crate::features::styling::domain::cascade::priority::{Importance, Layer};
    use crate::features::styling::domain::cascade::specificity::Specificity;
    use crate::features::styling::domain::computed_style::ComputedStyle;
    use crate::shared::primitives::color::DrawingColor;

    fn style_with_color(hex: &str) -> DeclaredStyle {
        let mut style = ComputedStyle::default();
        style.set_color(DrawingColor::parse(hex).unwrap());
        DeclaredStyle::from_parts(
            style,
            InheritableKeywords::default(),
            None,
            DeclaredLengths::default(),
        )
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
        let result =
            fold_matches(vec![high.clone(), low.clone()]).collapse(&InheritedStyle::default());
        let expected = fold_matches(vec![low, high]).collapse(&InheritedStyle::default());
        assert_eq!(result, expected);
        assert_eq!(
            result.color(),
            Some(&DrawingColor::parse("#eeeeee").unwrap())
        );
    }
}
