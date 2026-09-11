use super::specificity::Specificity;

/// Whether a declaration was marked `!important`. Important declarations
/// always outrank normal ones, and layer order reverses for them, per the
/// CSS cascade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Importance {
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
pub struct Layer(u8);

impl Layer {
    pub const BASE: Self = Self(0);
    pub const MODULE: Self = Self(1);

    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// Total cascade priority of one matched declaration set. `Ord` gives
/// ascending application order: sort matches by this and fold them in order
/// so the last one applied — the highest priority — wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RulePriority {
    important: bool,
    layer_rank: u8,
    specificity: Specificity,
    sheet_index: u32,
    source_order: u32,
}

impl RulePriority {
    #[must_use]
    pub const fn new(
        importance: Importance,
        layer: Layer,
        specificity: Specificity,
        sheet_index: u32,
        source_order: u32,
    ) -> Self {
        let important = matches!(importance, Importance::Important);
        // Important declarations reverse layer order: the earliest-declared
        // layer wins, so its rank must sort highest.
        let layer_rank = if important {
            u8::MAX.saturating_sub(layer.value())
        } else {
            layer.value()
        };
        Self {
            important,
            layer_rank,
            specificity,
            sheet_index,
            source_order,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(n: u16) -> Specificity {
        Specificity::new(0, n, 0)
    }

    #[test]
    fn test_important_beats_normal_regardless_of_specificity() {
        let normal = RulePriority::new(Importance::Normal, Layer::MODULE, spec(10), 0, 0);
        let important = RulePriority::new(Importance::Important, Layer::BASE, spec(0), 0, 0);
        assert!(important > normal);
    }

    #[test]
    fn test_module_layer_beats_base_layer_when_normal() {
        let base = RulePriority::new(Importance::Normal, Layer::BASE, spec(5), 0, 0);
        let module = RulePriority::new(Importance::Normal, Layer::MODULE, spec(0), 0, 0);
        assert!(module > base);
    }

    #[test]
    fn test_base_layer_beats_module_layer_when_important() {
        let base = RulePriority::new(Importance::Important, Layer::BASE, spec(0), 0, 0);
        let module = RulePriority::new(Importance::Important, Layer::MODULE, spec(5), 0, 0);
        assert!(base > module);
    }

    #[test]
    fn test_specificity_breaks_ties_within_a_layer() {
        let low = RulePriority::new(Importance::Normal, Layer::MODULE, spec(1), 0, 5);
        let high = RulePriority::new(Importance::Normal, Layer::MODULE, spec(2), 0, 0);
        assert!(high > low);
    }

    #[test]
    fn test_source_order_breaks_ties_within_specificity() {
        let earlier = RulePriority::new(Importance::Normal, Layer::MODULE, spec(1), 0, 0);
        let later = RulePriority::new(Importance::Normal, Layer::MODULE, spec(1), 0, 1);
        assert!(later > earlier);
    }
}
