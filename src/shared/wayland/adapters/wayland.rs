use crate::shared::wayland::ports::DisplayServerPort;

use crate::shared::events::signals::SignalHub;
use crate::shared::wayland::ports::DisplayServerError;

use crate::shared::rendering::adapters::tiny_skia::TinySkiaCosmicCanvas;
use crate::shared::wayland::adapters::shm::ShmBuffer;
use crate::shared::primitives::geometry::{LogicalPx, Scale};
use crate::shared::rendering::ports::canvas::Canvas;
use tiny_skia::PixmapMut;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use tokio::io::unix::AsyncFd;
use tracing::{info, debug, info_span};

use cosmic_text::{FontSystem, SwashCache};
use std::collections::HashMap;
use std::os::unix::io::{AsFd, AsRawFd, RawFd};
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle,
    backend::WaylandError,
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_output::{self, WlOutput},
        wl_pointer::{self, WlPointer},
        wl_registry::{self, WlRegistry},
        wl_seat::{self, WlSeat},
        wl_shm::WlShm,
        wl_shm_pool::WlShmPool,
        wl_subcompositor::WlSubcompositor,
        wl_subsurface::WlSubsurface,
        wl_surface::WlSurface,
    },
};
use wayland_protocols::xdg::shell::client::{
    xdg_popup::XdgPopup, xdg_positioner::XdgPositioner, xdg_surface::XdgSurface,
    xdg_wm_base::XdgWmBase,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{ZwlrLayerShellV1, Layer}, 
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1, Anchor},
};

use crate::shared::wayland::ports::SurfaceManagerPort;

struct WaylandFd(RawFd);
impl AsRawFd for WaylandFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

pub struct SurfaceCommand {
    pub module_id: crate::shared::primitives::ModuleId,
    pub monitor_id: crate::shared::primitives::MonitorId,
    pub position: crate::shared::primitives::geometry::Position,
    pub buffer: crate::shared::primitives::render::RenderBuffer,
}

#[derive(Clone)]
pub struct WaylandSurfaceManager {
    pending_surfaces: Arc<Mutex<HashMap<(crate::shared::primitives::ModuleId, crate::shared::primitives::MonitorId), SurfaceCommand>>>,
    notify_tx: tokio::sync::mpsc::Sender<()>,
}

#[async_trait]
impl SurfaceManagerPort for WaylandSurfaceManager {
    async fn submit_buffer(
        &self,
        module_id: crate::shared::primitives::ModuleId,
        monitor_id: crate::shared::primitives::MonitorId,
        position: crate::shared::primitives::geometry::Position,
        buffer: crate::shared::primitives::render::RenderBuffer,
    ) {
        {
            let mut map = self.pending_surfaces.lock().unwrap();
            map.insert(
                (module_id, monitor_id.clone()),
                SurfaceCommand {
                    module_id,
                    monitor_id,
                    position,
                    buffer,
                },
            );
        }
        let _ = self.notify_tx.try_send(());
    }
}

pub struct WaylandAdapter {
    connection: Connection,
    event_queue: EventQueue<WaylandState>,
    state: WaylandState,
    async_fd: AsyncFd<WaylandFd>,
    pending_surfaces: Arc<Mutex<HashMap<(crate::shared::primitives::ModuleId, crate::shared::primitives::MonitorId), SurfaceCommand>>>,
    notify_rx: tokio::sync::mpsc::Receiver<()>,
    config_rx: tokio::sync::watch::Receiver<crate::shared::config::domain::Config>,
}

pub struct WaylandState {
    hub: Arc<SignalHub>,
    compositor: Option<WlCompositor>,
    subcompositor: Option<WlSubcompositor>,
    layer_shell: Option<ZwlrLayerShellV1>,
    xdg_wm_base: Option<XdgWmBase>,
    shm: Option<WlShm>,
    outputs: Vec<WaylandOutputInfo>,
    bars: Vec<WaylandBar>,
    seat: Option<WlSeat>,
    pointer: Option<WlPointer>,

    command_tx: tokio::sync::mpsc::Sender<crate::app::commands::AppCommand>,

    surface_to_id: HashMap<WlSurface, (crate::shared::primitives::ModuleId, crate::shared::primitives::MonitorId)>,
    pointer_surface: Option<WlSurface>,
    pointer_pos: (f64, f64),

    font_system: FontSystem,
    swash_cache: SwashCache,
    tooltip: Option<TooltipSurface>,
}

struct TooltipSurface {
    surface: WlSurface,
    xdg_surface: XdgSurface,
    xdg_popup: XdgPopup,
    shm_buffer: ShmBuffer,
    size: crate::shared::primitives::geometry::Size,
    layout: crate::features::layout_engine::domain::LayoutNode,
    reposition_token: u32,
}

struct WaylandOutputInfo {
    global_id: u32,
    output: WlOutput,
    name: String,
    scale: i32,
}

struct WaylandBar {
    output_name: String,
    surface: WlSurface,
    layer_surface: ZwlrLayerSurfaceV1,
    shm_buffer: ShmBuffer,
    width: u32,
    height: u32,
    config_height: u32,
    config_margin: crate::shared::config::domain::MarginConfig,
    scale: i32,
    module_surfaces: HashMap<crate::shared::primitives::ModuleId, ModuleSurface>,
    configured: bool,
}

struct ModuleSurface {
    surface: WlSurface,
    subsurface: WlSubsurface,
    shm_buffer: ShmBuffer,
    size: crate::shared::primitives::geometry::Size,
    x: i32,
    y: i32,
}

impl Drop for ModuleSurface {
    fn drop(&mut self) {
        self.subsurface.destroy();
        self.surface.destroy();
    }
}

