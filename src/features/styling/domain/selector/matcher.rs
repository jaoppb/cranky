//! Matches a compiled selector against an element query, walking combinators from right to
//! left.
use super::super::grid_types::PseudoClass;
use super::super::query::ElementQuery;
use super::model::{Combinator, CompiledSelector, SelectorStep};

/// Whether `query` matches `selector`: its first step against the element itself, then each
/// further step against an ancestor, by the combinator the step before it carries (descendant
/// when it carries none); sibling combinators never match.
#[must_use]
pub(crate) fn matches_selector(selector: &CompiledSelector, query: &ElementQuery<'_>) -> bool {
    let Some((first_step, ancestor_steps)) = selector.steps.split_first() else {
        return false;
    };
    if !step_matches(first_step, query) {
        return false;
    }
    let mut current_query = query.parent();
    for (previous_step, step) in selector.steps.iter().zip(ancestor_steps) {
        let combinator = previous_step.combinator.unwrap_or(Combinator::Descendant);
        match match_ancestor_step(combinator, step, current_query) {
            AncestorWalk::Matched(next_query) => current_query = next_query,
            AncestorWalk::Failed => return false,
        }
    }
    true
}

/// Where the walk up the ancestors stands after one more selector step.
enum AncestorWalk<'query> {
    /// The step matched an ancestor; the walk goes on from that ancestor's parent.
    Matched(Option<&'query ElementQuery<'query>>),
    /// No ancestor the combinator allows matched the step.
    Failed,
}

/// Matches `step` against the ancestors `combinator` allows, starting at `from`: only `from`
/// itself for a child combinator, the nearest matching one of `from` and its ancestors for a
/// descendant combinator, none for a sibling combinator.
fn match_ancestor_step<'query>(
    combinator: Combinator,
    step: &SelectorStep,
    from: Option<&'query ElementQuery<'query>>,
) -> AncestorWalk<'query> {
    match combinator {
        Combinator::Child => match from {
            Some(parent) if step_matches(step, parent) => AncestorWalk::Matched(parent.parent()),
            _ => AncestorWalk::Failed,
        },
        Combinator::Descendant => std::iter::successors(from, |ancestor| ancestor.parent())
            .find(|ancestor| step_matches(step, ancestor))
            .map_or(AncestorWalk::Failed, |ancestor| {
                AncestorWalk::Matched(ancestor.parent())
            }),
        Combinator::NextSibling | Combinator::LaterSibling => AncestorWalk::Failed,
    }
}

/// Whether `query` matches one compound step: its tag, id, every class, every pseudo-class,
/// and none of its negations, checked in that order.
#[must_use]
pub(crate) fn step_matches(step: &SelectorStep, query: &ElementQuery<'_>) -> bool {
    tag_matches(step, query)
        && id_matches(step, query)
        && classes_match(step, query)
        && step
            .pseudo_classes
            .iter()
            .all(|pseudo| pseudo_matches(pseudo, query))
        && !step
            .negations
            .iter()
            .any(|not_sel| matches_selector(not_sel, query))
}

/// Whether the step names no tag, the universal `*`, or the element's tag in lower case.
fn tag_matches(step: &SelectorStep, query: &ElementQuery<'_>) -> bool {
    step.tag
        .as_ref()
        .is_none_or(|tag| tag == "*" || tag == &query.tag().to_lowercase())
}

/// Whether the step names no id, or the element's own.
fn id_matches(step: &SelectorStep, query: &ElementQuery<'_>) -> bool {
    step.id
        .as_ref()
        .is_none_or(|id| query.id().is_some_and(|query_id| query_id.as_str() == id))
}

/// Whether the element carries every class the step names.
fn classes_match(step: &SelectorStep, query: &ElementQuery<'_>) -> bool {
    step.classes.iter().all(|class| {
        query
            .classes()
            .iter()
            .any(|query_class| query_class.as_str() == class)
    })
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
