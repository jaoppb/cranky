use super::super::grid_types::PseudoClass;
use super::super::query::ElementQuery;
use super::model::{Combinator, CompiledSelector, SelectorStep};

#[must_use]
pub fn matches_selector(selector: &CompiledSelector, query: &ElementQuery<'_>) -> bool {
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
pub fn step_matches(step: &SelectorStep, query: &ElementQuery<'_>) -> bool {
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
pub fn pseudo_matches(pseudo: &PseudoClass, query: &ElementQuery<'_>) -> bool {
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
pub fn nth_matches(step_size: i32, offset: i32, index_1based: usize) -> bool {
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
