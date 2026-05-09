use crate::ports::DisplayServerPort;
use crate::ports::canvas::Canvas;
use crate::domain::errors::PortError;
use crate::domain::signals::{SignalHub, PointerEvent};
use crate::domain::commands::AppCommand;
use crate::domain::app::CrankyApp;
use crate::adapters::rendering::TinySkiaCosmicCanvas;
use crate::core::shm::ShmBuffer;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::io::unix::AsyncFd;
use tracing::{info, error, debug, info_span, debug_span, trace_span};
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
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1, Layer},
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
    
    surface_to_id: HashMap<WlSurface, u32>,
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
    buffer: Option<WlBuffer>,
    width: u32,
    height: u32,
    scale: i32,
}

impl WaylandAdapter {
    pub fn new(hub: Arc<SignalHub>) -> Result<Self, PortError> {
        let connection = Connection::connect_to_env().map_err(|e| PortError::DisplayConnectionFailed { 
            reason: e.to_string() 
        })?;
        let event_queue = connection.new_event_queue();
        let qh = event_queue.handle();

        connection.display().get_registry(&qh, ());

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
        })
    }

    pub async fn run(
        &mut self, 
        mut app: CrankyApp,
        mut command_rx: mpsc::Receiver<AppCommand>
    ) -> Result<(), PortError> {
        info!("Wayland adapter starting main loop...");

        let raw_fd = self.connection.as_fd().as_raw_fd();
        let async_fd = AsyncFd::new(WaylandFd(raw_fd))
            .map_err(|e| PortError::DisplayConnectionFailed { reason: e.to_string() })?;

        loop {
            let wayland_loop_span = info_span!("wayland_loop_iteration");
            let _enter = wayland_loop_span.enter();

            let _ = self.connection.flush();
            
            self.event_queue.dispatch_pending(&mut self.state)
                .map_err(|e| PortError::DisplayConnectionFailed { reason: e.to_string() })?;

            let mut read_guard = self.event_queue.prepare_read();

            tokio::select! {
                read_ready = async_fd.readable(), if read_guard.is_some() => {
                    let span = trace_span!("dispatch_events");
                    let _enter = span.enter();
                    if let Ok(mut guard) = read_ready {
                        let r_guard = read_guard.take().unwrap();
                        match r_guard.read() {
                            Ok(_) => { 
                                guard.retain_ready(); 
                                self.event_queue.dispatch_pending(&mut self.state)
                                    .map_err(|e| PortError::DisplayConnectionFailed { reason: e.to_string() })?;
                            }
                            Err(WaylandError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => { guard.clear_ready(); }
                            Err(e) => { error!("Wayland read error: {:?}", e); }
                        }
                    } else {
                        drop(read_guard);
                    }
                }
                _ = tokio::task::yield_now(), if read_guard.is_none() => {}
                Some(command) = command_rx.recv() => {
                    drop(read_guard);
                    self.handle_command(command, &mut app)?;
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                    drop(read_guard);
                }
            }
        }
    }

    fn handle_command(
        &mut self, 
        command: AppCommand, 
        app: &mut CrankyApp
    ) -> Result<(), PortError> {
        let span = debug_span!("handle_command", ?command);
        let _enter = span.enter();
        let qh = self.event_queue.handle();
        match command {
            AppCommand::RequestRender(output_id) => {
                debug!("Received RequestRender command for output {}", output_id);
                self.render_all_outputs(app, &qh)?;
            }
            AppCommand::CreateBar(id, name) => {
                info!("Command: Create bar {} for output {}", name, id);
            }
            AppCommand::DestroyBar(id) => {
                info!("Command: Destroy bar for output {}", id);
            }
            AppCommand::Log(level, msg) => {
                match level {
                    tracing::Level::ERROR => tracing::error!("{}", msg),
                    tracing::Level::WARN => tracing::warn!("{}", msg),
                    tracing::Level::INFO => tracing::info!("{}", msg),
                    tracing::Level::DEBUG => tracing::debug!("{}", msg),
                    tracing::Level::TRACE => tracing::trace!("{}", msg),
                }
            }
        }
        Ok(())
    }

    fn render_all_outputs(&mut self, app: &mut CrankyApp, qh: &QueueHandle<WaylandState>) -> Result<(), PortError> {
        let span = info_span!("render_all_outputs");
        let _enter = span.enter();
        let WaylandState {
            ref mut bars,
            ref mut font_system,
            ref mut swash_cache,
            ..
        } = self.state;

        if bars.is_empty() {
            debug!("No bars available for rendering.");
            return Ok(());
        }

        for bar in bars {
            debug!("Rendering bar for output: {} (size: {}x{}, scale: {})", bar.output_name, bar.width, bar.height, bar.scale);
            let (width, height, scale) = (bar.width, bar.height, bar.scale);
            let pixmap_data = bar.shm_buffer.mmap_mut();
            let mut pixmap = PixmapMut::from_bytes(pixmap_data, width, height).unwrap();
            
            // Clear and draw background
            let config_bg = app.config().bar().background().clone();
            let mut canvas = TinySkiaCosmicCanvas::new(
                pixmap,
                font_system,
                swash_cache,
                scale as f32
            );
            canvas.draw_rect(0.0, 0.0, width as f32 / scale as f32, height as f32 / scale as f32, config_bg, 0.0);

            app.render(0, &mut canvas, &bar.output_name).map_err(|e| PortError::SurfaceError { 
                target_id: 0, 
                reason: e.to_string() 
            })?;

            let buffer = bar.shm_buffer.pool().create_buffer(
                0,
                width as i32,
                height as i32,
                (width * 4) as i32,
                wayland_client::protocol::wl_shm::Format::Argb8888,
                qh,
                ()
            );

            bar.surface.attach(Some(&buffer), 0, 0);
            bar.surface.damage(0, 0, width as i32, height as i32);
            bar.surface.commit();
            bar.buffer = Some(buffer);
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

        let bar_height = self.hub.config_rx().borrow().bar().height();
        info!("Creating bar for output: {} (height: {})", output_name, bar_height);

        let compositor = self.compositor.as_ref().ok_or(PortError::DisplayConnectionFailed { reason: "Compositor not bound".to_string() })?;
        let layer_shell = self.layer_shell.as_ref().ok_or(PortError::DisplayConnectionFailed { reason: "Layer shell not bound".to_string() })?;
        let shm = self.shm.as_ref().ok_or(PortError::DisplayConnectionFailed { reason: "SHM not bound".to_string() })?;

        let surface = compositor.create_surface(qh, ());
        let layer_surface = layer_shell.get_layer_surface(&surface, Some(output), Layer::Top, "cranky".to_string(), qh, ());

        layer_surface.set_anchor(Anchor::Top | Anchor::Left | Anchor::Right);
        layer_surface.set_size(0, bar_height);
        layer_surface.set_exclusive_zone(bar_height as i32);
        surface.commit();

        let shm_buffer = ShmBuffer::new(shm, 1920, bar_height, qh).map_err(|e| PortError::Io(e))?;

        self.bars.push(WaylandBar {
            output_name,
            surface,
            layer_surface,
            shm_buffer,
            buffer: None,
            width: 1920,
            height: bar_height,
            scale: output_scale,
        });

        // Request an initial render now that a bar is created
        let _ = self.hub.dirty_tx().send(0);

        Ok(())
    }
}

impl DisplayServerPort for WaylandAdapter {
    fn create_bar(&self, _output_id: u32, _name: &str) -> Result<(), PortError> { Ok(()) }
    fn destroy_bar(&self, _output_id: u32) -> Result<(), PortError> { Ok(()) }
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
    fn event(state: &mut Self, _proxy: &WlPointer, event: wl_pointer::Event, _data: &(), _conn: &Connection, _qh: &QueueHandle<Self>) {
        match event {
            wl_pointer::Event::Enter { surface, surface_x, surface_y, .. } => {
                state.pointer_surface = Some(surface.clone());
                state.pointer_pos = (surface_x, surface_y);
                if let Some(&id) = state.surface_to_id.get(&surface) {
                    let _ = state.hub.pointer_tx().send(PointerEvent::Enter { target_id: id, x: surface_x, y: surface_y });
                }
            }
            wl_pointer::Event::Leave { surface, .. } => {
                if let Some(&id) = state.surface_to_id.get(&surface) {
                    let _ = state.hub.pointer_tx().send(PointerEvent::Leave { target_id: id });
                }
                state.pointer_surface = None;
            }
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                state.pointer_pos = (surface_x, surface_y);
                if let Some(surface) = &state.pointer_surface {
                    if let Some(&id) = state.surface_to_id.get(surface) {
                        let _ = state.hub.pointer_tx().send(PointerEvent::Motion { target_id: id, x: surface_x, y: surface_y });
                    }
                }
            }
            wl_pointer::Event::Button { button, state: button_state, .. } => {
                if button_state == wayland_client::WEnum::Value(wl_pointer::ButtonState::Pressed) {
                    if let Some(surface) = &state.pointer_surface {
                        if let Some(&id) = state.surface_to_id.get(surface) {
                            let (x, y) = state.pointer_pos;
                            let _ = state.hub.pointer_tx().send(PointerEvent::Click { target_id: id, x, y, button });
                        }
                    }
                }
            }
            wl_pointer::Event::Axis { axis, value, .. } => {
                if let Some(surface) = &state.pointer_surface {
                    if let Some(&id) = state.surface_to_id.get(surface) {
                        let axis_val = match axis {
                            wayland_client::WEnum::Value(v) => v as u32,
                            wayland_client::WEnum::Unknown(v) => v,
                        };
                        let _ = state.hub.pointer_tx().send(PointerEvent::Scroll { target_id: id, axis: axis_val, value });
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
                    bar.width = width;
                    bar.height = height;
                    if let Ok(new_shm) = ShmBuffer::new(state.shm.as_ref().unwrap(), width, height, qh) {
                        bar.shm_buffer = new_shm;
                        // Request a render immediately after configuration
                        let tx = state.hub.dirty_tx();
                        tokio::spawn(async move {
                            let _ = tx.send(0).await;
                        });
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