impl Drop for WaylandBar {
    fn drop(&mut self) {
        self.module_surfaces.clear();
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}

impl WaylandAdapter {
    pub fn new(
        hub: Arc<SignalHub>,
        command_tx: tokio::sync::mpsc::Sender<crate::app::commands::AppCommand>,
    ) -> Result<(Self, WaylandSurfaceManager), DisplayServerError> {
        let connection =
            Connection::connect_to_env().map_err(|e| DisplayServerError::ConnectionFailed {
                reason: e.to_string(),
            })?;
        let event_queue = connection.new_event_queue();
        let qh = event_queue.handle();

        connection.display().get_registry(&qh, ());

        let raw_fd = connection.as_fd().as_raw_fd();
        let async_fd =
            AsyncFd::new(WaylandFd(raw_fd)).map_err(|e| DisplayServerError::ConnectionFailed {
                reason: e.to_string(),
            })?;

        let config_rx = hub.config_rx();
        let state = WaylandState {
            hub,
            compositor: None,
            shm: None,
            layer_shell: None,
            subcompositor: None,
            xdg_wm_base: None,
            outputs: Vec::new(),
            bars: Vec::new(),
            seat: None,
            pointer: None,
            command_tx,
            surface_to_id: HashMap::new(),
            pointer_surface: None,
            pointer_pos: (0.0, 0.0),
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            tooltip: None,
        };

        let pending_surfaces = Arc::new(Mutex::new(HashMap::new()));
        let (notify_tx, notify_rx) = tokio::sync::mpsc::channel(1);

        let adapter = Self {
            connection,
            event_queue,
            state,
            async_fd,
            pending_surfaces: pending_surfaces.clone(),
            notify_rx,
            config_rx,
        };

        let manager = WaylandSurfaceManager {
            pending_surfaces,
            notify_tx,
        };

        Ok((adapter, manager))
    }
}

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
                    // Drop read guard to process surface commands, then we will loop again in App
                    drop(r_guard);
                    let cmds: Vec<SurfaceCommand> = {
                        let mut map = self.pending_surfaces.lock().unwrap();
                        map.drain().map(|(_, cmd)| cmd).collect()
                    };
                    for cmd in cmds {
                        self.handle_surface_command(cmd)?;
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
            tracing::info!("WaylandAdapter detected config change, recreating bars...");
            self.state.bars.clear(); // Drop existing bars
            self.state.surface_to_id.clear();

            // Recreate bars for all currently known outputs
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

        // Ensure all known outputs have bars created once their names are known
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
        read_model: &crate::app::state::AppReadModel,
        layout_senders: &std::collections::HashMap<
            crate::shared::primitives::ModuleId,
            Box<dyn crate::features::module_runtime::ports::LayoutSender>,
        >,
    ) -> Result<(), DisplayServerError> {
        let qh = self.event_queue.handle();
        self.render_all_outputs(read_model, layout_senders, &qh)
    }

