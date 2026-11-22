//! The adapters one module's render pipeline works with, injected when its actor is built.
use crate::features::styling::ports::StyleResolverPort;
use crate::features::vdom::ports::VdomDiffPort;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::sync::Arc;

/// The adapters one module's render pipeline works with: the vdom differ of its diff phase, the
/// style resolver of its layout phase, and the canvas factory it measures text and paints with.
pub(crate) struct RenderServices<F: CanvasFactory> {
    pub(crate) canvas_factory: F,
    pub(crate) style_resolver: Arc<dyn StyleResolverPort>,
    pub(crate) vdom_diff: Arc<dyn VdomDiffPort>,
}

impl<F: CanvasFactory> RenderServices<F> {
    #[must_use]
    pub(crate) fn new(
        canvas_factory: F,
        style_resolver: Arc<dyn StyleResolverPort>,
        vdom_diff: Arc<dyn VdomDiffPort>,
    ) -> Self {
        Self {
            canvas_factory,
            style_resolver,
            vdom_diff,
        }
    }
}
