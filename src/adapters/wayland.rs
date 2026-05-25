use crate::ports::DisplayServerPort;
use crate::ports::canvas::Canvas;
use crate::domain::errors::PortError;
use crate::domain::signals::SignalHub;
use crate::domain::app::CrankyApp;
use crate::adapters::rendering::TinySkiaCosmicCanvas;
use crate::core::shm::ShmBuffer;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::io::unix::AsyncFd;
use tracing::{info, debug, info_span};
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle,
    backend::WaylandError,
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::{self, WlOutput},
        wl_pointer::{self, WlPointer},
        wl_registry::{self, WlRegistry},
        wl_seat::{self, WlSeat},
        wl_shm::WlShm,
        wl_subcompositor::WlSubcompositor,
        wl_surface::WlSurface,
        wl_buffer::WlBuffer,
        wl_shm_pool::WlShmPool,
        wl_subsurface::WlSubsurface,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{ZwlrLayerShellV1, Layer},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1, Anchor},
};
use cosmic_text::{FontSystem, SwashCache};
use std::collections::HashMap;
use std::os::unix::io::{AsFd, AsRawFd, RawFd};
use tiny_skia::PixmapMut;

struct WaylandFd(RawFd);
impl AsRawFd for WaylandFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

pub struct WaylandAdapter {
    connection: Connection,
    event_queue: EventQueue<WaylandState>,
    state: WaylandState,
    async_fd: AsyncFd<WaylandFd>,
}

pub struct WaylandState {
    hub: Arc<SignalHub>,
    compositor: Option<WlCompositor>,
    shm: Option<WlShm>,
    layer_shell: Option<ZwlrLayerShellV1>,
    subcompositor: Option<WlSubcompositor>,
    outputs: Vec<WaylandOutputInfo>,
    bars: Vec<WaylandBar>,
    seat: Option<WlSeat>,
    pointer: Option<WlPointer>,
    
    command_tx: tokio::sync::mpsc::Sender<crate::domain::commands::AppCommand>,
    
    surface_to_id: HashMap<WlSurface, crate::domain::ModuleId>,
    pointer_surface: Option<WlSurface>,
    pointer_pos: (f64, f64),
    
    font_system: FontSystem,
    swash_cache: SwashCache,
}

struct WaylandOutputInfo {
    output: WlOutput,
    id: u32,
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
    config_margin: crate::domain::config::MarginConfig,
    scale: i32,
    module_surfaces: HashMap<crate::domain::ModuleId, ModuleSurface>,
}

struct ModuleSurface {
    surface: WlSurface,
    subsurface: WlSubsurface,
    shm_buffer: ShmBuffer,
    size: crate::domain::geometry::Size,
}

impl WaylandAdapter {
    pub fn new(hub: Arc<SignalHub>, command_tx: tokio::sync::mpsc::Sender<crate::domain::commands::AppCommand>) -> Result<Self, PortError> {
        let connection = Connection::connect_to_env().map_err(|e| PortError::DisplayConnectionFailed { 
            reason: e.to_string() 
        })?;
        let event_queue = connection.new_event_queue();
        let qh = event_queue.handle();

        connection.display().get_registry(&qh, ());

        let raw_fd = connection.as_fd().as_raw_fd();
        let async_fd = AsyncFd::new(WaylandFd(raw_fd))
            .map_err(|e| PortError::DisplayConnectionFailed { reason: e.to_string() })?;

        let state = WaylandState {
            hub,
            compositor: None,
            shm: None,
            layer_shell: None,
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
        };

        Ok(Self {
            connection,
            event_queue,
            state,
            async_fd,
        })
    }
}

#[async_trait]
impl DisplayServerPort for WaylandAdapter {
    fn create_bar(&self, _output_id: u32, _name: &str) -> Result<(), PortError> { Ok(()) }
    fn destroy_bar(&self, _output_id: u32) -> Result<(), PortError> { Ok(()) }

