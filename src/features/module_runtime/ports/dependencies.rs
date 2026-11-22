//! The shared handles every module actor is built from: the signal hub, the surface manager, the
//! three outbound channels and the canvas factory.
use super::layout::LayoutEventSender;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::events::signals::SignalHub;
use crate::shared::wayland::ports::DynSurfaceManager;
use std::sync::Arc;

/// The three channels a module actor sends through: layout events and UI commands to the app,
/// display commands to the display server.
pub(crate) struct ModuleSenders<LS, DS, US> {
    layout_events: Arc<LS>,
    display_commands: Arc<DS>,
    ui_commands: Arc<US>,
}

impl<LS: LayoutEventSender, DS: DisplayCommandSender, US: UiCommandSender>
    ModuleSenders<LS, DS, US>
{
    #[must_use]
    pub(crate) const fn new(
        layout_events: Arc<LS>,
        display_commands: Arc<DS>,
        ui_commands: Arc<US>,
    ) -> Self {
        Self {
            layout_events,
            display_commands,
            ui_commands,
        }
    }

    #[must_use]
    pub(crate) fn layout_sender(&self) -> &LS {
        self.layout_events.as_ref()
    }

    #[must_use]
    pub(crate) fn display_sender(&self) -> &DS {
        self.display_commands.as_ref()
    }

    #[must_use]
    pub(crate) fn ui_sender(&self) -> &US {
        self.ui_commands.as_ref()
    }
}

impl<LS, DS, US> Clone for ModuleSenders<LS, DS, US> {
    fn clone(&self) -> Self {
        Self {
            layout_events: Arc::clone(&self.layout_events),
            display_commands: Arc::clone(&self.display_commands),
            ui_commands: Arc::clone(&self.ui_commands),
        }
    }
}

/// What a module actor's context talks to the rest of the app through: the signal hub it reads,
/// the surface manager it submits its buffers to, and the channels it sends through.
pub(crate) struct ModuleHandles<LS, DS, US> {
    hub: Arc<SignalHub>,
    surface_manager: DynSurfaceManager,
    senders: ModuleSenders<LS, DS, US>,
}

impl<LS, DS, US> ModuleHandles<LS, DS, US> {
    #[must_use]
    pub(crate) const fn new(
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        senders: ModuleSenders<LS, DS, US>,
    ) -> Self {
        Self {
            hub,
            surface_manager,
            senders,
        }
    }

    #[must_use]
    pub(crate) const fn hub(&self) -> &Arc<SignalHub> {
        &self.hub
    }

    #[must_use]
    pub(crate) const fn surface_manager(&self) -> &DynSurfaceManager {
        &self.surface_manager
    }

    #[must_use]
    pub(crate) const fn senders(&self) -> &ModuleSenders<LS, DS, US> {
        &self.senders
    }
}

impl<LS, DS, US> Clone for ModuleHandles<LS, DS, US> {
    fn clone(&self) -> Self {
        Self {
            hub: Arc::clone(&self.hub),
            surface_manager: Arc::clone(&self.surface_manager),
            senders: self.senders.clone(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ModuleRuntimeDependencies<
    Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> {
    pub(crate) handles: ModuleHandles<LS, DS, US>,
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
    pub(crate) const fn new(handles: ModuleHandles<LS, DS, US>, canvas_factory: Fact) -> Self {
        Self {
            handles,
            canvas_factory,
        }
    }
}