    fn show_tooltip(&mut self, layout: crate::features::layout_engine::domain::LayoutNode) -> Result<(), DisplayServerError> {
        tracing::debug!("Requested show_tooltip");
        if let Some(tooltip) = &self.state.tooltip
            && tooltip.layout == layout
        {
            tracing::trace!("Tooltip layout hasn't changed, skipping update");
            return Ok(());
        }

        let state = &mut self.state;

        let Some(parent_surface) = state.pointer_surface.clone() else {
            tracing::debug!("show_tooltip skipped: state.pointer_surface is None");
            return Ok(());
        };

        let Some(compositor) = state.compositor.as_ref() else {
            tracing::debug!("show_tooltip skipped: state.compositor is None");
            return Ok(());
        };
        let Some(_layer_shell) = state.layer_shell.as_ref() else {
            tracing::debug!("show_tooltip skipped: state.layer_shell is None");
            return Ok(());
        };
        let Some(shm) = state.shm.as_ref() else {
            tracing::debug!("show_tooltip skipped: state.shm is None");
            return Ok(());
        };

        let Some(xdg_wm_base) = state.xdg_wm_base.as_ref() else {
            tracing::debug!("show_tooltip skipped: state.xdg_wm_base is None");
            return Ok(());
        };

        let mut bar_scale = 1;
        let mut output_name = String::new();
        let mut pointer_x = state.pointer_pos.0;
        let mut _pointer_y = state.pointer_pos.1;
        let mut bar_height = 0;
        let mut _bar_margin_left: i32 = 0;
        let mut _bar_margin_top: i32 = 0;
        let mut bar_layer_surface = None;

        for bar in &state.bars {
            if bar.surface == parent_surface {
                bar_scale = bar.scale;
                output_name = bar.output_name.clone();
                bar_height = bar.height;
                _bar_margin_left = bar.config_margin.left().value();
                _bar_margin_top = bar.config_margin.top().value();
                bar_layer_surface = Some(bar.layer_surface.clone());
                break;
            }
            if let Some(ms) = bar
                .module_surfaces
                .values()
                .find(|m| m.surface == parent_surface)
            {
                bar_scale = bar.scale;
                output_name = bar.output_name.clone();
                bar_height = bar.height;
                _bar_margin_left = bar.config_margin.left().value();
                _bar_margin_top = bar.config_margin.top().value();
                bar_layer_surface = Some(bar.layer_surface.clone());
                // pointer_pos is relative to the module, add module offset
                pointer_x += ms.x as f64;
                _pointer_y += ms.y as f64;
                break;
            }
        }

        if output_name.is_empty() {
            tracing::debug!("show_tooltip skipped: output_name is empty (parent_surface not found in bars)");
            return Ok(());
        }

        let Some(bar_layer_surface) = bar_layer_surface else {
            tracing::debug!("show_tooltip skipped: bar_layer_surface is None");
            return Ok(());
        };

        let _output = state
            .outputs
            .iter()
            .find(|o| o.name == output_name)
            .map(|o| &o.output);

        let font_family = crate::shared::config::domain::FontFamily::new("Inter".to_string());
        let font_size = crate::shared::config::domain::FontSize::new(12.0);
        let scale = Scale::new(bar_scale as f32);

        let (text_w, text_h, render_node) = {
            let mut measurer = crate::shared::rendering::adapters::tiny_skia::CosmicTextMeasurer::new(
                &mut state.font_system,
                scale,
                font_family.clone(),
                font_size,
            );
            
            use crate::features::layout_engine::ports::LayoutEnginePort;
            let mut engine = crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter::new();
            if let Ok(render_node) = engine.calculate_layout(layout.clone(), &mut measurer, crate::shared::primitives::geometry::Position::new(0, 0)) {
                let rect = render_node.rect();
                (rect.width() as i32, rect.height() as i32, render_node)
            } else {
                (1, 1, crate::features::layout_engine::domain::RenderNode::Rect {
                    rect: crate::shared::primitives::geometry::Rect::new(crate::shared::primitives::geometry::Position::new(0, 0), crate::shared::primitives::geometry::Size::new(1, 1)),
                    color: crate::shared::primitives::color::DrawingColor::Solid(crate::shared::primitives::color::Color::new(0, 0, 0, 255)),
                    radius: None,
                    on_click: None,
                    on_hover: None,
                    tooltip: None,
                })
            }
        };

        let width = ((text_w as f32) * scale.value()).ceil() as u32;
        let height = ((text_h as f32) * scale.value()).ceil() as u32;

        let width = width.max(1);
        let height = height.max(1);
        let new_size = crate::shared::primitives::geometry::Size::new(width, height);

        if let Some(tooltip) = &mut state.tooltip {
            if tooltip.size == new_size {
                let data = tooltip.shm_buffer.mmap_mut();
                if let Some(pixmap) = tiny_skia::PixmapMut::from_bytes(data, width, height) {
                    let mut actual_canvas = TinySkiaCosmicCanvas::new(
                        pixmap,
                        &mut state.font_system,
                        &mut state.swash_cache,
                        scale,
                        font_family.clone(),
                        font_size,
                    );
                    render_node.render_to_canvas(&mut actual_canvas);
                }
                tooltip.layout = layout;
                tooltip.surface.attach(Some(tooltip.shm_buffer.current_buffer()), 0, 0);
                tooltip.surface.damage_buffer(0, 0, width as i32, height as i32);
                tooltip.surface.commit();
                tracing::debug!(size = ?new_size, "Redrew tooltip in-place (same Size VO)");
                return Ok(());
            } else {
                let qh = self.event_queue.handle();
                let mut new_shm_buffer = ShmBuffer::new(shm, width, height, &qh)
                    .map_err(|e| DisplayServerError::Internal(e.to_string()))?;
                let data = new_shm_buffer.mmap_mut();
                if let Some(pixmap) = tiny_skia::PixmapMut::from_bytes(data, width, height) {
                    let mut actual_canvas = TinySkiaCosmicCanvas::new(
                        pixmap,
                        &mut state.font_system,
                        &mut state.swash_cache,
                        scale,
                        font_family.clone(),
                        font_size,
                    );
                    render_node.render_to_canvas(&mut actual_canvas);
                }
                let positioner = xdg_wm_base.create_positioner(&qh, ());
                positioner.set_size(width as i32, height as i32);
                positioner.set_anchor_rect(pointer_x as i32, bar_height as i32, 1, 1);
                positioner.set_anchor(wayland_protocols::xdg::shell::client::xdg_positioner::Anchor::Bottom);
                positioner.set_gravity(wayland_protocols::xdg::shell::client::xdg_positioner::Gravity::Bottom);
                positioner.set_constraint_adjustment(
                    wayland_protocols::xdg::shell::client::xdg_positioner::ConstraintAdjustment::SlideX |
                    wayland_protocols::xdg::shell::client::xdg_positioner::ConstraintAdjustment::SlideY |
                    wayland_protocols::xdg::shell::client::xdg_positioner::ConstraintAdjustment::FlipY
                );
                tooltip.reposition_token = tooltip.reposition_token.wrapping_add(1);
                tooltip.xdg_popup.reposition(&positioner, tooltip.reposition_token);
                positioner.destroy();
                tooltip.shm_buffer = new_shm_buffer;
                tooltip.size = new_size;
                tooltip.layout = layout;
                tooltip.surface.attach(Some(tooltip.shm_buffer.current_buffer()), 0, 0);
                tooltip.surface.damage_buffer(0, 0, width as i32, height as i32);
                tooltip.surface.commit();
                tracing::debug!(size = ?new_size, token = tooltip.reposition_token, "Redrew and repositioned tooltip in-place (new Size VO)");
                return Ok(());
            }
        }

        let qh = self.event_queue.handle();
        let mut shm_buffer = ShmBuffer::new(shm, width, height, &qh)
            .map_err(|e| DisplayServerError::Internal(e.to_string()))?;

        {
            let data = shm_buffer.mmap_mut();
            if let Some(pixmap) = tiny_skia::PixmapMut::from_bytes(data, width, height) {
                let mut actual_canvas = TinySkiaCosmicCanvas::new(
                    pixmap,
                    &mut state.font_system,
                    &mut state.swash_cache,
                    scale,
                    font_family.clone(),
                    font_size,
                );
                
                render_node.render_to_canvas(&mut actual_canvas);
            }
        }

        let positioner = xdg_wm_base.create_positioner(&qh, ());
        positioner.set_size(width as i32, height as i32);
        positioner.set_anchor_rect(pointer_x as i32, bar_height as i32, 1, 1);
        positioner
            .set_anchor(wayland_protocols::xdg::shell::client::xdg_positioner::Anchor::Bottom);
        positioner
            .set_gravity(wayland_protocols::xdg::shell::client::xdg_positioner::Gravity::Bottom);
        positioner.set_constraint_adjustment(
            wayland_protocols::xdg::shell::client::xdg_positioner::ConstraintAdjustment::SlideX |
            wayland_protocols::xdg::shell::client::xdg_positioner::ConstraintAdjustment::SlideY |
            wayland_protocols::xdg::shell::client::xdg_positioner::ConstraintAdjustment::FlipY
        );

        let surface = compositor.create_surface(&qh, ());
        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &qh, ());
        let xdg_popup = xdg_surface.get_popup(None, &positioner, &qh, ());
        tracing::debug!(width, height, pointer_x, bar_height, "Creating tooltip surface");

        bar_layer_surface.get_popup(&xdg_popup);

        positioner.destroy();
        surface.commit();

