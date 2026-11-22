//! Cascade priority of a matched rule: importance, layer, specificity, then source order.
use super::specificity::Specificity;

/// Whether a declaration was marked `!important`. Important declarations
/// always outrank normal ones, and layer order reverses for them, per the
/// CSS cascade. `Ord` follows that: `Normal` sorts before `Important`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Importance {
    Normal,
    Important,
}

/// A cascade layer, ordered by declaration: layer `0` is declared first and
/// loses to every later layer for normal declarations (and, symmetrically,
/// wins over them for `!important` ones).
///
/// Only two layers exist today — `BASE` (the shared `base.css`) and `MODULE`
/// (a module's own stylesheets) — assigned by `CompositeStyleResolver` based
/// on stylesheet identity, not by an in-file `@layer` at-rule: base and
/// module sheets are parsed independently, so there is no single document to
/// carry a real `@layer` statement across them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Layer(u8);

impl Layer {
    pub(crate) const BASE: Self = Self(0);
    pub(crate) const MODULE: Self = Self(1);

    #[must_use]
    pub(crate) const fn value(self) -> u8 {
        self.0
    }
}

/// Total cascade priority of one matched declaration set. `Ord` gives
/// ascending application order: sort matches by this and fold them in order
/// so the last one applied — the highest priority — wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RulePriority {
    importance: Importance,
    layer_rank: u8,
    specificity: Specificity,
    sheet_index: u32,
    source_order: u32,
}

/// Where a rule ranks inside its own sheet: whether it is `!important`, how specific its
/// selector is, and where it sits in the sheet's source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuleRank {
    importance: Importance,
    specificity: Specificity,
    source_order: u32,
}

impl RuleRank {
    /// A rule of `importance` matched through a selector of `specificity`, at `source_order`.
    #[must_use]
    pub(crate) const fn new(
        importance: Importance,
        specificity: Specificity,
        source_order: u32,
    ) -> Self {
        Self {
            importance,
            specificity,
            source_order,
        }
    }
}

/// Where a rule's sheet sits among the sheets composed into one cascade: its layer, and its
/// index among the composed sheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CascadeOrigin {
    layer: Layer,
    sheet_index: u32,
}

impl CascadeOrigin {
    /// A sheet in `layer`, composed at `sheet_index`.
    #[must_use]
    pub(crate) const fn new(layer: Layer, sheet_index: u32) -> Self {
        Self { layer, sheet_index }
    }
}

impl RulePriority {
    #[must_use]
    pub(crate) const fn new(rank: RuleRank, origin: CascadeOrigin) -> Self {
        // Important declarations reverse layer order: the earliest-declared
        // layer wins, so its rank must sort highest.
        let layer_rank = if matches!(rank.importance, Importance::Important) {
            u8::MAX.saturating_sub(origin.layer.value())
        } else {
            origin.layer.value()
        };
        Self {
            importance: rank.importance,
            layer_rank,
            specificity: rank.specificity,
            sheet_index: origin.sheet_index,
            source_order: rank.source_order,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(class_count: u16) -> Specificity {
        Specificity::new(0, class_count, 0)
    }

    #[test]
    fn test_important_beats_normal_regardless_of_specificity() {
        let normal = RulePriority::new(
            RuleRank::new(Importance::Normal, spec(10), 0),
            CascadeOrigin::new(Layer::MODULE, 0),
        );
        let important = RulePriority::new(
            RuleRank::new(Importance::Important, spec(0), 0),
            CascadeOrigin::new(Layer::BASE, 0),
        );
        assert!(important > normal);
    }

    #[test]
    fn test_module_layer_beats_base_layer_when_normal() {
        let base = RulePriority::new(
            RuleRank::new(Importance::Normal, spec(5), 0),
            CascadeOrigin::new(Layer::BASE, 0),
        );
        let module = RulePriority::new(
            RuleRank::new(Importance::Normal, spec(0), 0),
            CascadeOrigin::new(Layer::MODULE, 0),
        );
        assert!(module > base);
    }

    #[test]
    fn test_base_layer_beats_module_layer_when_important() {
        let base = RulePriority::new(
            RuleRank::new(Importance::Important, spec(0), 0),
            CascadeOrigin::new(Layer::BASE, 0),
        );
        let module = RulePriority::new(
            RuleRank::new(Importance::Important, spec(5), 0),
            CascadeOrigin::new(Layer::MODULE, 0),
        );
        assert!(base > module);
    }

    #[test]
    fn test_specificity_breaks_ties_within_a_layer() {
        let low = RulePriority::new(
            RuleRank::new(Importance::Normal, spec(1), 5),
            CascadeOrigin::new(Layer::MODULE, 0),
        );
        let high = RulePriority::new(
            RuleRank::new(Importance::Normal, spec(2), 0),
            CascadeOrigin::new(Layer::MODULE, 0),
        );
        assert!(high > low);
    }

    #[test]
    fn test_source_order_breaks_ties_within_specificity() {
        let earlier = RulePriority::new(
            RuleRank::new(Importance::Normal, spec(1), 0),
            CascadeOrigin::new(Layer::MODULE, 0),
        );
        let later = RulePriority::new(
            RuleRank::new(Importance::Normal, spec(1), 1),
            CascadeOrigin::new(Layer::MODULE, 0),
        );
        assert!(later > earlier);
    }

    #[test]
    fn test_later_sheet_beats_earlier_source_order_within_specificity() {
        let earlier_sheet = RulePriority::new(
            RuleRank::new(Importance::Normal, spec(1), 7),
            CascadeOrigin::new(Layer::MODULE, 0),
        );
        let later_sheet = RulePriority::new(
            RuleRank::new(Importance::Normal, spec(1), 0),
            CascadeOrigin::new(Layer::MODULE, 2),
        );
        assert!(later_sheet > earlier_sheet);
    }
}
