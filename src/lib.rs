#![deny(unsafe_code)]
#![warn(clippy::type_complexity, clippy::needless_lifetimes)]

pub mod app;
pub mod features;
pub mod shared;
pub mod utils;

#[cfg(test)]
#[macro_use]
pub mod test_utils;