    async fn wait_for_events(&mut self) -> Result<(), PortError> {
        let mut read_guard = self.event_queue.prepare_read();
        if let Some(r_guard) = read_guard.take() {
            if let Ok(mut guard) = self.async_fd.readable().await {
                match r_guard.read() {
                    Ok(_) => {
                        guard.retain_ready();
                    }
                    Err(WaylandError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        guard.clear_ready();
                    }
                    Err(e) => return Err(PortError::DisplayConnectionFailed { reason: e.to_string() }),
                }
            }
        }
        Ok(())
    }

    fn dispatch_pending(&mut self) -> Result<(), PortError> {
        self.event_queue.dispatch_pending(&mut self.state)
            .map_err(|e| PortError::DisplayConnectionFailed { reason: e.to_string() })?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), PortError> {
        let _ = self.connection.flush();
        Ok(())
    }

    fn render_all(&mut self, app: &mut CrankyApp) -> Result<(), PortError> {
        let qh = self.event_queue.handle();
        self.render_all_outputs(app, &qh)
    }
}

impl WaylandAdapter {
    fn render_all_outputs(&mut self, app: &mut CrankyApp, qh: &QueueHandle<WaylandState>) -> Result<(), PortError> {
        let span = info_span!("render_all_outputs");
        let _enter = span.enter();
        
        // Prepare app state once per render pass
        app.prepare_render();

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

        let compositor = compositor.as_ref().unwrap();
        let subcompositor = subcompositor.as_ref().unwrap();
        let shm = shm.as_ref().unwrap();

        for bar in bars {
            debug!("Rendering bar for output: {} (size: {}x{}, scale: {})", bar.output_name, bar.width, bar.height, bar.scale);
            let (width, height, scale) = (bar.width, bar.height, bar.scale);
            let physical_width = width * scale as u32;
            let physical_height = height * scale as u32;

            let pixmap_data = bar.shm_buffer.mmap_mut();
            let pixmap = PixmapMut::from_bytes(pixmap_data, physical_width, physical_height).unwrap();
            
            let bar_config = app.config().bar().clone();
            let config_bg = bar_config.background().clone();
            let border_config = bar_config.border();
            
            // Check if hot-reload of height or margin is needed
            if bar.config_height != bar_config.height() || bar.config_margin != *bar_config.margin() {
                debug!("Hot-reloading bar height/margin for output: {}", bar.output_name);
                bar.config_height = bar_config.height();
                bar.config_margin = bar_config.margin().clone();
                
                let margin = bar_config.margin();
                bar.layer_surface.set_size(0, bar.config_height);
                bar.layer_surface.set_margin(
                    margin.top().value(),
                    margin.right().value(),
                    margin.bottom().value(),
                    margin.left().value(),
                );
                bar.layer_surface.set_exclusive_zone(bar.config_height as i32 + margin.top().value() + margin.bottom().value());
                bar.surface.commit();
                
                // Note: The actual resize will happen asynchronously via the Configure event.
                // We proceed with the current size for this frame to avoid visual glitches.
            }

            let mut bar_canvas = TinySkiaCosmicCanvas::new(
                pixmap,
                font_system,
                swash_cache,
                scale as f32
            );
            bar_canvas.clear();
            let border_size = border_config.size().value();
            let half_border = border_size / 2.0;
            
            bar_canvas.draw_rect(0.0, 0.0, width as f32, height as f32, config_bg, border_config.radius().value());
            bar_canvas.draw_border(
                half_border,
                half_border,
                width as f32 - border_size,
                height as f32 - border_size,
                border_config.color().clone(),
                border_config.radius().value(),
                border_size,
            );

            // Calculate layout
            let monitor_id = crate::domain::MonitorId::new(&bar.output_name);
            let layouts = app.calculate_layout(&monitor_id, width, &mut bar_canvas);

            for layout in layouts {
                let module_id = layout.id();
                let bounds = layout.bounds();
                
                let ms = bar.module_surfaces.entry(module_id).or_insert_with(|| {
                    let surface = compositor.create_surface(qh, ());
                    let subsurface = subcompositor.get_subsurface(&surface, &bar.surface, qh, ());
                    subsurface.set_desync();
                    
                    let width = (bounds.width() * scale as u32).max(1);
                    let height = (bounds.height() * scale as u32).max(1);
                    let shm_buffer = ShmBuffer::new(shm, width, height, qh).unwrap();
                    
                    surface_to_id.insert(surface.clone(), module_id);

                    ModuleSurface {
                        surface,
                        subsurface,
                        shm_buffer,
                        size: *bounds.size(),
                    }
                });

                // Update position
                ms.subsurface.set_position(bounds.x(), bounds.y());

                // Recreate buffer if size changed
                if ms.size != *bounds.size() {
                    ms.size = *bounds.size();
                    let width = (bounds.width() * scale as u32).max(1);
                    let height = (bounds.height() * scale as u32).max(1);
                    ms.shm_buffer = ShmBuffer::new(shm, width, height, qh).unwrap();
                }

                // Render module into its own buffer
                let module_pixmap_data = ms.shm_buffer.mmap_mut();
                let width = (bounds.width() * scale as u32).max(1);
                let height = (bounds.height() * scale as u32).max(1);
                let module_pixmap = PixmapMut::from_bytes(module_pixmap_data, width, height).unwrap();
                
                let mut module_canvas = TinySkiaCosmicCanvas::new(
                    module_pixmap,
                    font_system,
                    swash_cache,
                    scale as f32
                );
                
                // Clear module surface (transparent)
                module_canvas.clear();
                
                app.render_module(module_id, &mut module_canvas, &monitor_id);

                let buffer = ms.shm_buffer.current_buffer();

                ms.surface.set_buffer_scale(scale);
                ms.surface.attach(Some(buffer), 0, 0);
                ms.surface.damage(0, 0, bounds.width() as i32, bounds.height() as i32);
                ms.surface.commit();
                ms.shm_buffer.swap_buffers();
            }

            let buffer = bar.shm_buffer.current_buffer();

            bar.surface.set_buffer_scale(scale);
            bar.surface.attach(Some(buffer), 0, 0);
            bar.surface.damage(0, 0, width as i32, height as i32);
            bar.surface.commit();
            bar.shm_buffer.swap_buffers();
        }
        let _ = self.connection.flush();
        Ok(())
    }
}



