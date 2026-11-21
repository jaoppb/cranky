//! The virtual DOM scripts render into: the `VNode` tree, its identifiers and actions, the differ
//! that tells a render pipeline whether anything changed, and the bridge that resolves a tree's
//! styles.
pub mod adapters;
pub mod domain;
pub mod ports;
