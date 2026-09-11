use crate::features::styling::domain::{Combinator, CompiledSelector, PseudoClass, SelectorStep};
use lightningcss::selector::{
    Combinator as LightningCombinator, Component, PseudoClass as LightningPseudoClass, Selector,
};
use parcel_selectors::parser::NthType;

const fn convert_combinator(comb: LightningCombinator) -> Option<Combinator> {
    match comb {
        LightningCombinator::Child => Some(Combinator::Child),
        LightningCombinator::Descendant => Some(Combinator::Descendant),
        LightningCombinator::NextSibling => Some(Combinator::NextSibling),
        LightningCombinator::LaterSibling => Some(Combinator::LaterSibling),
        _ => None,
    }
}

pub fn compile_nth_data(
    nth_data: &parcel_selectors::parser::NthSelectorData,
    step: &mut SelectorStep,
) {
    match nth_data.ty {
        NthType::Child => {
            if nth_data.is_function() {
                step.pseudo_classes.push(PseudoClass::NthChild {
                    a: nth_data.a,
                    b: nth_data.b,
                });
            } else {
                step.pseudo_classes.push(PseudoClass::FirstChild);
            }
        }
        NthType::LastChild => {
            if nth_data.is_function() {
                step.pseudo_classes.push(PseudoClass::NthLastChild {
                    a: nth_data.a,
                    b: nth_data.b,
                });
            } else {
                step.pseudo_classes.push(PseudoClass::LastChild);
            }
        }
        NthType::OnlyChild => {
            step.pseudo_classes.push(PseudoClass::OnlyChild);
        }
        _ => {}
    }
}

pub fn compile_pseudo_class(pseudo: &LightningPseudoClass, step: &mut SelectorStep) {
    match pseudo {
        LightningPseudoClass::Hover => step.pseudo_classes.push(PseudoClass::Hover),
        LightningPseudoClass::Active => step.pseudo_classes.push(PseudoClass::Active),
        LightningPseudoClass::Focus | LightningPseudoClass::FocusVisible => {
            step.pseudo_classes.push(PseudoClass::Focused);
        }
        LightningPseudoClass::Custom { name } if name.as_ref() == "focused" => {
            step.pseudo_classes.push(PseudoClass::Focused);
        }
        _ => {}
    }
}

#[must_use]
pub fn compile_selector(selector: &Selector) -> CompiledSelector {
    let mut steps = Vec::new();
    let mut current_step = SelectorStep::default();

    for component in selector.iter_raw_match_order() {
        match component {
            Component::LocalName(local_name) => {
                current_step.tag = Some(local_name.name.as_ref().to_lowercase());
            }
            Component::ID(id) => {
                current_step.id = Some(id.as_ref().to_string());
            }
            Component::Class(class) => {
                current_step.classes.push(class.as_ref().to_string());
            }
            Component::Nth(nth_data) => compile_nth_data(nth_data, &mut current_step),
            Component::NthOf(nth_of_data) => {
                compile_nth_data(nth_of_data.nth_data(), &mut current_step);
            }
            Component::Empty => {
                current_step.pseudo_classes.push(PseudoClass::Empty);
            }
            Component::Negation(selectors) => {
                for inner_sel in selectors.as_ref() {
                    current_step.negations.push(compile_selector(inner_sel));
                }
            }
            Component::NonTSPseudoClass(pseudo) => {
                compile_pseudo_class(pseudo, &mut current_step);
            }
            Component::Combinator(comb) => {
                current_step.combinator = convert_combinator(*comb);
                steps.push(current_step);
                current_step = SelectorStep::default();
            }
            _ => {}
        }
    }
    steps.push(current_step);

    CompiledSelector { steps }
}
