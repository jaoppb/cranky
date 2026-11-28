//! The run loop's machinery: what wakes `CrankyApp::run`, how it waits for that, and the
//! handler each wakeup goes to.

use super::{AppError, CrankyApp};
use crate::app::commands::SystemCommand;
use crate::features::layout_engine::domain::DisplayCommand;
use crate::features::module_runtime::ports::LayoutEvent;
use crate::features::vdom::domain::UiCommand;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::{HyprlandState, SignalHub};
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::Scale;
use crate::shared::wayland::ports::{DisplayServerError, DisplayServerPort};
use std::collections::HashMap;
use tokio::sync::watch;

/// What woke the run loop: one display event batch, the first of a run of queued commands or
/// layout events, or a signal that changed, with the value it changed to.
pub(super) enum Wakeup {
    /// The display server has events to dispatch, or failed waiting for them.
    DisplayEvents(Result<(), DisplayServerError>),
    Queued(Queued),
    Signal(SignalChange),
}

/// The first of a run of commands or layout events queued on one of the app's channels.
pub(super) enum Queued {
    Display(DisplayCommand),
    Layout(LayoutEvent),
    Interface(UiCommand),
    System(SystemCommand),
}

/// A hub signal that changed, with the value it changed to.
pub(super) enum SignalChange {
    Config(Box<Config>),
    Hyprland(Box<HyprlandState>),
    MonitorScales,
}

/// The display server and the systray host the run loop drives.
pub(super) struct LoopPorts<D, S> {
    pub(super) display: D,
    pub(super) sni: S,
}

/// The receivers of the hub signals the run loop reacts to.
pub(super) struct SignalReceivers {
    config: watch::Receiver<Config>,
    hyprland: watch::Receiver<HyprlandState>,
    monitor_scales: watch::Receiver<HashMap<MonitorId, Scale>>,
}

impl SignalReceivers {
    /// Subscribes to the configuration, Hyprland and monitor-scale signals of `hub`, in that
    /// order.
    pub(super) fn new(hub: &SignalHub) -> Self {
        Self {
            config: hub.config_rx(),
            hyprland: hub.hyprland_rx(),
            monitor_scales: hub.monitor_scales_rx(),
        }
    }

    /// Waits for the next change of any of the three signals; a signal whose sender is gone is
    /// no longer waited on, and with all three gone this never resolves.
    async fn next_change(&mut self) -> SignalChange {
        tokio::select! {
            Ok(()) = self.config.changed() => {
                SignalChange::Config(Box::new(self.config.borrow().clone()))
            }
            Ok(()) = self.hyprland.changed() => {
                SignalChange::Hyprland(Box::new(self.hyprland.borrow().clone()))
            }
            Ok(()) = self.monitor_scales.changed() => SignalChange::MonitorScales,
            else => std::future::pending().await,
        }
    }
}

impl<
    R: crate::features::module_runtime::ports::ModuleRegistryPort<F, LS, DS, US, Snd> + 'static,
    F: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
    LS: crate::features::module_runtime::ports::LayoutEventSender + 'static,
    DS: crate::features::layout_engine::domain::DisplayCommandSender + 'static,
    US: crate::features::vdom::domain::UiCommandSender + 'static,
    Snd: crate::features::module_runtime::ports::LayoutSender + 'static,
> CrankyApp<R, F, LS, DS, US, Snd>
{
    /// Waits for whichever comes first: display events, a queued command or layout event, or a
    /// signal change.
    pub(super) async fn next_wakeup(
        &mut self,
        display: &mut impl DisplayServerPort,
        signals: &mut SignalReceivers,
    ) -> Wakeup {
        tokio::select! {
            res = display.wait_for_events() => Wakeup::DisplayEvents(res),
            queued = self.next_queued() => Wakeup::Queued(queued),
            change = signals.next_change() => Wakeup::Signal(change),
        }
    }

    /// Waits for the next command or layout event on any of the four channels; a closed channel
    /// is no longer waited on, and with all four closed this never resolves.
    async fn next_queued(&mut self) -> Queued {
        tokio::select! {
            Some(display_cmd) = self.display_rx.recv() => Queued::Display(display_cmd),
            Some(layout_event) = self.layout_rx.recv() => Queued::Layout(layout_event),
            Some(ui_cmd) = self.ui_rx.recv() => Queued::Interface(ui_cmd),
            Some(sys_cmd) = self.system_rx.recv() => Queued::System(sys_cmd),
            else => std::future::pending().await,
        }
    }

    /// Hands what woke the run loop to its handler; fails only when the display server does.
    pub(super) async fn handle_wakeup(
        &mut self,
        wakeup: Wakeup,
        ports: &mut LoopPorts<
            impl DisplayServerPort,
            impl crate::features::systray::ports::SniPort,
        >,
    ) -> Result<(), AppError> {
        match wakeup {
            Wakeup::DisplayEvents(res) => Self::dispatch_display_events(res, &mut ports.display)?,
            Wakeup::Queued(queued) => self.handle_queued(queued, ports).await,
            Wakeup::Signal(change) => self.handle_signal_change(change, &mut ports.display),
        }
        Ok(())
    }

    /// Hands a queued command or layout event, and those queued behind it, to its handler.
    async fn handle_queued(
        &mut self,
        queued: Queued,
        ports: &mut LoopPorts<
            impl DisplayServerPort,
            impl crate::features::systray::ports::SniPort,
        >,
    ) {
        let LoopPorts { display, sni } = ports;
        match queued {
            Queued::Display(display_cmd) => self.handle_display_commands(display_cmd, display),
            Queued::Layout(layout_event) => {
                self.handle_layout_events(layout_event, display).await;
            }
            Queued::Interface(ui_cmd) => self.handle_ui_commands(ui_cmd, sni).await,
            Queued::System(sys_cmd) => self.handle_system_commands(sys_cmd, display),
        }
    }

    /// Hands a signal change to its handler.
    fn handle_signal_change(&mut self, change: SignalChange, display: &mut impl DisplayServerPort) {
        match change {
            SignalChange::Config(new_config) => self.handle_config_changed(*new_config),
            SignalChange::Hyprland(state) => self.handle_hyprland_changed(&state, display),
            SignalChange::MonitorScales => self.prune_removed_monitors(),
        }
    }

    /// Dispatches the events the display server woke the loop for, unless waiting for them
    /// failed.
    fn dispatch_display_events(
        res: Result<(), DisplayServerError>,
        display: &mut impl DisplayServerPort,
    ) -> Result<(), AppError> {
        res.map_err(|error| AppError::Internal {
            message: error.to_string(),
        })?;
        display
            .dispatch_pending()
            .map_err(|error| AppError::Internal {
                message: error.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::SignalReceivers;
    use crate::shared::config::domain::Config;
    use crate::shared::events::signals::SignalHub;

    /// With every signal sender gone, waiting for a signal change never resolves (rather than
    /// panicking on a `select!` with every branch disabled), so the run loop keeps waiting on
    /// its other wakeups.
    #[test]
    fn test_signal_receivers_wait_forever_once_the_hub_is_gone() {
        crate::test_utils::block_on(async {
            let hub = SignalHub::new(Config::default());
            let mut signals = SignalReceivers::new(&hub);
            drop(hub);

            let change = futures_util::FutureExt::now_or_never(signals.next_change());
            assert!(change.is_none());
        });
    }
}