        state.tooltip = Some(TooltipSurface {
            surface,
            xdg_surface,
            xdg_popup,
            shm_buffer,
            size: new_size,
            layout,
            reposition_token: 0,
        });

        Ok(())
    }

    fn hide_tooltip(&mut self) -> Result<(), DisplayServerError> {
        let state = &mut self.state;
        if let Some(tooltip) = state.tooltip.take() {
            tracing::debug!("Hiding tooltip");
            tooltip.xdg_popup.destroy();
            tooltip.xdg_surface.destroy();
            tooltip.surface.destroy();
        }
        Ok(())
    }
}

impl WaylandAdapter {
        fn handle_surface_command(&mut self, cmd: SurfaceCommand) -> Result<(), DisplayServerError> {
        let qh = self.event_queue.handle();

        let Some(compositor) = self.state.compositor.as_ref() else {
            return Ok(());
        };
        let Some(subcompositor) = self.state.subcompositor.as_ref() else {
            return Ok(());
        };
        let Some(shm) = self.state.shm.as_ref() else {
            return Ok(());
        };

        let bar = match self
            .state
            .bars
            .iter_mut()
            .find(|b| b.output_name == cmd.monitor_id.as_str())
        {
            Some(b) => b,
            None => return Ok(()),
        };

        let width = cmd.buffer.width().max(1);
        let height = cmd.buffer.height().max(1);

        let mut new_surface_to_register = None;
        let ms = bar.module_surfaces.entry(cmd.module_id).or_insert_with(|| {
            let surface = compositor.create_surface(&qh, ());
            let subsurface = subcompositor.get_subsurface(&surface, &bar.surface, &qh, ());
            subsurface.set_desync();

            let shm_buffer =
                ShmBuffer::new(shm, width, height, &qh).expect("Failed to create SHM buffer");

            new_surface_to_register = Some(surface.clone());

            ModuleSurface {
                surface,
                subsurface,
                shm_buffer,
                size: *cmd.buffer.size(),
                x: 0,
                y: 0,
            }
        });

        if let Some(surface) = new_surface_to_register {
            self.state
                .surface_to_id
                .insert(surface, (cmd.module_id, cmd.monitor_id.clone()));
        }

        if ms.size != *cmd.buffer.size() {
            ms.shm_buffer = ShmBuffer::new(shm, width, height, &qh)
                .expect("Failed to recreate SHM buffer for resize");
            ms.size = *cmd.buffer.size();
        }
        
        if ms.x != cmd.position.x() || ms.y != cmd.position.y() {
            ms.subsurface.set_position(cmd.position.x(), cmd.position.y());
            ms.x = cmd.position.x();
            ms.y = cmd.position.y();
        }

        let data = ms.shm_buffer.mmap_mut();
        let src_data = cmd.buffer.data();
        let len = std::cmp::min(data.len(), src_data.len());
        data[..len].copy_from_slice(&src_data[..len]);

        ms.surface
            .attach(Some(ms.shm_buffer.current_buffer()), 0, 0);
        ms.surface.damage_buffer(0, 0, width as i32, height as i32);
        ms.surface.commit();
        
        // Always commit the parent surface to ensure the compositor applies the subsurface 
        // update, working around bugs in some compositors (like Hyprland/wlroots) where 
        // subsurface desync mode is ignored for layer shell surfaces.
        bar.surface.commit();

        ms.shm_buffer.swap_buffers();
        
        self.state
            .surface_to_id
            .insert(ms.surface.clone(), (cmd.module_id, cmd.monitor_id.clone()));

        Ok(())
    }