impl WaylandState {
    fn create_bar(&mut self, output: &WlOutput, qh: &QueueHandle<Self>) -> Result<(), PortError> {
        let (output_name, output_scale) = {
            let info = self.outputs.iter().find(|i| &i.output == output).ok_or_else(|| PortError::DisplayConnectionFailed { reason: "Output not found".to_string() })?;
            if info.name.is_empty() { return Ok(()); }
            (info.name.clone(), info.scale)
        };

        if self.bars.iter().any(|b| b.output_name == output_name) { return Ok(()); }

        let bar_config = self.hub.config_rx().borrow().bar().clone();
        let bar_height = bar_config.height();
        let margin = bar_config.margin();
        info!("Creating bar for output: {} (height: {}, scale: {})", output_name, bar_height, output_scale);

        let compositor = self.compositor.as_ref().ok_or(PortError::DisplayConnectionFailed { reason: "Compositor not bound".to_string() })?;
        let layer_shell = self.layer_shell.as_ref().ok_or(PortError::DisplayConnectionFailed { reason: "Layer shell not bound".to_string() })?;
        let shm = self.shm.as_ref().ok_or(PortError::DisplayConnectionFailed { reason: "SHM not bound".to_string() })?;

        let surface = compositor.create_surface(qh, ());
        let layer_surface = layer_shell.get_layer_surface(&surface, Some(output), Layer::Top, "cranky".to_string(), qh, ());

        layer_surface.set_anchor(Anchor::Top | Anchor::Left | Anchor::Right);
        layer_surface.set_size(0, bar_height);
        layer_surface.set_margin(
            margin.top().value(),
            margin.right().value(),
            margin.bottom().value(),
            margin.left().value(),
        );
        layer_surface.set_exclusive_zone(bar_height as i32 + margin.top().value() + margin.bottom().value());
        surface.set_buffer_scale(output_scale);
        surface.commit();

        let shm_buffer = ShmBuffer::new(shm, 1920 * output_scale as u32, bar_height * output_scale as u32, qh).map_err(|e| PortError::Io(e))?;

        self.bars.push(WaylandBar {
            output_name,
            surface,
            layer_surface,
            shm_buffer,
            width: 1920,
            height: bar_height,
            config_height: bar_height,
            config_margin: margin.clone(),
            scale: output_scale,
            module_surfaces: HashMap::new(),
        });

        // Request an initial render now that a bar is created
        let _ = self.hub.dirty_tx().send(crate::domain::ModuleId::new(0));

        Ok(())
    }
}

