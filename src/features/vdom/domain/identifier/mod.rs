//! Identifiers for VDOM nodes: per-build ids, script-given keys, positional paths, and the refs
//! that remember pointer interaction across renders.
pub mod keyed_tree;
pub mod node_id;
mod node_key;
mod node_path;
pub mod node_ref;

pub(crate) use keyed_tree::{KeyedTree, resolve_segment};
pub(crate) use node_id::NodeId;
pub use node_key::NodeKey;
pub use node_path::{NodePath, SurfaceSpace};
pub(crate) use node_ref::{AnchorSegment, InteractionContext, NodeRef};
