//! Colors and the paints (solid or gradient) parsed from style and config strings.
mod color_struct;
mod drawing_color;
pub mod error;
mod parser;
#[cfg(test)]
mod tests;

pub use color_struct::Color;
pub use drawing_color::DrawingColor;
#[cfg(test)]
pub(crate) use error::ColorError;
