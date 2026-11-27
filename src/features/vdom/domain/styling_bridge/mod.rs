//! Turns a `VNode` tree into the layout engine's styled tree, resolving each node's CSS, including
//! the pseudo-classes pointer interaction sets.
pub mod anchor;
mod mapper;
pub mod pseudo;
mod resolver;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_interaction;
#[cfg(test)]
mod tests_mapping;
#[cfg(test)]
mod tests_resolution;
