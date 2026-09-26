use super::adapter::WaylandAdapter;
use super::command::SurfaceCommand;
use super::floating_surface::{hide_floating, show_floating, FloatingPayload};
use super::render::render_outputs;
use super::surface_handler::handle_cmd;
use crate::features::layout_engine::domain::FloatingKind;
use crate::features::module_runtime::ports::LayoutSender;
use crate::shared::primitives::ModuleId;
use crate::shared::wayland::ports::{AppReadModel, DisplayServerError, DisplayServerPort};
use async_trait::async_trait;
use std::collections::HashMap;
use wayland_client::backend::WaylandError;

#[async_trait]
impl DisplayServerPort for WaylandAdapter {
    async fn wait_for_events(&mut self) -> Result<(), DisplayServerError> {
        let mut read_guard = self.event_queue.prepare_read();
        if let Some(r_guard) = read_guard.take() {
            tokio::select! {
                result = self.async_fd.readable() => {
                    if let Ok(mut guard) = result {
                        match r_guard.read() {
                            Ok(_) => {
                                guard.retain_ready();
                            }
                            Err(WaylandError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                guard.clear_ready();
                            }
                            Err(e) => return Err(DisplayServerError::ConnectionFailed { reason: e.to_string() }),
                        }
                    }
                }
                Some(()) = self.notify_rx.recv() => {
                    drop(r_guard);
                    let cmds: Vec<SurfaceCommand> = {
                        let mut map = self.pending_surfaces.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                        map.drain().map(|(_, cmd)| cmd).collect()
                    };
                    let qh = self.event_queue.handle();
                    for cmd in cmds {
                        handle_cmd(&mut self.state, &qh, &cmd)?;
                    }
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn dispatch_pending(&mut self) -> Result<(), DisplayServerError> {
        if self.config_rx.has_changed().unwrap_or(false) {
            let _ = self.config_rx.borrow_and_update();
            tracing::debug!("WaylandAdapter detected config change, recreating bars...");
            self.state.bars.clear();
            self.state.surface_to_id.clear();

            let outputs: Vec<_> = self
                .state
                .outputs
                .iter()
                .map(|o| o.output.clone())
                .collect();
            for output in outputs {
                let _ = self.state.create_bar(&output, &self.event_queue.handle());
            }
        }

        self.event_queue
            .dispatch_pending(&mut self.state)
            .map_err(|e| DisplayServerError::ConnectionFailed {
                reason: e.to_string(),
            })?;

        let outputs_missing_bars: Vec<_> = self
            .state
            .outputs
            .iter()
            .filter(|o| !o.name.is_empty())
            .filter(|o| !self.state.bars.iter().any(|b| b.output_name == o.name))
            .map(|o| o.output.clone())
            .collect();

        for output in outputs_missing_bars {
            if let Err(e) = self.state.create_bar(&output, &self.event_queue.handle()) {
                tracing::debug!("Deferred bar creation: {}", e);
            }
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), DisplayServerError> {
        let _ = self.connection.flush();
        Ok(())
    }

    fn render_all(
        &mut self,
        read_model: &AppReadModel,
        layout_senders: &HashMap<ModuleId, Box<dyn LayoutSender>>,
    ) -> Result<(), DisplayServerError> {
        render_outputs(&mut self.state, &self.connection, read_model, layout_senders);
        Ok(())
    }

    fn show_floating_surface(
        &mut self,
        floating: crate::features::layout_engine::domain::ShowFloating,
    ) -> Result<(), DisplayServerError> {
        let qh = self.event_queue.handle();
        show_floating(
            &mut self.state,
            &qh,
            floating.kind,
            floating.monitor_id,
            floating.anchor_rect,
            &FloatingPayload {
                buffer: &floating.buffer,
                logical_size: floating.logical_size,
                offset: floating.offset,
            },
            floating.parent,
        )
    }

    fn hide_floating_surface(&mut self, kind: &FloatingKind) -> Result<(), DisplayServerError> {
        hide_floating(&mut self.state, kind);
        Ok(())
    }
}
