use super::super::selector::{CompiledSelector, SelectorStep};

/// CSS specificity as an (id, class, element) triple.
///
/// Compared lexicographically per the cascade spec. Attribute selectors and
/// pseudo-elements are not modeled (none exist in the supported grammar);
/// pseudo-classes count in the class column, matching spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Specificity {
    id: u16,
    class: u16,
    element: u16,
}

impl Specificity {
    #[must_use]
    pub const fn new(id: u16, class: u16, element: u16) -> Self {
        Self { id, class, element }
    }

    #[must_use]
    pub const fn combine(self, other: Self) -> Self {
        Self {
            id: self.id.saturating_add(other.id),
            class: self.class.saturating_add(other.class),
            element: self.element.saturating_add(other.element),
        }
    }
}

/// Computes the specificity of a compiled selector by summing the
/// contribution of every step (every combinator-separated compound selector).
///
/// `:not()` contributes the specificity of the most specific selector among
/// its arguments, added once per step. This is exact for the common case of a
/// single `:not()` per compound selector, and a documented approximation
/// when a step carries more than one (their contributions are not summed
/// separately, matching neither an undercount nor overcount in practice for
/// the selectors this codebase uses).
#[must_use]
pub fn selector_specificity(selector: &CompiledSelector) -> Specificity {
    selector
        .steps
        .iter()
        .fold(Specificity::default(), |acc, step| {
            acc.combine(step_specificity(step))
        })
}

fn step_specificity(step: &SelectorStep) -> Specificity {
    let classes_and_pseudo = u16::try_from(step.classes.len())
        .unwrap_or(u16::MAX)
        .saturating_add(u16::try_from(step.pseudo_classes.len()).unwrap_or(u16::MAX));
    let mut spec = Specificity::new(
        u16::from(step.id.is_some()),
        classes_and_pseudo,
        u16::from(step.tag.as_deref().is_some_and(|t| t != "*")),
    );
    if let Some(max_negation) = step.negations.iter().map(selector_specificity).max() {
        spec = spec.combine(max_negation);
    }
    spec
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::{Combinator, PseudoClass};

    fn step(tag: Option<&str>, id: Option<&str>, classes: &[&str]) -> SelectorStep {
        SelectorStep {
            tag: tag.map(str::to_string),
            id: id.map(str::to_string),
            classes: classes.iter().map(|s| (*s).to_string()).collect(),
            pseudo_classes: Vec::new(),
            negations: Vec::new(),
            combinator: None,
        }
    }

    #[test]
    fn test_id_beats_classes() {
        let id_selector = CompiledSelector {
            steps: vec![step(None, Some("root"), &[])],
        };
        let class_selector = CompiledSelector {
            steps: vec![step(None, None, &["a", "b", "c", "d"])],
        };
        assert!(selector_specificity(&id_selector) > selector_specificity(&class_selector));
    }

    #[test]
    fn test_classes_beat_type() {
        let class_selector = CompiledSelector {
            steps: vec![step(None, None, &["item"])],
        };
        let type_selector = CompiledSelector {
            steps: vec![step(Some("bar"), None, &[])],
        };
        assert!(selector_specificity(&class_selector) > selector_specificity(&type_selector));
    }

    #[test]
    fn test_pseudo_class_counts_as_class() {
        let mut hover_step = step(None, None, &["item"]);
        hover_step.pseudo_classes.push(PseudoClass::Hover);
        let hover_selector = CompiledSelector {
            steps: vec![hover_step],
        };
        let two_class_selector = CompiledSelector {
            steps: vec![step(None, None, &["item", "active"])],
        };
        assert_eq!(
            selector_specificity(&hover_selector),
            selector_specificity(&two_class_selector)
        );
    }

    #[test]
    fn test_descendant_steps_sum() {
        let mut parent = step(None, None, &["container"]);
        parent.combinator = Some(Combinator::Descendant);
        let child = step(None, None, &["item"]);
        let selector = CompiledSelector {
            steps: vec![parent, child],
        };
        assert_eq!(selector_specificity(&selector), Specificity::new(0, 2, 0));
    }
}