impl Dispatch<WlRegistry, ()> for WaylandState {
    fn event(state: &mut Self, proxy: &WlRegistry, event: wl_registry::Event, _data: &(), _conn: &Connection, qh: &QueueHandle<Self>) {
        match event {
            wl_registry::Event::Global { name, interface, version } => {
                match interface.as_str() {
                    "wl_compositor" => state.compositor = Some(proxy.bind(name, version, qh, ())),
                    "wl_shm" => state.shm = Some(proxy.bind(name, version, qh, ())),
                    "zwlr_layer_shell_v1" => state.layer_shell = Some(proxy.bind(name, version, qh, ())),
                    "wl_subcompositor" => state.subcompositor = Some(proxy.bind(name, version, qh, ())),
                    "wl_output" => {
                        let output: WlOutput = proxy.bind(name, version, qh, ());
                        state.outputs.push(WaylandOutputInfo { output, id: name, name: String::new(), scale: 1 });
                    }
                    "wl_seat" => state.seat = Some(proxy.bind(name, version, qh, ())),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlCompositor, ()> for WaylandState { fn event(_: &mut Self, _: &WlCompositor, _: wayland_client::protocol::wl_compositor::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<WlShm, ()> for WaylandState { fn event(_: &mut Self, _: &WlShm, _: wayland_client::protocol::wl_shm::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<ZwlrLayerShellV1, ()> for WaylandState { fn event(_: &mut Self, _: &ZwlrLayerShellV1, _: wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<WlSubcompositor, ()> for WaylandState { fn event(_: &mut Self, _: &WlSubcompositor, _: wayland_client::protocol::wl_subcompositor::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<WlOutput, ()> for WaylandState { 
    fn event(state: &mut Self, proxy: &WlOutput, event: wl_output::Event, _data: &(), _conn: &Connection, qh: &QueueHandle<Self>) {
        if let Some(info) = state.outputs.iter_mut().find(|i| &i.output == proxy) {
            match event {
                wl_output::Event::Name { name } => info.name = name,
                wl_output::Event::Scale { factor } => info.scale = factor,
                wl_output::Event::Done => { let _ = state.create_bar(proxy, qh); }
                _ => {}
            }
        }
    }
}
impl Dispatch<WlSeat, ()> for WaylandState {
    fn event(state: &mut Self, proxy: &WlSeat, event: wl_seat::Event, _data: &(), _conn: &Connection, qh: &QueueHandle<Self>) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            let caps = wayland_client::protocol::wl_seat::Capability::from_bits(capabilities.into()).unwrap_or(wayland_client::protocol::wl_seat::Capability::empty());
            if caps.contains(wayland_client::protocol::wl_seat::Capability::Pointer) {
                state.pointer = Some(proxy.get_pointer(qh, ()));
            }
        }
    }
}
impl Dispatch<WlPointer, ()> for WaylandState {
    fn event(state: &mut Self, proxy: &WlPointer, event: wl_pointer::Event, _data: &(), _conn: &Connection, qh: &QueueHandle<Self>) {
        use crate::domain::events::InputEvent;
        use crate::domain::commands::AppCommand;

        match event {
            wl_pointer::Event::Enter { surface, surface_x, surface_y, .. } => {
                state.pointer_surface = Some(surface.clone());
                state.pointer_pos = (surface_x, surface_y);
                if let Some(id) = state.surface_to_id.get(&surface) {
                    let _ = state.command_tx.try_send(AppCommand::Input(*id, InputEvent::PointerEnter));
                }
            }
            wl_pointer::Event::Leave { surface, .. } => {
                if let Some(surface) = state.pointer_surface.take() {
                    if let Some(id) = state.surface_to_id.get(&surface) {
                        let _ = state.command_tx.try_send(AppCommand::Input(*id, InputEvent::PointerLeave));
                    }
                }
            }
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                state.pointer_pos = (surface_x, surface_y);
            }
            wl_pointer::Event::Button { button, state: button_state, .. } => {
                // Send click event on button release (0 is release, 1 is press in wl_pointer::ButtonState)
                if button_state == wayland_client::WEnum::Value(wayland_client::protocol::wl_pointer::ButtonState::Released) {
                    if let Some(surface) = &state.pointer_surface {
                        if let Some(id) = state.surface_to_id.get(surface) {
                            let _ = state.command_tx.try_send(AppCommand::Input(
                                *id,
                                InputEvent::Click { 
                                    button,
                                    x: state.pointer_pos.0,
                                    y: state.pointer_pos.1,
                                }
                            ));
                        }
                    }
                }
            }
            wl_pointer::Event::Axis { axis, value, .. } => {
                if let Some(surface) = &state.pointer_surface {
                    if let Some(id) = state.surface_to_id.get(surface) {
                        let axis_val = match axis {
                            wayland_client::WEnum::Value(wayland_client::protocol::wl_pointer::Axis::VerticalScroll) => 0,
                            wayland_client::WEnum::Value(wayland_client::protocol::wl_pointer::Axis::HorizontalScroll) => 1,
                            _ => 0,
                        };
                        let _ = state.command_tx.try_send(AppCommand::Input(
                            *id,
                            InputEvent::Scroll {
                                axis: axis_val,
                                amount: value,
                            }
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}
impl Dispatch<WlSurface, ()> for WaylandState { fn event(_: &mut Self, _: &WlSurface, _: wayland_client::protocol::wl_surface::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<ZwlrLayerSurfaceV1, ()> for WaylandState { 
    fn event(state: &mut Self, proxy: &ZwlrLayerSurfaceV1, event: zwlr_layer_surface_v1::Event, _data: &(), _conn: &Connection, qh: &QueueHandle<Self>) {
        if let zwlr_layer_surface_v1::Event::Configure { serial, width, height } = event {
            proxy.ack_configure(serial);
            if let Some(bar) = state.bars.iter_mut().find(|b| &b.layer_surface == proxy) {
                if width > 0 && height > 0 {
                    let old_width = bar.width;
                    let old_height = bar.height;
                    
                    bar.width = width;
                    bar.height = height;

                    if old_width != width || old_height != height {
                        debug!("Bar resized to {}x{} (scale: {})", width, height, bar.scale);
                        let physical_width = width * bar.scale as u32;
                        let physical_height = height * bar.scale as u32;
                        
                        if let Ok(new_shm) = ShmBuffer::new(state.shm.as_ref().unwrap(), physical_width, physical_height, qh) {
                            bar.shm_buffer = new_shm;
                            // Request a render immediately after configuration
                            let tx = state.hub.dirty_tx();
                            tokio::spawn(async move {
                                let _ = tx.send(crate::domain::ModuleId::new(0)).await;
                            });
                        }
                    }
                }
            }
        }
    }
}
impl Dispatch<WlBuffer, ()> for WaylandState { fn event(_: &mut Self, _: &WlBuffer, _: wayland_client::protocol::wl_buffer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
impl Dispatch<WlShmPool, ()> for WaylandState { 
    fn event(_: &mut Self, _: &WlShmPool, _: wayland_client::protocol::wl_shm_pool::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} 
}
impl Dispatch<WlSubsurface, ()> for WaylandState { fn event(_: &mut Self, _: &WlSubsurface, _: wayland_client::protocol::wl_subsurface::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {} }
