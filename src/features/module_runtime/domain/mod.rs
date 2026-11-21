//! Pure module-runtime logic: the identity of each module site, pointer handling, and the
//! per-monitor render pipeline.
pub mod identity;
pub(super) mod pointer_handler;
pub mod render_pipeline;

pub(crate) use identity::ModuleIdentity;
pub(in crate::features::module_runtime) use pointer_handler::{PointerAction, PointerHandler};
#[cfg(test)]
pub(crate) use render_pipeline::{FloatingLayouts, SizeChange};
pub(crate) use render_pipeline::{LayoutContext, PopupRenderLayout, RenderOutcome, RenderPipeline};
