pub mod color_struct;
pub mod drawing_color;
pub mod error;
pub(crate) mod parser;
#[cfg(test)]
mod tests;

pub use color_struct::Color;
pub use drawing_color::DrawingColor;
pub use error::ColorError;
