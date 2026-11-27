//! The four channels the bar's actors talk over, and how they are handed out at boot.

use crate::app::commands::SystemCommand;
use crate::features::layout_engine::domain::DisplayCommand;
use crate::features::module_runtime::ports::{
    LayoutEvent, ModuleHandles, ModuleRuntimeDependencies, ModuleSenders,
};
use crate::features::vdom::domain::UiCommand;
use crate::shared::events::signals::SignalHub;
use crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory;
use crate::shared::wayland::ports::DynSurfaceManager;
use std::sync::Arc;
use tokio::sync::mpsc;

/// How many messages each boot channel holds before a sender waits or a `try_send` fails.
const CHANNEL_CAPACITY: usize = 100;

/// The module runtime dependencies the bar runs with: the tiny-skia canvas factory and the
/// senders of the layout, display and UI channels.
pub(super) type BarDependencies = ModuleRuntimeDependencies<
    TinySkiaCanvasFactory,
    mpsc::Sender<LayoutEvent>,
    mpsc::Sender<DisplayCommand>,
    mpsc::Sender<UiCommand>,
>;

/// Both ends of the display, layout, UI and system channels, each bounded to
/// [`CHANNEL_CAPACITY`] messages.
pub(super) struct BootChannels {
    display_tx: mpsc::Sender<DisplayCommand>,
    display_rx: mpsc::Receiver<DisplayCommand>,
    layout_tx: mpsc::Sender<LayoutEvent>,
    layout_rx: mpsc::Receiver<LayoutEvent>,
    ui_tx: mpsc::Sender<UiCommand>,
    ui_rx: mpsc::Receiver<UiCommand>,
    system_tx: mpsc::Sender<SystemCommand>,
    system_rx: mpsc::Receiver<SystemCommand>,
}

impl BootChannels {
    /// Opens the display, layout, UI and system channels, in that order.
    pub(super) fn new() -> Self {
        let (display_tx, display_rx) = mpsc::channel::<DisplayCommand>(CHANNEL_CAPACITY);
        let (layout_tx, layout_rx) = mpsc::channel::<LayoutEvent>(CHANNEL_CAPACITY);
        let (ui_tx, ui_rx) = mpsc::channel::<UiCommand>(CHANNEL_CAPACITY);
        let (system_tx, system_rx) = mpsc::channel::<SystemCommand>(CHANNEL_CAPACITY);
        Self {
            display_tx,
            display_rx,
            layout_tx,
            layout_rx,
            ui_tx,
            ui_rx,
            system_tx,
            system_rx,
        }
    }

    /// The sender of the display channel, which the Wayland adapter also writes to.
    pub(super) const fn display_tx(&self) -> &mpsc::Sender<DisplayCommand> {
        &self.display_tx
    }

    /// Hands the module senders, with `hub`, `surface_manager` and a tiny-skia canvas factory,
    /// to the module runtime and every receiver to the application, and gives back the system
    /// sender for the file watchers.
    pub(super) fn into_app_parts(
        self,
        hub: &Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
    ) -> (
        BarDependencies,
        crate::app::state::StateChannels,
        mpsc::Sender<SystemCommand>,
    ) {
        let canvas_factory = TinySkiaCanvasFactory::new();
        let deps = ModuleRuntimeDependencies::new(
            ModuleHandles::new(
                hub.clone(),
                surface_manager,
                ModuleSenders::new(
                    Arc::new(self.layout_tx),
                    Arc::new(self.display_tx),
                    Arc::new(self.ui_tx),
                ),
            ),
            canvas_factory,
        );
        let channels = crate::app::state::StateChannels::new(
            crate::app::state::ModuleReceivers::new(self.layout_rx, self.display_rx, self.ui_rx),
            self.system_rx,
        );
        (deps, channels, self.system_tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::Config;

    #[test]
    fn test_every_channel_holds_a_hundred_messages() {
        let channels = BootChannels::new();
        assert_eq!(channels.display_tx.max_capacity(), 100);
        assert_eq!(channels.layout_tx.max_capacity(), 100);
        assert_eq!(channels.ui_tx.max_capacity(), 100);
        assert_eq!(channels.system_tx.max_capacity(), 100);
    }

    #[test]
    fn test_each_sender_reaches_its_own_receiver() {
        let mut channels = BootChannels::new();
        let command = SystemCommand::ReloadModule("clock".into());
        channels.system_tx.try_send(command.clone()).unwrap();
        channels
            .display_tx
            .try_send(DisplayCommand::RequestRender)
            .unwrap();

        assert_eq!(channels.system_rx.try_recv().unwrap(), command);
        assert!(matches!(
            channels.display_rx.try_recv().unwrap(),
            DisplayCommand::RequestRender
        ));
        assert!(channels.layout_rx.try_recv().is_err());
        assert!(channels.ui_rx.try_recv().is_err());
    }

    /// The runtime gets the hub and the module senders, and the application keeps every
    /// receiver open: a send on each sender is accepted and uses one slot of its channel.
    #[test]
    fn test_app_parts_keep_every_channel_open() {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let surface_manager: DynSurfaceManager =
            Arc::new(crate::features::module_runtime::test_support::MockSurfaceManager);

        let (deps, _channels, system_tx) =
            BootChannels::new().into_app_parts(&hub, surface_manager);

        assert!(Arc::ptr_eq(deps.handles.hub(), &hub));
        let senders = deps.handles.senders();
        senders
            .display_sender()
            .try_send(DisplayCommand::RequestRender)
            .unwrap();
        assert_eq!(senders.display_sender().capacity(), 99);
        system_tx
            .try_send(SystemCommand::ReloadModule("clock".into()))
            .unwrap();
        assert_eq!(system_tx.capacity(), 99);
        assert_eq!(senders.layout_sender().capacity(), 100);
        assert!(!senders.layout_sender().is_closed());
        assert!(!senders.ui_sender().is_closed());
    }
}
