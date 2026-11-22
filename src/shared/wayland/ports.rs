//! The ports the app and the render pipelines drive the display server through, and its error.
use crate::shared::primitives::render::ModuleFrame;
pub use crate::shared::wayland::domain::AppReadModel;
#[cfg(test)]
pub use crate::shared::wayland::domain::ModuleLayout;
pub(crate) use crate::shared::wayland::surface_parent::SurfaceParent;
use async_trait::async_trait;
use std::sync::Arc;

/// Where a render pipeline hands its finished buffers, to be shown on the module's surface.
pub trait SurfaceManagerPort: Send + Sync {
    /// Submit a module's rendered frame for one monitor.
    fn submit_buffer(&self, frame: ModuleFrame);

    /// Submit a module's rendered frame with a parent surface to
    /// subsurface against — `None` for the root painting its own
    /// layer-surface buffer directly, `Some` for anything embedded via
    /// `ui.module(name)`, wherever it was embedded.
    fn submit_child_buffer(&self, frame: ModuleFrame, parent: Option<SurfaceParent>) {
        let _ = parent;
        self.submit_buffer(frame);
    }
}

pub(crate) type DynSurfaceManager = Arc<dyn SurfaceManagerPort>;

use thiserror::Error;

/// Failure in the display server connection or in drawing its surfaces.
#[derive(Error, Debug)]
pub enum DisplayServerError {
    /// The compositor connection failed, lacks a global or output the bar needs (such as `wl_shm`),
    /// or reading or dispatching its events failed.
    #[error("Display server connection failed: {reason}")]
    ConnectionFailed {
        /// The connection error, as text.
        reason: String,
    },

    /// An `std::io::Error` passed up with `?`; the Wayland adapter reports its own I/O failures as
    /// `ConnectionFailed` or `Internal`.
    #[error("I/O error: {0}")]
    InputOutput(#[from] std::io::Error),
    /// Any other failure, such as a missing bar or a shared-memory buffer that could not be
    /// created; the message says which.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// What the app's event loop drives the display server through: waiting for and dispatching events,
/// flushing, rendering, and showing or hiding floating surfaces.
#[async_trait]
pub trait DisplayServerPort: Send + Sync {
    /// # Errors
    ///
    /// Returns `DisplayServerError` if waiting for display events fails.
    async fn wait_for_events(&mut self) -> Result<(), DisplayServerError>;

    /// # Errors
    ///
    /// Returns `DisplayServerError` if dispatching pending events fails.
    fn dispatch_pending(&mut self) -> Result<(), DisplayServerError>;

    /// # Errors
    ///
    /// Returns `DisplayServerError` if flushing requests to the display server fails.
    fn flush(&mut self) -> Result<(), DisplayServerError>;

    /// # Errors
    ///
    /// Returns `DisplayServerError` if rendering fails.
    fn render_all<Snd: crate::features::module_runtime::ports::LayoutSender + 'static>(
        &mut self,
        read_model: &AppReadModel,
        layout_senders: &std::collections::HashMap<crate::shared::primitives::ModuleId, Snd>,
    ) -> Result<(), DisplayServerError>;

    /// # Errors
    ///
    /// Returns `DisplayServerError` if displaying the floating surface fails.
    fn show_floating_surface(
        &mut self,
        floating: crate::features::layout_engine::domain::ShowFloating,
    ) -> Result<(), DisplayServerError>;

    /// # Errors
    ///
    /// Returns `DisplayServerError` if hiding the floating surface fails.
    fn hide_floating_surface(
        &mut self,
        kind: &crate::features::layout_engine::domain::FloatingKind,
    ) -> Result<(), DisplayServerError>;
}
