//! Matches a compiled selector against an element query, walking combinators from right to
//! left.
use super::super::grid_types::PseudoClass;
use super::super::query::ElementQuery;
use super::model::{Combinator, CompiledSelector, SelectorStep};

#[must_use]
pub(crate) fn matches_selector(selector: &CompiledSelector, query: &ElementQuery<'_>) -> bool {
    let Some(first_step) = selector.steps.first() else {
        return false;
    };
    if !step_matches(first_step, query) {
        return false;
    }

    if selector.steps.len() == 1 {
        return true;
    }

    let mut current_query = query.parent();
    let mut step_idx = 1usize;

    while step_idx < selector.steps.len() {
        let prev_comb = selector
            .steps
            .get(step_idx.saturating_sub(1))
            .and_then(|previous_step| previous_step.combinator)
            .unwrap_or(Combinator::Descendant);
        let Some(step) = selector.steps.get(step_idx) else {
            return false;
        };

        match prev_comb {
            Combinator::Child => {
                let Some(parent) = current_query else {
                    return false;
                };
                if !step_matches(step, parent) {
                    return false;
                }
                current_query = parent.parent();
                step_idx = step_idx.saturating_add(1);
            }
            Combinator::Descendant => {
                let mut matched = false;
                while let Some(ancestor) = current_query {
                    if step_matches(step, ancestor) {
                        matched = true;
                        current_query = ancestor.parent();
                        step_idx = step_idx.saturating_add(1);
                        break;
                    }
                    current_query = ancestor.parent();
                }
                if !matched {
                    return false;
                }
            }
            Combinator::NextSibling | Combinator::LaterSibling => {
                return false;
            }
        }
    }

    true
}

#[must_use]
pub(crate) fn step_matches(step: &SelectorStep, query: &ElementQuery<'_>) -> bool {
    if let Some(tag) = &step.tag
        && tag != "*"
        && tag != &query.tag().to_lowercase()
    {
        return false;
    }

    if let Some(id) = &step.id {
        match query.id() {
            Some(qid) if qid.as_str() == id => {}
            _ => return false,
        }
    }

    for class in &step.classes {
        if !query
            .classes()
            .iter()
            .any(|query_class| query_class.as_str() == class)
        {
            return false;
        }
    }

    for pseudo in &step.pseudo_classes {
        if !pseudo_matches(pseudo, query) {
            return false;
        }
    }

    for not_sel in &step.negations {
        if matches_selector(not_sel, query) {
            return false;
        }
    }

    true
}

#[must_use]
pub(crate) fn pseudo_matches(pseudo: &PseudoClass, query: &ElementQuery<'_>) -> bool {
    match pseudo {
        PseudoClass::Hover | PseudoClass::Active | PseudoClass::Focused => {
            query.pseudo_classes().contains(pseudo)
        }
        PseudoClass::FirstChild => query.child_index() == 0,
        PseudoClass::LastChild => {
            query.total_children() > 0
                && query.child_index().saturating_add(1) == query.total_children()
        }
        PseudoClass::OnlyChild => query.total_children() == 1,
        PseudoClass::NthChild { step_size, offset } => {
            nth_matches(*step_size, *offset, query.child_index().saturating_add(1))
        }
        PseudoClass::NthLastChild { step_size, offset } => {
            let from_end = query.total_children().saturating_sub(query.child_index());
            nth_matches(*step_size, *offset, from_end)
        }
        PseudoClass::Empty => query.is_empty(),
    }
}

