//! Validated names a script or module hands to the styling layer: classes, element ids and
//! stylesheet names.
pub mod class_name;
pub mod element_id;
pub mod stylesheet_name;

pub use class_name::*;
pub use element_id::*;
pub use stylesheet_name::*;