        fn render_all_outputs(
        &mut self,
        read_model: &crate::app::state::AppReadModel,
        layout_senders: &std::collections::HashMap<
            crate::shared::primitives::ModuleId,
            Box<dyn crate::features::module_runtime::ports::LayoutSender>,
        >,
        qh: &QueueHandle<WaylandState>,
    ) -> Result<(), DisplayServerError> {
        let span = info_span!("render_all_outputs");
        let _enter = span.enter();

        let WaylandState {
            ref mut bars,
            ref mut font_system,
            ref mut swash_cache,
            ref compositor,
            ref subcompositor,
            ref shm,
            ref mut surface_to_id,
            ..
        } = self.state;

        if bars.is_empty() {
            debug!("No bars available for rendering.");
            return Ok(());
        }

        let Some(compositor) = compositor.as_ref() else {
            tracing::error!("Compositor not bound");
            return Ok(());
        };
        let Some(subcompositor) = subcompositor.as_ref() else {
            tracing::error!("Subcompositor not bound");
            return Ok(());
        };
        let Some(shm) = shm.as_ref() else {
            tracing::error!("SHM not bound");
            return Ok(());
        };

        let mut all_layouts_by_module: std::collections::HashMap<
            crate::shared::primitives::ModuleId,
            std::collections::HashMap<crate::shared::primitives::MonitorId, crate::shared::primitives::geometry::Rect>,
        > = std::collections::HashMap::new();

        for bar in bars {
            if !bar.configured {
                debug!("Skipping render for unconfigured bar: {}", bar.output_name);
                continue;
            }
            debug!(
                "Rendering bar for output: {} (size: {}x{}, scale: {})",
                bar.output_name, bar.width, bar.height, bar.scale
            );
            let (width, height, scale) = (bar.width, bar.height, bar.scale);
            let physical_width = width * scale as u32;
            let physical_height = height * scale as u32;

            let pixmap_data = bar.shm_buffer.mmap_mut();
            let Some(mut pixmap) = PixmapMut::from_bytes(pixmap_data, physical_width, physical_height)
            else {
                tracing::error!("Failed to create pixmap for bar {}", bar.output_name);
                continue;
            };

            let hypr_rx = self.state.hub.hyprland_rx();
            let hyprland_state = hypr_rx.borrow();
            let is_focused = hyprland_state
                .focused_monitor()
                .is_some_and(|name| name.as_str() == bar.output_name);


            let mut bar_config = read_model.config().bar().clone();
            if !is_focused {
                bar_config = bar_config.as_unfocused();
            }

            let config_bg = bar_config.background().clone();
            let border_config = bar_config.border();

            // Check if hot-reload of height or margin is needed
            if bar.config_height != bar_config.height().value() || bar.config_margin != *bar_config.margin()
            {
                debug!(
                    "Hot-reloading bar height/margin for output: {}",
                    bar.output_name
                );
                bar.config_height = bar_config.height().value();
                bar.config_margin = bar_config.margin().clone();

                let margin = bar_config.margin();
                bar.layer_surface.set_size(0, bar.config_height);
                bar.layer_surface.set_margin(
                    margin.top().value(),
                    margin.right().value(),
                    margin.bottom().value(),
                    margin.left().value(),
                );
                bar.layer_surface.set_exclusive_zone(
                    bar.config_height as i32 + margin.top().value() + margin.bottom().value(),
                );
                bar.surface.commit();

                // Note: The actual resize will happen asynchronously via the Configure event.
                // We proceed with the current size for this frame to avoid visual glitches.
            }

            pixmap.fill(tiny_skia::Color::TRANSPARENT);
            let mut bar_canvas = TinySkiaCosmicCanvas::new(
                pixmap,
                font_system,
                swash_cache,
                Scale::new(scale as f32),
                bar_config.font_family().clone(),
                bar_config.font_size(),
            );
            let border_size = border_config.size().value();
            let half_border = border_size / 2.0;

            bar_canvas.draw_rect(
                LogicalPx::new(0.0),
                LogicalPx::new(0.0),
                LogicalPx::new(width as f32),
                LogicalPx::new(height as f32),
                config_bg,
                LogicalPx::new(border_config.radius().value()),
            );
            bar_canvas.draw_border(crate::shared::primitives::geometry::Position::new(half_border as i32, half_border as i32), crate::shared::primitives::geometry::Size::new((width as f32 - border_size) as u32, (height as f32 - border_size) as u32), border_config.color().clone(), LogicalPx::new(border_config.radius().value()), LogicalPx::new(border_size));

            // Calculate layout
            let monitor_id = crate::shared::primitives::MonitorId::new(&bar.output_name);
            // Note: We no longer iterate over layouts to render modules directly.
            // Layout is broadcasted via app.calculate_layout, and module actors render themselves
            // asynchronously and submit their buffers to the Wayland adapter via the SurfaceManager.
            let layouts = read_model.calculate_layout(
                &monitor_id,
                crate::shared::primitives::geometry::BarWidth::new(width),
                &bar_config,
            );

            // Collect layout bounds to modules for this monitor
            for layout in &layouts {
                all_layouts_by_module
                    .entry(layout.id())
                    .or_default()
                    .insert(monitor_id.clone(), *layout.bounds());
            }


            // However, the display server must still position the subsurfaces correctly on the screen!
            for layout in layouts {
                let module_id = layout.id();
                let bounds = layout.bounds();

                let ms = match bar.module_surfaces.entry(module_id) {
                    std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
                    std::collections::hash_map::Entry::Vacant(v) => {
                        let surface = compositor.create_surface(qh, ());
                        let subsurface =
                            subcompositor.get_subsurface(&surface, &bar.surface, qh, ());
                        subsurface.set_desync();

                        let width = bounds.width().max(1);
                        let height = bounds.height().max(1);
                        let shm_buffer =
                            match crate::shared::wayland::adapters::shm::ShmBuffer::new(shm, width, height, qh) {
                                Ok(b) => b,
                                Err(e) => {
                                    tracing::error!("Failed to create shm buffer: {}", e);
                                    continue;
                                }
                            };

                        v.insert(ModuleSurface {
                            surface,
                            subsurface,
                            shm_buffer,
                            size: *bounds.size(),
                            x: bounds.x(),
                            y: bounds.y(),
                        })
                    }
                };

                surface_to_id.insert(ms.surface.clone(), (module_id, monitor_id.clone()));

                ms.x = bounds.x();
                ms.y = bounds.y();
                ms.subsurface.set_position(bounds.x(), bounds.y());
            }

            let buffer = bar.shm_buffer.current_buffer();

            bar.surface.set_buffer_scale(scale);
            bar.surface.attach(Some(buffer), 0, 0);
            bar.surface.damage(0, 0, width as i32, height as i32);
            bar.surface.commit();
            bar.shm_buffer.swap_buffers();
        }

        // Broadcast layout bounds to modules for ALL active monitors
        for (id, sender) in layout_senders {
            if let Some(rects) = all_layouts_by_module.get(id) {
                sender.send_layout(rects.clone());
            } else {
                sender.send_layout(std::collections::HashMap::new());
            }
        }

        let _ = self.connection.flush();
        Ok(())
    }

}

impl WaylandState {
        fn create_bar(
        &mut self,
        output: &WlOutput,
        qh: &QueueHandle<Self>,
    ) -> Result<(), DisplayServerError> {
        let (output_name, output_scale) = {
            let info = self
                .outputs
                .iter()
                .find(|i| &i.output == output)
                .ok_or_else(|| DisplayServerError::ConnectionFailed {
                    reason: "Output not found".to_string(),
                })?;
            if info.name.is_empty() {
                return Ok(());
            }
            (info.name.clone(), info.scale)
        };

        if self.bars.iter().any(|b| b.output_name == output_name) {
            return Ok(());
        }

        let bar_config = self.hub.config_rx().borrow().bar().clone();
        let bar_height = bar_config.height();
        let margin = bar_config.margin();
        info!(
            "Creating bar for output: {} (height: {}, scale: {})",
            output_name, bar_height.value(), output_scale
        );

        let compositor = self
            .compositor
            .as_ref()
            .ok_or(DisplayServerError::ConnectionFailed {
                reason: "Compositor not bound".to_string(),
            })?;
        let layer_shell =
            self.layer_shell
                .as_ref()
                .ok_or(DisplayServerError::ConnectionFailed {
                    reason: "Layer shell not bound".to_string(),
                })?;
        let shm = self
            .shm
            .as_ref()
            .ok_or(DisplayServerError::ConnectionFailed {
                reason: "SHM not bound".to_string(),
            })?;

        let surface = compositor.create_surface(qh, ());
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            Some(output),
            Layer::Top,
            "cranky".to_string(),
            qh,
            (),
        );

