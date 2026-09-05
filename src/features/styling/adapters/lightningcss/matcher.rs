use super::selector::{CompiledSelector, SelectorStep};
use crate::features::styling::domain::{ElementQuery, PseudoClass};
use lightningcss::selector::Combinator;

#[must_use]
pub fn matches_selector(selector: &CompiledSelector, query: &ElementQuery) -> bool {
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
            .and_then(|s| s.combinator)
            .unwrap_or(Combinator::Descendant);
        let Some(step) = selector.steps.get(step_idx) else {
            return false;
        };

        match prev_comb {
            Combinator::Child => {
                let Some(q) = current_query else {
                    return false;
                };
                if !step_matches(step, q) {
                    return false;
                }
                current_query = q.parent();
                step_idx = step_idx.saturating_add(1);
            }
            Combinator::Descendant => {
                let mut matched = false;
                while let Some(q) = current_query {
                    if step_matches(step, q) {
                        matched = true;
                        current_query = q.parent();
                        step_idx = step_idx.saturating_add(1);
                        break;
                    }
                    current_query = q.parent();
                }
                if !matched {
                    return false;
                }
            }
            _ => {
                return false;
            }
        }
    }

    true
}

#[must_use]
pub fn step_matches(step: &SelectorStep, query: &ElementQuery) -> bool {
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
        if !query.classes().iter().any(|c| c.as_str() == class) {
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
pub fn pseudo_matches(pseudo: &PseudoClass, query: &ElementQuery) -> bool {
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
        PseudoClass::NthChild { a, b } => {
            nth_matches(*a, *b, query.child_index().saturating_add(1))
        }
        PseudoClass::NthLastChild { a, b } => {
            let from_end = query.total_children().saturating_sub(query.child_index());
            nth_matches(*a, *b, from_end)
        }
        PseudoClass::Empty => query.is_empty(),
    }
}

#[must_use]
pub fn nth_matches(a: i32, b: i32, index_1based: usize) -> bool {
    let Ok(n_pos) = i32::try_from(index_1based) else {
        return false;
    };
    if a == 0 {
        return n_pos == b;
    }
    let Some(diff) = n_pos.checked_sub(b) else {
        return false;
    };
    if a > 0 {
        diff >= 0 && diff.checked_rem(a) == Some(0)
    } else {
        diff <= 0 && diff.checked_rem(a) == Some(0)
    }
}
