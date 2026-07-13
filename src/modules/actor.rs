use crate::domain::commands::AppCommand;
use crate::domain::shared::render::RenderBuffer;
use crate::domain::signals::SignalHub;
use crate::domain::{
    MonitorId,
    shared::geometry::{Rect, Scale, Size},
};
use crate::ports::registry::AnyModulePort;
use crate::ports::surface::DynSurfaceManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::watch;

pub struct ModuleContext {
    id: crate::domain::ModuleId,
    hub: Arc<SignalHub>,
    surface_manager: DynSurfaceManager,
    command_tx: Arc<dyn crate::ports::registry::CommandSender>,
    layout_rx: watch::Receiver<HashMap<MonitorId, Rect>>,
    pointer_rx: tokio::sync::broadcast::Receiver<(
        crate::domain::ModuleId,
        crate::domain::events::PointerEvent,
    )>,
}

impl ModuleContext {
    pub fn new(
        id: crate::domain::ModuleId,
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        command_tx: Arc<dyn crate::ports::registry::CommandSender>,
        layout_rx: watch::Receiver<HashMap<MonitorId, Rect>>,
    ) -> Self {
        let pointer_rx = hub.pointer_rx();
        Self {
            id,
            hub,
            surface_manager,
            command_tx,
            layout_rx,
            pointer_rx,
        }
    }

    pub fn id(&self) -> crate::domain::ModuleId {
        self.id
    }

    pub fn hub(&self) -> &Arc<SignalHub> {
        &self.hub
    }

    pub fn surface_manager(&self) -> &DynSurfaceManager {
        &self.surface_manager
    }

    pub fn command_tx(&self) -> &dyn crate::ports::registry::CommandSender {
        self.command_tx.as_ref()
    }

    pub fn rxs_mut(
        &mut self,
    ) -> (
        &mut watch::Receiver<HashMap<MonitorId, Rect>>,
        &mut crate::domain::events::PointerReceiver,
    ) {
        (&mut self.layout_rx, &mut self.pointer_rx)
    }
}

pub struct ModuleActor<F: crate::ports::canvas::CanvasFactory + 'static> {
    port: Box<dyn AnyModulePort>,
    ctx: ModuleContext,
    sizes: HashMap<MonitorId, Size>,
    canvas_factory: std::sync::Arc<std::sync::Mutex<F>>,
}

