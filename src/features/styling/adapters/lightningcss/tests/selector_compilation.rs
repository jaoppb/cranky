//! Pins how `compile_selector` turns each lightningcss selector component into compiled steps.
use crate::features::styling::adapters::lightningcss::selector::compile_selector;
use crate::features::styling::domain::{Combinator, CompiledSelector, PseudoClass, SelectorStep};
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};

/// The debug form of every selector compiled from the selector list `selectors`.
fn compiled(selectors: &str) -> Vec<String> {
    let css = format!("{selectors} {{ color: red; }}");
    let sheet = StyleSheet::parse(&css, ParserOptions::default()).unwrap();
    let Some(CssRule::Style(rule)) = sheet.rules.0.first() else {
        panic!("{selectors} did not parse into a style rule");
    };
    rule.selectors
        .0
        .iter()
        .map(|selector| format!("{:?}", compile_selector(selector)))
        .collect()
}

/// The debug form of a compiled selector made of `steps`, rightmost step first.
fn expected(steps: Vec<SelectorStep>) -> String {
    format!("{:?}", CompiledSelector { steps })
}

/// A step that only requires `pseudo_classes`.
fn pseudo_step(pseudo_classes: Vec<PseudoClass>) -> SelectorStep {
    SelectorStep {
        pseudo_classes,
        ..SelectorStep::default()
    }
}

/// A step that only requires the tag `tag`, joined to the next step by `combinator`.
fn tag_step(tag: &str, combinator: Option<Combinator>) -> SelectorStep {
    SelectorStep {
        tag: Some(tag.to_owned()),
        combinator,
        ..SelectorStep::default()
    }
}

#[test]
fn tag_id_and_classes_fill_one_step_with_the_tag_lowercased() {
    let step = SelectorStep {
        tag: Some("flex".to_owned()),
        id: Some("main".to_owned()),
        classes: vec!["item".to_owned(), "big".to_owned()],
        ..SelectorStep::default()
    };
    assert_eq!(compiled("FLEX#main.item.big"), [expected(vec![step])]);
}

#[test]
fn each_combinator_closes_the_step_on_its_right() {
    for (selector, combinator) in [
        ("a > b", Combinator::Child),
        ("a b", Combinator::Descendant),
        ("a + b", Combinator::NextSibling),
        ("a ~ b", Combinator::LaterSibling),
    ] {
        let steps = vec![tag_step("b", Some(combinator)), tag_step("a", None)];
        assert_eq!(compiled(selector), [expected(steps)], "{selector}");
    }
}

#[test]
fn a_combinator_starts_a_fresh_step() {
    let right = SelectorStep {
        classes: vec!["item".to_owned()],
        pseudo_classes: vec![PseudoClass::Hover],
        combinator: Some(Combinator::Child),
        ..SelectorStep::default()
    };
    let left = SelectorStep {
        id: Some("bar".to_owned()),
        ..SelectorStep::default()
    };
    assert_eq!(
        compiled("#bar > .item:hover"),
        [expected(vec![right, left])]
    );
}

#[test]
fn a_pseudo_element_closes_the_step_without_a_combinator() {
    let steps = vec![SelectorStep::default(), tag_step("flex", None)];
    assert_eq!(compiled("flex::before"), [expected(steps)]);
}

/// Asserts that each selector of `cases` compiles to one step requiring only its pseudo-class.
fn assert_pseudo_classes(cases: &[(&str, PseudoClass)]) {
    for (selector, pseudo) in cases {
        let step = pseudo_step(vec![*pseudo]);
        assert_eq!(compiled(selector), [expected(vec![step])], "{selector}");
    }
}

#[test]
fn nth_child_functions_keep_their_step_size_and_offset() {
    let nth_child = |step_size, offset| PseudoClass::NthChild { step_size, offset };
    let nth_last_child = |step_size, offset| PseudoClass::NthLastChild { step_size, offset };
    assert_pseudo_classes(&[
        (":nth-child(2n+1)", nth_child(2, 1)),
        (":nth-last-child(3n-2)", nth_last_child(3, -2)),
        (":nth-child(2 of .item)", nth_child(0, 2)),
    ]);
}

#[test]
fn keyword_structural_selectors_compile_to_their_pseudo_classes() {
    assert_pseudo_classes(&[
        (":first-child", PseudoClass::FirstChild),
        (":last-child", PseudoClass::LastChild),
        (":only-child", PseudoClass::OnlyChild),
        (":empty", PseudoClass::Empty),
    ]);
}

#[test]
fn type_based_nth_selectors_are_ignored() {
    for selector in [":nth-of-type(2)", ":first-of-type", ":last-of-type"] {
        let step = pseudo_step(Vec::new());
        assert_eq!(compiled(selector), [expected(vec![step])], "{selector}");
    }
}

#[test]
fn interactive_pseudo_classes_compile_in_order_and_focus_aliases_to_focused() {
    let step = pseudo_step(vec![
        PseudoClass::Hover,
        PseudoClass::Active,
        PseudoClass::Focused,
        PseudoClass::Focused,
        PseudoClass::Focused,
    ]);
    assert_eq!(
        compiled(":hover:active:focus:focus-visible:focused"),
        [expected(vec![step])]
    );
}

#[test]
fn unsupported_components_are_ignored() {
    for selector in [":disabled", ":unknown-thing", "*", "[href]"] {
        let step = pseudo_step(Vec::new());
        assert_eq!(compiled(selector), [expected(vec![step])], "{selector}");
    }
}

#[test]
fn negation_compiles_every_inner_selector() {
    let first = SelectorStep {
        classes: vec!["first".to_owned()],
        ..SelectorStep::default()
    };
    let second = SelectorStep {
        tag: Some("text".to_owned()),
        id: Some("second".to_owned()),
        ..SelectorStep::default()
    };
    let step = SelectorStep {
        negations: vec![
            CompiledSelector { steps: vec![first] },
            CompiledSelector {
                steps: vec![second],
            },
        ],
        ..SelectorStep::default()
    };
    assert_eq!(
        compiled(":not(.first, text#second)"),
        [expected(vec![step])]
    );
}

#[test]
fn each_selector_of_a_list_compiles_on_its_own() {
    assert_eq!(
        compiled("a, b"),
        [
            expected(vec![tag_step("a", None)]),
            expected(vec![tag_step("b", None)])
        ]
    );
}
