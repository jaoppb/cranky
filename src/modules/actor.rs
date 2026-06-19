use crate::adapters::rendering::TinySkiaCosmicCanvas;
use crate::domain::commands::AppCommand;
use crate::domain::shared::render::RenderBuffer;
use crate::domain::{
    MonitorId,
    shared::geometry::{Rect, Scale, Size},
};
use crate::ports::registry::{AnyModulePort, ModuleContext};
use std::collections::HashMap;

pub struct ModuleActor {
    port: Box<dyn AnyModulePort>,
    ctx: ModuleContext,
    sizes: HashMap<MonitorId, Size>,
}

impl ModuleActor {
    pub fn new(port: Box<dyn AnyModulePort>, ctx: ModuleContext) -> Self {
        Self {
            port,
            ctx,
            sizes: HashMap::new(),
        }
    }

    pub fn spawn(mut self) {
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            let mut font_system = cosmic_text::FontSystem::new();
            let mut swash_cache = cosmic_text::SwashCache::new();

            let mut time_rx = self.ctx.hub().time_rx();
            let mut hypr_rx = self.ctx.hub().hyprland_rx();
            let mut applets_rx = self.ctx.hub().applets_rx();
            let mut metrics_rx = self.ctx.hub().metrics_rx();

            // Initial refresh
            self.port.refresh(self.ctx.hub());
            self.measure_and_render_all(&mut font_system, &mut swash_cache);

            loop {
                // Determine what woke us up
                let mut changed = false;

                let should_continue = rt.block_on(async {
                    let ctx_id = self.ctx.id();
                    let (layout_rx, input_rx) = self.ctx.rxs_mut();
                    tokio::select! {
                        Ok(_) = time_rx.changed() => changed = true,
                        Ok(_) = hypr_rx.changed() => changed = true,
                        Ok(_) = applets_rx.changed() => changed = true,
                        Ok(_) = metrics_rx.changed() => changed = true,
                        res = layout_rx.changed() => {
                            if res.is_err() {
                                return false; // layout_rx dropped, we should exit
                            }
                        }
                        Ok((target_id, event)) = input_rx.recv() => {
                            if target_id == ctx_id {
                                let cmds = self.port.on_pointer_event(event);
                                for cmd in cmds {
                                    let _ = self.ctx.command_tx().try_send(cmd);
                                }
                                changed = true;
                            }
                        }
                    }

                    // Debounce rapid changes (e.g. layout bounds updates following size changes)
                    tokio::time::sleep(std::time::Duration::from_millis(16)).await;

                    if time_rx.has_changed().unwrap_or(false) {
                        let _ = time_rx.changed().await;
                        changed = true;
                    }
                    if hypr_rx.has_changed().unwrap_or(false) {
                        let _ = hypr_rx.changed().await;
                        changed = true;
                    }
                    if applets_rx.has_changed().unwrap_or(false) {
                        let _ = applets_rx.changed().await;
                        changed = true;
                    }
                    if metrics_rx.has_changed().unwrap_or(false) {
                        let _ = metrics_rx.changed().await;
                        changed = true;
                    }
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

                self.measure_and_render_all(&mut font_system, &mut swash_cache);
            }
        });
    }

    #[tracing::instrument(level = "debug", skip(self, font_system, swash_cache), fields(module = %self.ctx.id()))]
    fn measure_and_render_all(
        &mut self,
        font_system: &mut cosmic_text::FontSystem,
        swash_cache: &mut cosmic_text::SwashCache,
    ) {
        let t0 = std::time::Instant::now();
        let monitors: Vec<MonitorId> = self
            .ctx
            .hub()
            .hyprland_rx()
            .borrow()
            .monitors()
            .iter()
            .map(|m| MonitorId::new(m.name().as_str()))
            .collect();
        let layouts: std::collections::HashMap<MonitorId, Rect> =
            self.ctx.rxs_mut().0.borrow().clone();

        for monitor_id in monitors {
            // Measure
            let mut dummy_data = vec![0u8; 4];
            let dummy_pixmap = tiny_skia::PixmapMut::from_bytes(&mut dummy_data, 1, 1).unwrap();
            let config = self.ctx.hub().config_rx().borrow().clone();
            let default_font_family = config.bar().font_family().clone();
            let default_font_size = config.bar().font_size();

            let mut dummy_canvas = TinySkiaCosmicCanvas::new(
                dummy_pixmap,
                font_system,
                swash_cache,
                Scale::new(1.0),
                default_font_family.clone(),
                default_font_size,
            );

            let size = self.port.measure(&mut dummy_canvas, &monitor_id);

            let old_size = self
                .sizes
                .get(&monitor_id)
                .copied()
                .unwrap_or(Size::new(0, 0));
            if size != old_size {
                self.sizes.insert(monitor_id.clone(), size);
                let _ = self
                    .ctx
                    .command_tx()
                    .blocking_send(AppCommand::ModuleSizeChanged(
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
                if let Some(pixmap) = tiny_skia::PixmapMut::from_bytes(&mut data, w, h) {
                    let config = self.ctx.hub().config_rx().borrow().clone();
                    let default_font_family = config.bar().font_family().clone();
                    let default_font_size = config.bar().font_size();

                    let mut canvas = TinySkiaCosmicCanvas::new(
                        pixmap,
                        font_system,
                        swash_cache,
                        Scale::new(1.0),
                        default_font_family,
                        default_font_size,
                    );
                    self.port.view(&mut canvas, &monitor_id);

                    // Send buffer to surface manager
                    let buffer = RenderBuffer::new(data, *bounds.size());
                    let rt = tokio::runtime::Handle::current();
                    let sm = self.ctx.surface_manager().clone();
                    let mod_id = self.ctx.id();
                    let mon_id = monitor_id.clone();
                    rt.block_on(async move {
                        sm.submit_buffer(mod_id, mon_id, buffer).await;
                    });
                }
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
