//! The shared handles every module actor is built from: the signal hub, the surface manager, the
//! three outbound channels and the canvas factory.
use super::layout::LayoutEventSender;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::events::signals::SignalHub;
use crate::shared::wayland::ports::DynSurfaceManager;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ModuleRuntimeDependencies<
    Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> {
    pub(crate) hub: Arc<SignalHub>,
    pub(crate) surface_manager: DynSurfaceManager,
    pub(crate) layout_sender: Arc<LS>,
    pub(crate) display_sender: Arc<DS>,
    pub(crate) ui_sender: Arc<US>,
    pub(crate) canvas_factory: Fact,
}

impl<
    Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> ModuleRuntimeDependencies<Fact, LS, DS, US>
{
    #[must_use]
    pub(crate) fn new(
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        layout_sender: Arc<LS>,
        display_sender: Arc<DS>,
        ui_sender: Arc<US>,
        canvas_factory: Fact,
    ) -> Self {
        Self {
            hub,
            surface_manager,
            layout_sender,
            display_sender,
            ui_sender,
            canvas_factory,
        }
    }
}