#[must_use]
pub(crate) fn nth_matches(step_size: i32, offset: i32, index_1based: usize) -> bool {
    let Ok(n_pos) = i32::try_from(index_1based) else {
        return false;
    };
    if step_size == 0 {
        return n_pos == offset;
    }
    let Some(diff) = n_pos.checked_sub(offset) else {
        return false;
    };
    if step_size > 0 {
        diff >= 0 && diff.checked_rem(step_size) == Some(0)
    } else {
        diff <= 0 && diff.checked_rem(step_size) == Some(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::{ClassName, ElementId, ElementIdentity};

    fn step(tag: &str, combinator: Option<Combinator>) -> SelectorStep {
        SelectorStep {
            tag: Some(tag.to_string()),
            combinator,
            ..SelectorStep::default()
        }
    }

    /// A selector from its steps, subject first, each carrying the combinator to the next.
    fn selector(steps: Vec<SelectorStep>) -> CompiledSelector {
        CompiledSelector { steps }
    }

    /// Runs `check` against the leaf of the tree `bar > flex#main.wide.dark > Text`.
    fn with_leaf(check: impl FnOnce(&ElementQuery<'_>)) {
        let id = ElementId::new("main").unwrap();
        let classes = [
            ClassName::new("wide").unwrap(),
            ClassName::new("dark").unwrap(),
        ];
        let root = ElementQuery::new(ElementIdentity::new("bar", None, &[]), &[], None);
        let middle = ElementQuery::new(
            ElementIdentity::new("flex", Some(&id), &classes),
            &[PseudoClass::Hover],
            Some(&root),
        );
        let leaf = ElementQuery::new(ElementIdentity::new("Text", None, &[]), &[], Some(&middle));
        check(&leaf);
    }

    fn leaf_matches(steps: Vec<SelectorStep>) -> bool {
        let compiled = selector(steps);
        let mut matched = false;
        with_leaf(|leaf| matched = matches_selector(&compiled, leaf));
        matched
    }

    /// Whether `step` matches the `flex#main.wide.dark:hover` element.
    fn middle_matches(step: &SelectorStep) -> bool {
        let mut matched = false;
        with_leaf(|leaf| matched = step_matches(step, leaf.parent().unwrap()));
        matched
    }

    #[test]
    fn test_empty_selector_matches_nothing() {
        assert!(!leaf_matches(vec![]));
    }

    #[test]
    fn test_subject_tag_is_compared_with_the_lowercased_element_tag() {
        assert!(leaf_matches(vec![step("text", None)]));
        assert!(leaf_matches(vec![step("*", None)]));
        assert!(!leaf_matches(vec![step("TEXT", None)]));
        assert!(!leaf_matches(vec![step("flex", None)]));
    }

    #[test]
    fn test_child_combinator_needs_the_direct_parent() {
        let child = Some(Combinator::Child);
        assert!(leaf_matches(vec![step("text", child), step("flex", None)]));
        assert!(!leaf_matches(vec![step("text", child), step("bar", None)]));
        assert!(leaf_matches(vec![
            step("text", child),
            step("flex", child),
            step("bar", None)
        ]));
        assert!(!leaf_matches(vec![
            step("text", child),
            step("flex", child),
            step("bar", child),
            step("*", None)
        ]));
    }

    #[test]
    fn test_descendant_combinator_finds_any_ancestor_once() {
        let descendant = Some(Combinator::Descendant);
        assert!(leaf_matches(vec![
            step("text", descendant),
            step("bar", None)
        ]));
        assert!(leaf_matches(vec![step("text", None), step("bar", None)]));
        assert!(!leaf_matches(vec![step("text", None), step("popup", None)]));
        assert!(leaf_matches(vec![
            step("text", None),
            step("flex", None),
            step("bar", None)
        ]));
        assert!(!leaf_matches(vec![
            step("text", None),
            step("bar", None),
            step("flex", None)
        ]));
        assert!(!leaf_matches(vec![
            step("text", None),
            step("flex", None),
            step("flex", None)
        ]));
    }

    #[test]
    fn test_descendant_then_child_walks_on_from_the_matched_ancestor() {
        let child = Some(Combinator::Child);
        assert!(leaf_matches(vec![
            step("text", None),
            step("flex", child),
            step("bar", None)
        ]));
        assert!(!leaf_matches(vec![
            step("text", None),
            step("bar", child),
            step("flex", None)
        ]));
    }

    #[test]
    fn test_sibling_combinators_never_match() {
        for combinator in [Combinator::NextSibling, Combinator::LaterSibling] {
            assert!(!leaf_matches(vec![
                step("text", Some(combinator)),
                step("flex", None)
            ]));
        }
    }

    /// Whether the `flex` step, edited by `edit`, matches the `flex#main.wide.dark:hover` element.
    fn edited_flex_matches(edit: fn(&mut SelectorStep)) -> bool {
        let mut probe = step("flex", None);
        edit(&mut probe);
        middle_matches(&probe)
    }

    #[test]
    fn test_step_checks_id_and_classes() {
        assert!(edited_flex_matches(|_| {}));
        assert!(edited_flex_matches(
            |probe| probe.id = Some("main".to_string())
        ));
        assert!(!edited_flex_matches(
            |probe| probe.id = Some("other".to_string())
        ));
        assert!(edited_flex_matches(
            |probe| probe.classes = vec!["dark".into(), "wide".into()]
        ));
        assert!(!edited_flex_matches(
            |probe| probe.classes = vec!["wide".into(), "light".into()]
        ));
    }

    #[test]
    fn test_step_checks_pseudo_classes_and_negations() {
        let not = |tag: &str| vec![selector(vec![step(tag, None)])];
        assert!(edited_flex_matches(
            |probe| probe.pseudo_classes = vec![PseudoClass::Hover]
        ));
        assert!(!edited_flex_matches(
            |probe| probe.pseudo_classes = vec![PseudoClass::Active]
        ));
        let mut probe = step("flex", None);
        probe.negations = not("bar");
        assert!(middle_matches(&probe));
        probe.negations = not("flex");
        assert!(!middle_matches(&probe));
    }

    #[test]
    fn test_id_step_rejects_an_element_without_an_id() {
        let mut probe = step("text", None);
        probe.id = Some("main".to_string());
        assert!(!leaf_matches(vec![probe]));
    }
}
