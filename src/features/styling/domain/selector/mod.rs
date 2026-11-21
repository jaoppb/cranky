//! Compiled selectors, the per-sheet index that narrows candidate rules, and the matcher.
pub mod bucket;
pub mod matcher;
pub mod model;

pub(crate) use bucket::*;
pub(crate) use matcher::*;
pub(crate) use model::*;
