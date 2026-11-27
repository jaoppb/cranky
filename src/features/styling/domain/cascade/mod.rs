//! The CSS cascade: orders matched rules by priority, merges their declarations, then resolves
//! inheritance, `em`/`rem` and `var()` for one element.
pub mod declared_length;
pub mod declared_lengths;
pub mod declared_lengths_apply;
pub mod declared_parts;
pub mod declared_style;
pub mod declared_style_resolve;
pub mod fold;
pub mod inherited;
pub mod priority;
pub mod rule_match;
pub mod specificity;
pub mod value;
pub mod var_substitution;

#[cfg(test)]
mod tests_declared_style;

pub(crate) use declared_length::*;
pub(crate) use declared_lengths::*;
pub(crate) use declared_parts::*;
pub(crate) use declared_style::*;
pub(crate) use fold::*;
pub use inherited::*;
pub(crate) use priority::*;
pub use rule_match::*;
pub(crate) use specificity::*;
pub(crate) use value::*;
