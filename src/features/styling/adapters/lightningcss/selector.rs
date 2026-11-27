//! Compiles lightningcss selectors into the domain's `CompiledSelector` steps; components
//! and pseudo-classes the matcher does not support are ignored.
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

pub(crate) fn compile_nth_data(
    nth_data: &parcel_selectors::parser::NthSelectorData,
    step: &mut SelectorStep,
) {
    if let Some(pseudo_class) = nth_pseudo_class(nth_data) {
        step.pseudo_classes.push(pseudo_class);
    }
}

/// The structural pseudo-class an `:nth-*` component stands for, or `None` for the
/// type-based forms the matcher does not support.
fn nth_pseudo_class(nth_data: &parcel_selectors::parser::NthSelectorData) -> Option<PseudoClass> {
    match nth_data.ty {
        NthType::Child if nth_data.is_function() => Some(PseudoClass::NthChild {
            step_size: nth_data.a,
            offset: nth_data.b,
        }),
        NthType::Child => Some(PseudoClass::FirstChild),
        NthType::LastChild if nth_data.is_function() => Some(PseudoClass::NthLastChild {
            step_size: nth_data.a,
            offset: nth_data.b,
        }),
        NthType::LastChild => Some(PseudoClass::LastChild),
        NthType::OnlyChild => Some(PseudoClass::OnlyChild),
        _ => None,
    }
}

pub(crate) fn compile_pseudo_class(pseudo: &LightningPseudoClass<'_>, step: &mut SelectorStep) {
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
pub(crate) fn compile_selector(selector: &Selector<'_>) -> CompiledSelector {
    let mut steps = Vec::new();
    let mut current_step = SelectorStep::default();

    for component in selector.iter_raw_match_order() {
        if let Component::Combinator(comb) = component {
            current_step.combinator = convert_combinator(*comb);
            steps.push(std::mem::take(&mut current_step));
        } else {
            compile_component(component, &mut current_step);
        }
    }
    steps.push(current_step);

    CompiledSelector { steps }
}

/// Adds what one component other than a combinator requires to `step`; components the
/// matcher does not support are ignored.
fn compile_component(component: &Component<'_>, step: &mut SelectorStep) {
    match component {
        Component::LocalName(local_name) => {
            step.tag = Some(local_name.name.as_ref().to_lowercase());
        }
        Component::ID(id) => step.id = Some(id.as_ref().to_string()),
        Component::Class(class) => step.classes.push(class.as_ref().to_string()),
        Component::Nth(nth_data) => compile_nth_data(nth_data, step),
        Component::NthOf(nth_of_data) => compile_nth_data(nth_of_data.nth_data(), step),
        Component::Empty => step.pseudo_classes.push(PseudoClass::Empty),
        Component::Negation(selectors) => {
            step.negations
                .extend(selectors.as_ref().iter().map(compile_selector));
        }
        Component::NonTSPseudoClass(pseudo) => compile_pseudo_class(pseudo, step),
        _ => {}
    }
}