        layer_surface.set_anchor(Anchor::Top | Anchor::Left | Anchor::Right);
        layer_surface.set_size(0, bar_height.value());
        layer_surface.set_margin(
            margin.top().value(),
            margin.right().value(),
            margin.bottom().value(),
            margin.left().value(),
        );
        layer_surface
            .set_exclusive_zone(bar_height.value() as i32 + margin.top().value() + margin.bottom().value());
        surface.set_buffer_scale(output_scale);
        surface.commit();

        let shm_buffer = ShmBuffer::new(
            shm,
            1920 * output_scale as u32,
            bar_height.value() * output_scale as u32,
            qh,
        )
        .map_err(DisplayServerError::Io)?;

        self.bars.push(WaylandBar {
            output_name,
            surface,
            layer_surface,
            shm_buffer,
            width: 1920,
            height: bar_height.value(),
            config_height: bar_height.value(),
            config_margin: margin.clone(),
            scale: output_scale,
            module_surfaces: HashMap::new(),
            configured: false,
        });

        Ok(())
    }

}

impl Dispatch<WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                match interface.as_str() {
                    "wl_compositor" => state.compositor = Some(proxy.bind(name, version, qh, ())),
                    "wl_shm" => state.shm = Some(proxy.bind(name, version, qh, ())),
                    "zwlr_layer_shell_v1" => {
                        state.layer_shell = Some(proxy.bind(name, version, qh, ()))
                    }
                    "xdg_wm_base" => state.xdg_wm_base = Some(proxy.bind(name, u32::min(version, 5), qh, ())),
                    "wl_subcompositor" => state.subcompositor = Some(proxy.bind(name, version, qh, ())),
                    "wl_output" => {
                        let output: WlOutput = proxy.bind(name, version, qh, ());
                        state.outputs.push(WaylandOutputInfo {
                            global_id: name,
                            output,
                            name: String::new(),
                            scale: 1,
                        });
                    }
                    "wl_seat" => state.seat = Some(proxy.bind(name, version, qh, ())),
                    _ => {}
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(pos) = state.outputs.iter().position(|o| o.global_id == name) {
                    let info = state.outputs.remove(pos);
                    if let Some(bar_pos) = state.bars.iter().position(|b| b.output_name == info.name) {
                        let mut bar = state.bars.remove(bar_pos);
                        for (_, ms) in bar.module_surfaces.drain() {
                            ms.subsurface.destroy();
                            ms.surface.destroy();
                        }
                        bar.layer_surface.destroy();
                        bar.surface.destroy();
                    }
                    info.output.release();
                    
                    let tx = state.command_tx.clone();
                    tokio::spawn(async move {
                        let _ = tx.send(crate::app::commands::AppCommand::RequestRender).await;
                    });
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlCompositor, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlCompositor,
        _: wayland_client::protocol::wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<WlShm, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlShm,
        _: wayland_client::protocol::wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<ZwlrLayerShellV1, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &ZwlrLayerShellV1,
        _: wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<WlSubcompositor, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlSubcompositor,
        _: wayland_client::protocol::wl_subcompositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<WlOutput, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &WlOutput,
        event: wl_output::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let Some(info) = state.outputs.iter_mut().find(|i| &i.output == proxy) {
            match event {
                wl_output::Event::Name { name } => info.name = name,
                wl_output::Event::Scale { factor } => info.scale = factor,
                _ => {}
            }
        }
    }
}
impl Dispatch<WlSeat, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            let caps =
                wayland_client::protocol::wl_seat::Capability::from_bits(capabilities.into())
                    .unwrap_or(wayland_client::protocol::wl_seat::Capability::empty());
            if caps.contains(wayland_client::protocol::wl_seat::Capability::Pointer)
                && state.pointer.is_none()
            {
                state.pointer = Some(proxy.get_pointer(qh, ()));
            }
        }
    }
}
impl Dispatch<WlPointer, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use crate::shared::events::core::PointerEvent;

        match event {
            wl_pointer::Event::Enter {
                surface,
                surface_x,
                surface_y,
                ..
            } => {
                tracing::debug!(surface_x, surface_y, "wl_pointer Enter");
                state.pointer_surface = Some(surface.clone());
                state.pointer_pos = (surface_x, surface_y);
                if let Some((id, mon_id)) = state.surface_to_id.get(&surface) {
                    tracing::debug!(module = %id, monitor = %mon_id, "Forwarding PointerEnter");
                    let _ = state
                        .hub
                        .pointer_tx()
                        .send((*id, mon_id.clone(), PointerEvent::PointerEnter));
                }
            }
            wl_pointer::Event::Leave { surface: _, .. } => {
                tracing::debug!("wl_pointer Leave");
                if let Some(surface) = state.pointer_surface.take()
                    && let Some((id, mon_id)) = state.surface_to_id.get(&surface)
                {
                    tracing::debug!(module = %id, monitor = %mon_id, "Forwarding PointerLeave");
                    let _ = state
                        .hub
                        .pointer_tx()
                        .send((*id, mon_id.clone(), PointerEvent::PointerLeave));
                }
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                state.pointer_pos = (surface_x, surface_y);
                if let Some(surface) = &state.pointer_surface
                    && let Some((id, mon_id)) = state.surface_to_id.get(surface)
                {
                    let _ = state.hub.pointer_tx().send((
                        *id,
                        mon_id.clone(),
                        PointerEvent::PointerMotion {
                            x: surface_x,
                            y: surface_y,
                        },
                    ));
                }
            }
            wl_pointer::Event::Button {
                button,
                state: button_state,
                ..
            } => {
                tracing::debug!(button, ?button_state, "wl_pointer Button");
                if button_state
                    == wayland_client::WEnum::Value(
                        wayland_client::protocol::wl_pointer::ButtonState::Released,
                    )
                    && let Some(surface) = &state.pointer_surface
                    && let Some((id, mon_id)) = state.surface_to_id.get(surface)
                {
                    tracing::debug!(module = %id, monitor = %mon_id, "Forwarding Click");
                    let _ = state.hub.pointer_tx().send((
                        *id,
                        mon_id.clone(),
                        PointerEvent::Click {
                            button,
                            x: state.pointer_pos.0,
                            y: state.pointer_pos.1,
                        },
                    ));
                }
            }
            wl_pointer::Event::Axis { axis, value, .. } => {
                tracing::debug!(?axis, value, "wl_pointer Axis");
                if let Some(surface) = &state.pointer_surface
                    && let Some((id, mon_id)) = state.surface_to_id.get(surface)
                {
                    let axis_val = match axis {
                        wayland_client::WEnum::Value(
                            wayland_client::protocol::wl_pointer::Axis::VerticalScroll,
                        ) => 0,
                        wayland_client::WEnum::Value(
                            wayland_client::protocol::wl_pointer::Axis::HorizontalScroll,
                        ) => 1,
                        _ => 0,
                    };
                    tracing::debug!(module = %id, monitor = %mon_id, "Forwarding Scroll");
                    let _ = state.hub.pointer_tx().send((
                        *id,
                        mon_id.clone(),
                        PointerEvent::Scroll {
                            axis: axis_val,
                            amount: value,
                        },
                    ));
                }
            }
            _ => {}
        }
    }
}
impl Dispatch<WlSurface, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlSurface,
        _: wayland_client::protocol::wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<ZwlrLayerSurfaceV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let zwlr_layer_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            proxy.ack_configure(serial);
            if let Some(bar) = state.bars.iter_mut().find(|b| &b.layer_surface == proxy) {
                bar.configured = true;
                if width > 0 && height > 0 {
                    let old_width = bar.width;
                    let old_height = bar.height;

                    bar.width = width;
                    bar.height = height;

                    if old_width != width || old_height != height {
                        debug!("Bar resized to {}x{} (scale: {})", width, height, bar.scale);
                        let physical_width = width * bar.scale as u32;
                        let physical_height = height * bar.scale as u32;

                        if let Ok(new_shm) = ShmBuffer::new(
                            state.shm.as_ref().expect("SHM bound"),
                            physical_width,
                            physical_height,
                            qh,
                        ) {
                            bar.shm_buffer = new_shm;
                        }
                    }

                    // Always request a render after a configure event, even if size didn't change
                    let tx = state.command_tx.clone();
                    tokio::spawn(async move {
                        let _ = tx
                            .send(crate::app::commands::AppCommand::RequestRender)
                            .await;
                    });
                }
            }
        }
    }
}
impl Dispatch<WlBuffer, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        _: wayland_client::protocol::wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<XdgWmBase, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        proxy: &XdgWmBase,
        event: wayland_protocols::xdg::shell::client::xdg_wm_base::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wayland_protocols::xdg::shell::client::xdg_wm_base::Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

