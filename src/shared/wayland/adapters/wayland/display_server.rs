use super::adapter::{WaylandAdapter, connection_failed};
use super::command::SurfaceCommand;
use super::floating_surface::{hide_floating, show_floating};
use super::render::render_outputs;
use super::surface_handler::handle_cmd;
use crate::features::layout_engine::domain::FloatingKind;
use crate::features::module_runtime::ports::LayoutSender;
use crate::shared::primitives::ModuleId;
use crate::shared::wayland::ports::{AppReadModel, DisplayServerError, DisplayServerPort};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::io::Interest;
use tokio::net::UnixStream;
use wayland_client::backend::{ReadEventsGuard, WaylandError};
use wayland_client::protocol::wl_output::WlOutput;

#[async_trait]
impl DisplayServerPort for WaylandAdapter {
    async fn wait_for_events(&mut self) -> Result<(), DisplayServerError> {
        let mut read_guard = self.event_queue.prepare_read();
        if let Some(r_guard) = read_guard.take() {
            tokio::select! {
                result = self.socket.readable() => {
                    if result.is_ok() {
                        read_events(&self.socket, r_guard)?;
                    }
                }
                Some(()) = self.notify_rx.recv() => {
                    drop(r_guard);
                    return self.apply_pending_surfaces();
                }
            }
        }
        Ok(())
    }

    fn dispatch_pending(&mut self) -> Result<(), DisplayServerError> {
        self.recreate_bars_on_config_change();
        self.event_queue
            .dispatch_pending(&mut self.state)
            .map_err(|error| connection_failed(&error))?;
        for output in self.state.outputs_without_bars() {
            self.create_bar_or_defer(&output);
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), DisplayServerError> {
        let _ = self.connection.flush();
        Ok(())
    }

    fn render_all<Snd: LayoutSender + 'static>(
        &mut self,
        read_model: &AppReadModel,
        layout_senders: &HashMap<ModuleId, Snd>,
    ) -> Result<(), DisplayServerError> {
        render_outputs(&mut self.state, read_model, layout_senders);
        // Without a bar nothing was rendered, and nothing is flushed.
        if !self.state.bars.is_empty() {
            let _ = self.connection.flush();
        }
        Ok(())
    }

    fn show_floating_surface(
        &mut self,
        floating: crate::features::layout_engine::domain::ShowFloating,
    ) -> Result<(), DisplayServerError> {
        let queue_handle = self.event_queue.handle();
        show_floating(&mut self.state, &queue_handle, floating)
    }

    fn hide_floating_surface(&mut self, kind: &FloatingKind) -> Result<(), DisplayServerError> {
        hide_floating(&mut self.state, kind);
        Ok(())
    }
}

/// Reads the events waiting on `socket` into the queue `read_guard` was
/// prepared for.
fn read_events(socket: &UnixStream, read_guard: ReadEventsGuard) -> Result<(), DisplayServerError> {
    // The outer `Err` is only ever `WouldBlock`: `try_io` clears the
    // readiness and the next call waits again. A real failure travels
    // as the inner `Err`.
    let read = socket.try_io(Interest::READABLE, || match read_guard.read() {
        Ok(_) => Ok(Ok(())),
        Err(WaylandError::Io(error)) if error.kind() == std::io::ErrorKind::WouldBlock => {
            Err(error)
        }
        Err(error) => Ok(Err(error)),
    });
    if let Ok(Err(error)) = read {
        return Err(connection_failed(&error));
    }
    Ok(())
}

impl WaylandAdapter {
    /// Applies every surface command queued since the last wake, the latest
    /// per module and monitor.
    fn apply_pending_surfaces(&mut self) -> Result<(), DisplayServerError> {
        let cmds: Vec<SurfaceCommand> = {
            let mut map = self
                .pending_surfaces
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            map.drain().map(|(_, cmd)| cmd).collect()
        };
        let queue_handle = self.event_queue.handle();
        for cmd in cmds {
            handle_cmd(&mut self.state, &queue_handle, &cmd)?;
        }
        Ok(())
    }

    /// Rebuilds every bar when the config changed since the last look.
    fn recreate_bars_on_config_change(&mut self) {
        let Ok(true) = self.config_rx.has_changed() else {
            return;
        };
        let _ = self.config_rx.borrow_and_update();
        tracing::debug!("WaylandAdapter detected config change, recreating bars...");
        self.recreate_bars();
    }

