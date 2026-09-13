use std::collections::HashMap;

use super::super::query::ElementQuery;
use super::model::{CompiledSelector, RuleEntry, SelectorStep};

/// Indexes a stylesheet's rules by each selector's rightmost (subject)
/// simple selector, so `candidates` returns only the rules an element could
/// possibly match instead of every rule in the sheet.
///
/// Built once per parsed stylesheet, at parse time — there is no
/// invalidation to manage since a reparsed sheet just rebuilds a fresh
/// index alongside its fresh rule list.
#[derive(Debug, Default)]
pub struct RuleIndex {
    by_id: HashMap<String, Vec<usize>>,
    by_class: HashMap<String, Vec<usize>>,
    by_tag: HashMap<String, Vec<usize>>,
    /// Rules whose subject step has no id, class, or specific tag to key on
    /// (e.g. a bare `*` or `:hover`) — always a candidate, since nothing
    /// about the element can rule them out ahead of the full selector check.
    universal: Vec<usize>,
}

impl RuleIndex {
    #[must_use]
    pub fn build(rules: &[RuleEntry]) -> Self {
        let mut index = Self::default();
        for (rule_index, rule) in rules.iter().enumerate() {
            for selector in &rule.selectors {
                index.insert(selector, rule_index);
            }
        }
        index
    }

    fn insert(&mut self, selector: &CompiledSelector, rule_index: usize) {
        let Some(subject) = selector.steps.first() else {
            return;
        };
        match bucket_key(subject) {
            BucketKey::Id(id) => self.by_id.entry(id).or_default().push(rule_index),
            BucketKey::Class(class) => self.by_class.entry(class).or_default().push(rule_index),
            BucketKey::Tag(tag) => self.by_tag.entry(tag).or_default().push(rule_index),
            BucketKey::Universal => self.universal.push(rule_index),
        }
    }

    /// Every rule index that could possibly match `query`, given only its
    /// own id/classes/tag — a superset of the actual matches. The caller
    /// still runs the full `matches_selector` check (ancestors, pseudo
    /// classes, `:not()`) on each candidate; this just avoids paying for
    /// that check on rules that provably can't match.
    ///
    /// A selector is registered under exactly one of its own features (its
    /// id, else its first class, else its tag), so probing every feature
    /// the element actually has — not just one — is what guarantees a
    /// matching selector's bucket is always among the ones searched.
    #[must_use]
    pub fn candidates(&self, query: &ElementQuery) -> Vec<usize> {
        let mut found = self.universal.clone();

        if let Some(id) = query.id()
            && let Some(indices) = self.by_id.get(id.as_str())
        {
            found.extend(indices);
        }
        for class in query.classes() {
            if let Some(indices) = self.by_class.get(class.as_str()) {
                found.extend(indices);
            }
        }
        if let Some(indices) = self.by_tag.get(&query.tag().to_lowercase()) {
            found.extend(indices);
        }

        found.sort_unstable();
        found.dedup();
        found
    }
}

enum BucketKey {
    Id(String),
    Class(String),
    Tag(String),
    Universal,
}

/// The single most-selective feature on a subject step to index by.
fn bucket_key(step: &SelectorStep) -> BucketKey {
    if let Some(id) = &step.id {
        return BucketKey::Id(id.clone());
    }
    if let Some(class) = step.classes.first() {
        return BucketKey::Class(class.clone());
    }
    if let Some(tag) = &step.tag
        && tag != "*"
    {
        return BucketKey::Tag(tag.clone());
    }
    BucketKey::Universal
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::cascade::{DeclaredStyle, Importance};
    use crate::features::styling::domain::identifiers::ClassName;

    fn rule(selectors: Vec<CompiledSelector>) -> RuleEntry {
        RuleEntry {
            selectors,
            style: DeclaredStyle::default(),
            importance: Importance::Normal,
        }
    }

    fn class_step(class: &str) -> SelectorStep {
        SelectorStep {
            classes: vec![class.to_string()],
            ..Default::default()
        }
    }

    #[test]
    fn test_candidates_excludes_rules_with_no_shared_feature() {
        let rules = vec![
            rule(vec![CompiledSelector {
                steps: vec![class_step("foo")],
            }]),
            rule(vec![CompiledSelector {
                steps: vec![class_step("bar")],
            }]),
        ];
        let index = RuleIndex::build(&rules);

        let class_foo = ClassName::new("foo").unwrap();
        let classes = [class_foo];
        let query = ElementQuery::new("text", None, &classes, &[], None);

        assert_eq!(index.candidates(&query), vec![0]);
    }

    #[test]
    fn test_candidates_always_includes_universal_bucket() {
        let rules = vec![rule(vec![CompiledSelector {
            steps: vec![SelectorStep {
                pseudo_classes: vec![crate::features::styling::domain::PseudoClass::Hover],
                ..Default::default()
            }],
        }])];
        let index = RuleIndex::build(&rules);

        let query = ElementQuery::new("text", None, &[], &[], None);
        assert_eq!(index.candidates(&query), vec![0]);
    }

    #[test]
    fn test_candidates_dedupes_a_rule_matched_by_two_bucket_keys() {
        let rules = vec![rule(vec![
            CompiledSelector {
                steps: vec![class_step("foo")],
            },
            CompiledSelector {
                steps: vec![class_step("bar")],
            },
        ])];
        let index = RuleIndex::build(&rules);

        let class_foo = ClassName::new("foo").unwrap();
        let class_bar = ClassName::new("bar").unwrap();
        let classes = [class_foo, class_bar];
        let query = ElementQuery::new("text", None, &classes, &[], None);

        assert_eq!(index.candidates(&query), vec![0]);
    }
}