impl Dispatch<XdgSurface, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &XdgSurface,
        event: wayland_protocols::xdg::shell::client::xdg_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wayland_protocols::xdg::shell::client::xdg_surface::Event::Configure { serial } =
            event
        {
            tracing::debug!("xdg_surface Configure serial={serial}");
            proxy.ack_configure(serial);
            if let Some(tooltip) = &mut state.tooltip
                && &tooltip.xdg_surface == proxy
            {
                tracing::debug!("Attaching buffer to tooltip surface");
                tooltip
                    .surface
                    .attach(Some(tooltip.shm_buffer.current_buffer()), 0, 0);
                tooltip.surface.damage_buffer(
                    0,
                    0,
                    tooltip.size.width() as i32,
                    tooltip.size.height() as i32,
                );
                tooltip.surface.commit();
            } else {
                tracing::debug!("Configure event for unknown xdg_surface");
            }
        }
    }
}

impl Dispatch<XdgPopup, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &XdgPopup,
        event: wayland_protocols::xdg::shell::client::xdg_popup::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wayland_protocols::xdg::shell::client::xdg_popup::Event::Repositioned { token } => {
                tracing::debug!("Tooltip popup repositioned (token={token})");
            }
            other => tracing::debug!("xdg_popup Event: {:?}", other),
        }
    }
}

impl Dispatch<XdgPositioner, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &XdgPositioner,
        _event: wayland_protocols::xdg::shell::client::xdg_positioner::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<WlShmPool, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlShmPool,
        _: wayland_client::protocol::wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<WlSubsurface, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlSubsurface,
        _: wayland_client::protocol::wl_subsurface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}