impl<F: crate::ports::canvas::CanvasFactory + 'static> ModuleActor<F> {
    pub fn new(
        port: Box<dyn AnyModulePort>,
        ctx: ModuleContext,
        canvas_factory: std::sync::Arc<std::sync::Mutex<F>>,
    ) -> Self {
        Self {
            port,
            ctx,
            sizes: HashMap::new(),
            canvas_factory,
        }
    }

    pub fn spawn(mut self) {
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();

            use futures_util::stream::{SelectAll, StreamExt};
            use tokio_stream::wrappers::WatchStream;
            use crate::domain::signals::SignalKind;

            let mut events_stream = SelectAll::new();
            let subs = self.port.subscriptions();

            if subs.contains(&SignalKind::Time) {
                events_stream.push(WatchStream::new(self.ctx.hub().time_rx()).map(|_| SignalKind::Time).boxed());
            }
            if subs.contains(&SignalKind::Hyprland) {
                events_stream.push(WatchStream::new(self.ctx.hub().hyprland_rx()).map(|_| SignalKind::Hyprland).boxed());
            }
            if subs.contains(&SignalKind::Applets) {
                events_stream.push(WatchStream::new(self.ctx.hub().applets_rx()).map(|_| SignalKind::Applets).boxed());
            }
            if subs.contains(&SignalKind::Metrics) {
                events_stream.push(WatchStream::new(self.ctx.hub().metrics_rx()).map(|_| SignalKind::Metrics).boxed());
            }
            if subs.iter().any(|s| matches!(s, SignalKind::DBus(_))) {
                events_stream.push(WatchStream::new(self.ctx.hub().dbus_rx()).map(|_| SignalKind::DBus(crate::domain::dbus::DBusSubscription { bus: crate::domain::dbus::BusType::Session, destination: None, path: None, interface: None, member: None })).boxed());
            }

            // Initial refresh
            self.port.refresh(self.ctx.hub());
            self.measure_and_render_all();

            loop {
                // Determine what woke us up
                let mut changed = false;

                let should_continue = rt.block_on(async {
                    let ctx_id = self.ctx.id();
                    let (layout_rx, input_rx) = self.ctx.rxs_mut();

                    tokio::select! {
                        Some(_) = events_stream.next(), if !events_stream.is_empty() => {
                            changed = true;
                            // Drain any immediately pending events from the select_all stream to debounce
                            while let Some(Some(_)) = futures_util::FutureExt::now_or_never(events_stream.next()) {
                                // drained
                            }
                        }
                        res = layout_rx.changed() => {
                            if res.is_err() {
                                return false; // layout_rx dropped, we should exit
                            }
                        }
                        Ok((target_id, event)) = input_rx.recv() => {
                            if target_id == ctx_id {
                                self.port.on_pointer_event(event, self.ctx.command_tx());
                                changed = true;
                            }
                        }
                    }

                    // Debounce rapid layout changes
                    tokio::time::sleep(std::time::Duration::from_millis(16)).await;
                    
                    if self.ctx.rxs_mut().0.has_changed().unwrap_or(false) {
                        let _ = self.ctx.rxs_mut().0.changed().await;
                    }
                    
                    true
                });

                if !should_continue {
                    break;
                }

                if changed {
                    self.port.refresh(self.ctx.hub());
                }

                self.measure_and_render_all();
            }
        });
    }

    #[tracing::instrument(level = "debug", skip(self), fields(module = %self.ctx.id()))]
    fn measure_and_render_all(
        &mut self,
    ) {
        let t0 = std::time::Instant::now();
        let monitors: Vec<MonitorId> = self
            .ctx
            .hub()
            .hyprland_rx()
            .borrow()
            .monitors()
            .values()
            .map(|m| MonitorId::new(m.name().as_str()))
            .collect();
        let layouts: std::collections::HashMap<MonitorId, Rect> =
            self.ctx.rxs_mut().0.borrow().clone();

        for monitor_id in monitors {
            // Measure
            let mut dummy_data = vec![0u8; 4];
            let _dummy_pixmap = tiny_skia::PixmapMut::from_bytes(&mut dummy_data, 1, 1).unwrap();
            let config = self.ctx.hub().config_rx().borrow().clone();
            let default_font_family = config.bar().font_family().clone();
            let default_font_size = config.bar().font_size();

            let size = {
                let mut factory = self.canvas_factory.lock().unwrap();
                let mut dummy_canvas = factory.create_canvas(
                    &mut dummy_data,
                    Size::new(1, 1),
                    Scale::new(1.0),
                    default_font_family.clone(),
                    default_font_size,
                );
                self.port.measure(&mut dummy_canvas, &monitor_id)
            };

            let old_size = self
                .sizes
                .get(&monitor_id)
                .copied()
                .unwrap_or(Size::new(0, 0));
            if size != old_size {
                self.sizes.insert(monitor_id.clone(), size);
                self
                    .ctx
                    .command_tx()
                    .send_command(AppCommand::ModuleSizeChanged(
                        monitor_id.clone(),
                        self.ctx.id(),
                        size,
                    ));
            }

            // Render if we have bounds
            if let Some(bounds) = layouts.get(&monitor_id)
                && bounds.width() > 0
                && bounds.height() > 0
            {
                let w = bounds.width();
                let h = bounds.height();
                let mut data = vec![0u8; (w * h * 4) as usize];
                {
                    let config = self.ctx.hub().config_rx().borrow().clone();
                    let default_font_family = config.bar().font_family().clone();
                    let default_font_size = config.bar().font_size();

                    let mut factory = self.canvas_factory.lock().unwrap();
                    let mut canvas = factory.create_canvas(
                        &mut data,
                        *bounds.size(),
                        Scale::new(1.0),
                        default_font_family,
                        default_font_size,
                    );
                    self.port.view(&mut canvas, &monitor_id);
                }

                let buffer = RenderBuffer::new(data, *bounds.size());
                let rt = tokio::runtime::Handle::current();
                let sm = self.ctx.surface_manager().clone();
                let mod_id = self.ctx.id();
                let mon_id = monitor_id.clone();
                let position = crate::domain::shared::geometry::Position::new(bounds.x(), bounds.y());
                rt.block_on(async move {
                    sm.submit_buffer(mod_id, mon_id, position, buffer).await;
                });
            }
        }

        tracing::debug!(
            module = %self.ctx.id(),
            duration_ms = t0.elapsed().as_millis(),
            duration_micros = t0.elapsed().as_micros(),
            "Module UI updated"
        );
    }
}