    /// Drops every bar and pointer route, then creates a bar per output.
    fn recreate_bars(&mut self) {
        self.state.bars.clear();
        self.state.surface_to_id.clear();
        let outputs: Vec<_> = self
            .state
            .outputs
            .iter()
            .map(|info| info.output.clone())
            .collect();
        for output in outputs {
            let _ = self.state.create_bar(&output, &self.event_queue.handle());
        }
    }

    /// Creates `output`'s bar, or logs why it has to wait.
    fn create_bar_or_defer(&mut self, output: &WlOutput) {
        let Err(error) = self.state.create_bar(output, &self.event_queue.handle()) else {
            return;
        };
        tracing::debug!("Deferred bar creation: {}", error);
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::config::domain::{Config, PopupEmbed, RootConfig};
    use crate::shared::events::core::SurfaceKind;
    use crate::shared::primitives::geometry::BarHeight;
    use crate::shared::primitives::{ModuleId, MonitorId};
    use crate::shared::wayland::adapters::wayland::adapter::{WaylandAdapter, readiness_socket};
    use crate::shared::wayland::adapters::wayland::state::WaylandState;
    use crate::shared::wayland::adapters::wayland::tests::fixture::Fake;
    use crate::shared::wayland::ports::DisplayServerPort;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// An adapter over `state` on the fixture's connection.
    fn adapter_over(fake: &Fake, state: WaylandState) -> WaylandAdapter {
        let config_rx = state.hub.config_rx();
        let connection = fake.connection().clone();
        let (_notify_tx, notify_rx) = tokio::sync::mpsc::channel(1);
        WaylandAdapter {
            socket: readiness_socket(&connection).expect("socket"),
            event_queue: connection.new_event_queue(),
            connection,
            state,
            pending_surfaces: Arc::new(Mutex::new(HashMap::new())),
            notify_rx,
            config_rx,
        }
    }

    fn bar_outputs(adapter: &WaylandAdapter) -> Vec<(&str, u32)> {
        adapter
            .state
            .bars
            .iter()
            .map(|bar| (bar.output_name.as_str(), bar.config_height))
            .collect()
    }

    /// A config change drops every bar and every pointer route, then builds
    /// a bar of the new height for each output.
    #[test]
    fn config_change_recreates_every_bar() {
        crate::test_utils::block_on(async {
            let fake = Fake::new();
            let mut state = fake.state(PopupEmbed::Composite);
            state.outputs.push(fake.output_info(7, "DP-1"));
            let old = fake.bar("DP-1", 1);
            let old_layer = old.layer_surface.clone();
            let route = (ModuleId::new(1), MonitorId::new("DP-1"), SurfaceKind::Bar);
            state.surface_to_id.insert(old.surface.clone(), route);
            state.bars.push(old);
            let config_tx = state.hub.config_tx();
            let mut adapter = adapter_over(&fake, state);
            let root = RootConfig::default().with_height(BarHeight::new(40));
            config_tx
                .send(Config::default().with_root(root))
                .expect("config");

            adapter.dispatch_pending().expect("dispatched");

            assert_eq!(bar_outputs(&adapter), vec![("DP-1", 40)]);
            assert_ne!(adapter.state.bars[0].layer_surface, old_layer);
            assert!(adapter.state.surface_to_id.is_empty());
        });
    }

    /// Each named output without a bar gets one; an unnamed one waits, and a
    /// bar that cannot be created yet is deferred without failing.
    #[test]
    fn outputs_without_bars_get_one_once_named() {
        crate::test_utils::block_on(async {
            let fake = Fake::new();
            let mut state = fake.state(PopupEmbed::Composite);
            state.outputs.push(fake.output_info(7, "DP-1"));
            state.outputs.push(fake.output_info(8, ""));
            let mut adapter = adapter_over(&fake, state);
            adapter.dispatch_pending().expect("dispatched");
            assert_eq!(bar_outputs(&adapter), vec![("DP-1", 30)]);

            let mut state = fake.state(PopupEmbed::Composite);
            state.compositor = None;
            state.outputs.push(fake.output_info(9, "HDMI-A-1"));
            let mut deferred = adapter_over(&fake, state);
            deferred.dispatch_pending().expect("deferred, not failed");
            assert!(deferred.state.bars.is_empty());
        });
    }
}