#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::io::AsRawFd;

    #[test]
    fn test_wayland_fd() {
        let fd = WaylandFd(42);
        assert_eq!(fd.as_raw_fd(), 42);
    }

    #[tokio::test]
    async fn test_wayland_surface_manager() {
        let pending_surfaces = Arc::new(Mutex::new(HashMap::new()));
        let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel(1);
        let manager = WaylandSurfaceManager {
            pending_surfaces: pending_surfaces.clone(),
            notify_tx,
        };

        let module_id = crate::shared::primitives::ModuleId::new(1);
        let monitor_id = crate::shared::primitives::MonitorId::new("DP-1");
        let buffer = crate::shared::primitives::render::RenderBuffer::new(
            vec![0; 400],
            crate::shared::primitives::geometry::Size::new(10, 10),
        );

        manager.submit_buffer(module_id, monitor_id.clone(), crate::shared::primitives::geometry::Position::new(0, 0), buffer).await;

        notify_rx.recv().await.expect("Failed to receive notification");
        let cmd = pending_surfaces.lock().unwrap().remove(&(module_id, monitor_id)).expect("Failed to find command");
        assert_eq!(cmd.module_id, module_id);
        assert_eq!(cmd.monitor_id.as_str(), "DP-1");
        assert_eq!(cmd.buffer.size().width(), 10);
    }

    #[tokio::test]
    async fn test_wayland_surface_manager_coalesces_latest_frame() {
        let pending_surfaces = Arc::new(Mutex::new(HashMap::new()));
        let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel(1);
        let manager = WaylandSurfaceManager {
            pending_surfaces: pending_surfaces.clone(),
            notify_tx,
        };

        let module_id = crate::shared::primitives::ModuleId::new(1);
        let monitor_id = crate::shared::primitives::MonitorId::new("DP-1");
        let buffer1 = crate::shared::primitives::render::RenderBuffer::new(
            vec![0; 400],
            crate::shared::primitives::geometry::Size::new(10, 10),
        );
        let buffer2 = crate::shared::primitives::render::RenderBuffer::new(
            vec![0; 1600],
            crate::shared::primitives::geometry::Size::new(20, 20),
        );

        manager.submit_buffer(module_id, monitor_id.clone(), crate::shared::primitives::geometry::Position::new(0, 0), buffer1).await;
        manager.submit_buffer(module_id, monitor_id.clone(), crate::shared::primitives::geometry::Position::new(0, 0), buffer2).await;

        notify_rx.recv().await.expect("Failed to receive notification");
        let mut map = pending_surfaces.lock().unwrap();
        assert_eq!(map.len(), 1);
        let cmd = map.remove(&(module_id, monitor_id)).expect("Failed to find command");
        assert_eq!(cmd.buffer.size().width(), 20);
    }

    #[test]
    fn test_surface_command_struct() {
        let module_id = crate::shared::primitives::ModuleId::new(2);
        let monitor_id = crate::shared::primitives::MonitorId::new("HDMI-1");
        let buffer = crate::shared::primitives::render::RenderBuffer::new(
            vec![0; 1600],
            crate::shared::primitives::geometry::Size::new(20, 20),
        );
        let cmd = SurfaceCommand {
            module_id,
            monitor_id: monitor_id.clone(),
            position: crate::shared::primitives::geometry::Position::new(0, 0),
            buffer,
        };

        assert_eq!(cmd.module_id, module_id);
        assert_eq!(cmd.monitor_id, monitor_id);
        assert_eq!(cmd.buffer.size().height(), 20);
    }

    #[test]
    #[allow(drop_bounds)]
    fn test_regression_wayland_bar_and_module_surface_implement_drop() {
        // Regression test: Wayland proxy objects like WlSurface and ZwlrLayerSurfaceV1
        // must be explicitly destroyed in wayland-rs because dropping the proxy
        // struct does not send a destroy request.
        // We ensure that WaylandBar and ModuleSurface explicitly implement Drop
        // so they don't leak Wayland objects when cleared during config hot-reloads.
        fn assert_impl_drop<T: Drop>() {}

        assert_impl_drop::<super::WaylandBar>();
        assert_impl_drop::<super::ModuleSurface>();
    }

    #[tokio::test]
    async fn test_wayland_state_initialization() {
        let (command_tx, _) = tokio::sync::mpsc::channel(10);
        let config = crate::shared::config::domain::Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let state = WaylandState {
            hub,
            compositor: None,
            shm: None,
            layer_shell: None,
            xdg_wm_base: None,
            subcompositor: None,
            outputs: Vec::new(),
            bars: Vec::new(),
            seat: None,
            pointer: None,
            command_tx,
            surface_to_id: HashMap::new(),
            pointer_surface: None,
            pointer_pos: (0.0, 0.0),
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            tooltip: None,
        };
        assert!(state.bars.is_empty());
        assert!(state.outputs.is_empty());

        // Dummy reads to satisfy clippy in test profile (since these are hidden behind #[cfg(not(test))] implementations)
        let _ = &state.hub;
        let _ = &state.subcompositor;
        let _ = &state.seat;
        let _ = &state.pointer;
        let _ = &state.command_tx;

        // Removed dummy reads since the real implementation is compiled in test mode now.
    }

    #[tokio::test]
    async fn test_wayland_adapter_methods() {
        use std::os::unix::io::AsFd;
        use std::os::unix::net::UnixStream;
        let (client_stream, _) = UnixStream::pair().unwrap();
        // wayland_client::backend::Backend might require some imports
        let backend = wayland_client::backend::Backend::connect(client_stream).unwrap();
        let connection = Connection::from_backend(backend);
        let event_queue = connection.new_event_queue::<WaylandState>();
        let async_fd =
            tokio::io::unix::AsyncFd::new(WaylandFd(connection.as_fd().as_raw_fd())).unwrap();

        let (command_tx, _) = tokio::sync::mpsc::channel(10);
        let config = crate::shared::config::domain::Config::default();
        let hub = Arc::new(SignalHub::new(config));
        let config_rx = hub.config_rx();

        let state = WaylandState {
            hub: hub.clone(),
            compositor: None,
            shm: None,
            layer_shell: None,
            xdg_wm_base: None,
            subcompositor: None,
            outputs: Vec::new(),
            bars: Vec::new(),
            seat: None,
            pointer: None,
            command_tx: command_tx.clone(),
            surface_to_id: HashMap::new(),
            pointer_surface: None,
            pointer_pos: (0.0, 0.0),
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            tooltip: None,
        };

        let pending_surfaces = Arc::new(Mutex::new(HashMap::new()));
        let (_, notify_rx) = tokio::sync::mpsc::channel(1);

        let mut adapter = WaylandAdapter {
            connection,
            event_queue,
            state,
            async_fd,
            pending_surfaces,
            notify_rx,
            config_rx,
        };

        assert!(adapter.flush().is_ok());

        let _ = adapter.dispatch_pending();

        let cmd = SurfaceCommand {
            module_id: crate::shared::primitives::ModuleId::new(1),
            monitor_id: crate::shared::primitives::MonitorId::new("test"),
            position: crate::shared::primitives::geometry::Position::new(0, 0),
            buffer: crate::shared::primitives::render::RenderBuffer::new(
                vec![0; 4],
                crate::shared::primitives::geometry::Size::new(1, 1),
            ),
        };
        let _ = adapter.handle_surface_command(cmd);

        let (tx, _rx) = tokio::sync::mpsc::channel(10);
        let _ = WaylandAdapter::new(hub.clone(), tx);
    }
}
