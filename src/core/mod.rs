use crate::config::Config;
use crate::modules::{Event, ModuleRegistry, UpdateAction};
use std::os::unix::io::{AsFd, AsRawFd, RawFd};
use thiserror::Error;
use tokio::io::unix::AsyncFd;
use tokio::sync::mpsc;
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle,
    backend::WaylandError,
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::{self, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_shm::WlShm,
    },
};

struct WaylandFd(RawFd);
impl AsRawFd for WaylandFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

pub mod bar;
pub mod hyprland;
pub mod shm;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Wayland connection failed: {0}")]
    Connection(String),
    #[error("Wayland global error: {0}")]
    Global(String),
    #[error("Wayland dispatch error: {0}")]
    Dispatch(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;

pub struct WaylandManager {
    connection: Connection,
    event_queue: EventQueue<CrankyState>,
    state: CrankyState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TickCadence {
    Sleep(std::time::Duration),
    Yield,
}

fn tick_cadence(mode: &crate::config::RenderingMode) -> TickCadence {
    match mode {
        crate::config::RenderingMode::Immediate { fps_limit } => match fps_limit {
            Some(fps) if *fps > 0 => {
                TickCadence::Sleep(std::time::Duration::from_secs_f64(1.0 / (*fps as f64)))
            }
            _ => TickCadence::Yield,
        },
        crate::config::RenderingMode::Timebased { duration_ms } => {
            TickCadence::Sleep(std::time::Duration::from_millis((*duration_ms).max(1)))
        }
    }
}

pub struct OutputInfo {
    output: WlOutput,
    id: u32,
    name: String,
    scale: i32,
}

impl OutputInfo {
    pub fn output(&self) -> &WlOutput {
        &self.output
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn scale(&self) -> i32 {
        self.scale
    }
}

pub struct WaylandGlobals {
    compositor: WlCompositor,
    shm: WlShm,
    layer_shell: ZwlrLayerShellV1,
}

impl WaylandGlobals {
    pub fn new(compositor: WlCompositor, shm: WlShm, layer_shell: ZwlrLayerShellV1) -> Self {
        Self {
            compositor,
            shm,
            layer_shell,
        }
    }

    pub fn compositor(&self) -> &WlCompositor {
        &self.compositor
    }

    pub fn shm(&self) -> &WlShm {
        &self.shm
    }

    pub fn layer_shell(&self) -> &ZwlrLayerShellV1 {
        &self.layer_shell
    }
}

impl CrankyState {
    fn add_output(&mut self, id: u32, output: WlOutput) {
        self.outputs.push(OutputInfo {
            output,
            id,
            name: String::new(),
            scale: 1,
        });
    }

    fn remove_output(&mut self, id: u32) {
        if let Some(index) = self.outputs.iter().position(|i| i.id == id) {
            let info = self.outputs.remove(index);
            log::info!("Output removed: {} (id: {})", info.name, info.id);
            self.bars.retain(|b| b.monitor_name() != info.name);
        }
    }

    fn get_output_info(&self, output: &WlOutput) -> Option<&OutputInfo> {
        self.outputs.iter().find(|i| &i.output == output)
    }

    fn get_output_info_mut(&mut self, output: &WlOutput) -> Option<&mut OutputInfo> {
        self.outputs.iter_mut().find(|i| &i.output == output)
    }

    fn globals(&self) -> Option<WaylandGlobals> {
        match (&self.compositor, &self.shm, &self.layer_shell) {
            (Some(c), Some(s), Some(l)) => {
                Some(WaylandGlobals::new(c.clone(), s.clone(), l.clone()))
            }
            _ => None,
        }
    }

    fn create_bar_for_output(&mut self, output: &WlOutput, qh: &QueueHandle<Self>) {
        let (info_name, info_scale) = if let Some(info) = self.get_output_info(output) {
            if info.name.is_empty() {
                return;
            }
            (info.name.clone(), info.scale)
        } else {
            return;
        };

        // Check if bar already exists
        if self.bars.iter().any(|b| b.monitor_name() == info_name) {
            return;
        }

        if let Some(globals) = self.globals() {
            log::info!(
                "Creating bar for output: {} (scale: {})",
                info_name,
                info_scale
            );

            let bar_res = {
                let info = self.get_output_info(output).unwrap();
                bar::Bar::new(info, &globals, &self.config, qh)
            };

            match bar_res {
                Ok(bar) => self.bars.push(bar),
                Err(e) => log::error!("Failed to create bar for output {}: {}", info_name, e),
            }
        }
    }
}

pub struct CrankyState {
    registry: ModuleRegistry,
    config: Config,
    render_context: crate::render::RenderContext,
    error_message: Option<String>,

    // Globals
    compositor: Option<WlCompositor>,
    shm: Option<WlShm>,
    layer_shell: Option<ZwlrLayerShellV1>,

    // Monitors
    outputs: Vec<OutputInfo>,

    // Bars
    bars: Vec<bar::Bar>,

    // Hyprland
    hyprland_provider: Box<dyn crate::core::hyprland::HyprlandProvider>,
    focused_monitor: Option<String>,
}

impl WaylandManager {
    fn handle_periodic_update(&mut self) {
        let mut redraw = self.state.registry.update(Event::Timer) == UpdateAction::Redraw;

        // Centralized Hyprland polling
        let workspaces = self
            .state
            .hyprland_provider
            .get_workspaces()
            .unwrap_or_default();
        let monitors = self
            .state
            .hyprland_provider
            .get_monitors()
            .unwrap_or_default();

        let new_focused = monitors
            .iter()
            .find(|m| m.focused())
            .map(|m| m.name().to_string());

        if new_focused != self.state.focused_monitor {
            self.state.focused_monitor = new_focused;
            redraw = true;
        }

        if self.state.registry.update(Event::HyprlandUpdate {
            workspaces,
            monitors,
        }) == UpdateAction::Redraw
        {
            redraw = true;
        }

        if redraw {
            let qh = self.event_queue.handle();
            let shm = self.state.shm.as_ref().unwrap().clone();
            for bar in &mut self.state.bars {
                let bar_config =
                    if Some(bar.monitor_name()) == self.state.focused_monitor.as_deref() {
                        self.state.config.bar().clone()
                    } else {
                        self.state.config.bar().as_unfocused()
                    };

                bar.update_config(&shm, &self.state.config, &bar_config, &qh);
                bar.render(
                    &self.state.config,
                    &bar_config,
                    &self.state.registry,
                    &mut self.state.render_context,
                    &self.state.error_message,
                    &qh,
                );
            }
        }
    }

    pub fn new(config: Config) -> Result<Self> {
        let connection =
            Connection::connect_to_env().map_err(|e| CoreError::Connection(e.to_string()))?;

        let event_queue = connection.new_event_queue();
        let qh = event_queue.handle();

        let _wl_registry = connection.display().get_registry(&qh, ());

        let mut registry = ModuleRegistry::new();
        registry
            .load(&config)
            .map_err(|e| CoreError::Global(e.to_string()))?;

        let state = CrankyState {
            registry,
            config,
            render_context: crate::render::RenderContext::new(),
            error_message: None,
            compositor: None,
            shm: None,
            layer_shell: None,
            outputs: Vec::new(),
            bars: Vec::new(),
            hyprland_provider: Box::new(crate::core::hyprland::RealHyprlandProvider),
            focused_monitor: None,
        };

        Ok(Self {
            connection,
            event_queue,
            state,
        })
    }

    pub async fn run(
        mut self,
        mut config_rx: mpsc::Receiver<std::result::Result<Config, crate::config::ReloadError>>,
    ) -> Result<()> {
        // Initial dispatch to get globals and output info
        self.event_queue
            .roundtrip(&mut self.state)
            .map_err(|e| CoreError::Dispatch(e.to_string()))?;
        self.event_queue
            .roundtrip(&mut self.state)
            .map_err(|e| CoreError::Dispatch(e.to_string()))?;

        let raw_fd = self.connection.as_fd().as_raw_fd();
        let async_fd = AsyncFd::new(WaylandFd(raw_fd))
            .map_err(|e| CoreError::Global(format!("Failed to create AsyncFd: {}", e)))?;

        loop {
            // 1. Flush requests
            let _ = self.connection.flush();

            // 2. Dispatch any already buffered events
            self.event_queue
                .dispatch_pending(&mut self.state)
                .map_err(|e| CoreError::Dispatch(e.to_string()))?;

            let mut read_guard = self.event_queue.prepare_read();
            let tick_cadence = tick_cadence(self.state.config.rendering());
            let tick_sleep = match tick_cadence {
                TickCadence::Sleep(duration) => duration,
                TickCadence::Yield => std::time::Duration::from_millis(0),
            };

            tokio::select! {
                // Wayland events
                read_ready = async_fd.readable(), if read_guard.is_some() => {
                    if let Ok(mut guard) = read_ready {
                        let r_guard = read_guard.take().unwrap();
                        match r_guard.read() {
                            Ok(_) => {
                                guard.retain_ready();
                            }
                            Err(WaylandError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                guard.clear_ready();
                            }
                            Err(e) => {
                                log::error!("Wayland read error: {:?}", e);
                            }
                        }
                    } else {
                        drop(read_guard);
                    }
                }

                // If we can't prepare_read (it returned None), we need to yield to allow dispatch_pending to run again
                _ = tokio::task::yield_now(), if read_guard.is_none() => {
                    // Re-looping immediately calls dispatch_pending
                }

                // Periodic update check (timebased or fps-limited immediate mode)
                _ = tokio::time::sleep(tick_sleep), if matches!(tick_cadence, TickCadence::Sleep(_)) => {
                    drop(read_guard);
                    self.handle_periodic_update();
                }

                // Periodic update check (immediate mode with unlimited fps)
                _ = tokio::task::yield_now(), if matches!(tick_cadence, TickCadence::Yield) => {
                    drop(read_guard);
                    self.handle_periodic_update();
                }

                // Config updates
                res = config_rx.recv() => {
                    drop(read_guard);
                    if let Some(res) = res {
                        match res {
                            Ok(new_config) => {
                                log::info!("Config updated, reloading...");
                                if let Err(e) = self.state.registry.load(&new_config) {
                                    log::error!("Failed to reload modules: {}", e);
                                    self.state.error_message = Some(format!("{}", e));
                                } else {
                                    self.state.config = new_config;
                                    self.state.error_message = None;
                                }
                            }
                            Err(e) => {
                                log::error!("Config reload error: {}", e);
                                self.state.error_message = Some(format!("{}", e));
                            }
                        }

                        // Re-render all bars
                        let qh = self.event_queue.handle();
                        let shm = self.state.shm.as_ref().unwrap().clone();
                        for bar in &mut self.state.bars {
                            let bar_config = if Some(bar.monitor_name()) == self.state.focused_monitor.as_deref() {
                                self.state.config.bar().clone()
                            } else {
                                self.state.config.bar().as_unfocused()
                            };

                            bar.update_config(&shm, &self.state.config, &bar_config, &qh);
                            bar.render(
                                &self.state.config,
                                &bar_config,
                                &self.state.registry,
                                &mut self.state.render_context,
                                &self.state.error_message,
                                &qh,
                            );
                        }
                    }
                }
            }
        }
    }
}

impl Dispatch<WlRegistry, ()> for CrankyState {
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
            } => match &interface[..] {
                "wl_compositor" => {
                    state.compositor =
                        Some(proxy.bind::<WlCompositor, _, _>(name, version, qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(proxy.bind::<WlShm, _, _>(name, version, qh, ()));
                }
                "wl_output" => {
                    let output = proxy.bind::<WlOutput, _, _>(name, version, qh, ());
                    state.add_output(name, output);
                }
                "zwlr_layer_shell_v1" => {
                    state.layer_shell =
                        Some(proxy.bind::<ZwlrLayerShellV1, _, _>(name, version, qh, ()));
                }
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => {
                state.remove_output(name);
            }
            _ => {}
        }
    }
}

impl Dispatch<WlCompositor, ()> for CrankyState {
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

impl Dispatch<WlShm, ()> for CrankyState {
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

impl Dispatch<WlOutput, ()> for CrankyState {
    fn event(
        state: &mut Self,
        proxy: &WlOutput,
        event: wl_output::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_output::Event::Name { name } => {
                log::info!("Output name: {}", name);
                if let Some(info) = state.get_output_info_mut(proxy) {
                    info.name = name;
                }
            }
            wl_output::Event::Description { description } => {
                log::info!("Output description: {}", description);
            }
            wl_output::Event::Scale { factor } => {
                log::info!("Output scale factor for {:?}: {}", proxy, factor);
                if let Some(info) = state.get_output_info_mut(proxy) {
                    info.scale = factor;
                }
            }
            wl_output::Event::Done => {
                state.create_bar_for_output(proxy, qh);
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrLayerShellV1, ()> for CrankyState {
    fn event(
        _: &mut Self,
        _: &ZwlrLayerShellV1,
        _: zwlr_layer_shell_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wayland_client::protocol::wl_surface::WlSurface, ()> for CrankyState {
    fn event(
        _state: &mut Self,
        _proxy: &wayland_client::protocol::wl_surface::WlSurface,
        _event: wayland_client::protocol::wl_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wayland_client::protocol::wl_shm_pool::WlShmPool, ()> for CrankyState {
    fn event(
        _: &mut Self,
        _: &wayland_client::protocol::wl_shm_pool::WlShmPool,
        _: wayland_client::protocol::wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wayland_client::protocol::wl_buffer::WlBuffer, ()> for CrankyState {
    fn event(
        _: &mut Self,
        _: &wayland_client::protocol::wl_buffer::WlBuffer,
        _: wayland_client::protocol::wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for CrankyState {
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
            height: _,
        } = event
        {
            proxy.ack_configure(serial);

            if let Some(bar) = state.bars.iter_mut().find(|b| b.layer_surface() == proxy) {
                bar.set_configured();
                if width > 0
                    && let Some(shm) = &state.shm {
                        bar.set_width(shm, width, qh);
                    }

                let bar_config = if Some(bar.monitor_name()) == state.focused_monitor.as_deref() {
                    state.config.bar().clone()
                } else {
                    state.config.bar().as_unfocused()
                };

                bar.render(
                    &state.config,
                    &bar_config,
                    &state.registry,
                    &mut state.render_context,
                    &state.error_message,
                    qh,
                );
            }
        }
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

     #[test]
    fn test_wayland_manager_new_fail() {
        // Without WAYLAND_DISPLAY, it should fail
        let old_val = std::env::var_os("WAYLAND_DISPLAY");
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
        }

        let config = Config::default();
        let res = WaylandManager::new(config);
        assert!(res.is_err());

        if let Some(val) = old_val {
            unsafe {
                std::env::set_var("WAYLAND_DISPLAY", val);
            }
        }
    }

    #[test]
    fn test_output_info_getters() {
        unsafe {
            let mut info = std::mem::MaybeUninit::<OutputInfo>::uninit().assume_init();
            std::ptr::write(&mut info.id, 1);
            std::ptr::write(&mut info.name, "HDMI-A-1".to_string());
            std::ptr::write(&mut info.scale, 1);

            assert_eq!(info.name(), "HDMI-A-1");
            assert_eq!(info.scale(), 1);

            std::mem::forget(info);
        }
    }

    #[test]
    fn test_tick_cadence_timebased() {
        let cadence = tick_cadence(&crate::config::RenderingMode::Timebased { duration_ms: 100 });
        assert_eq!(
            cadence,
            TickCadence::Sleep(std::time::Duration::from_millis(100))
        );
    }

    #[test]
    fn test_tick_cadence_timebased_zero_is_clamped() {
        let cadence = tick_cadence(&crate::config::RenderingMode::Timebased { duration_ms: 0 });
        assert_eq!(
            cadence,
            TickCadence::Sleep(std::time::Duration::from_millis(1))
        );
    }

    #[test]
    fn test_tick_cadence_immediate_with_limit() {
        let cadence = tick_cadence(&crate::config::RenderingMode::Immediate {
            fps_limit: Some(60),
        });
        assert_eq!(
            cadence,
            TickCadence::Sleep(std::time::Duration::from_secs_f64(1.0 / 60.0))
        );
    }

    #[test]
    fn test_tick_cadence_immediate_unlimited() {
        let cadence = tick_cadence(&crate::config::RenderingMode::Immediate { fps_limit: None });
        assert_eq!(cadence, TickCadence::Yield);
    }

    #[test]
    fn test_tick_cadence_immediate_zero_is_unlimited() {
        let cadence = tick_cadence(&crate::config::RenderingMode::Immediate { fps_limit: Some(0) });
        assert_eq!(cadence, TickCadence::Yield);
    }

    #[test]
    fn test_wayland_fd_as_raw_fd() {
        let fd = WaylandFd(42);
        assert_eq!(fd.as_raw_fd(), 42);
    }

    #[test]
    fn test_cranky_state_add_output() {
        unsafe {
            let mut state = CrankyState {
                registry: ModuleRegistry::new(),
                config: Config::default(),
                render_context: crate::render::RenderContext::new(),
                error_message: None,
                compositor: None,
                shm: None,
                layer_shell: None,
                outputs: Vec::new(),
                bars: Vec::new(),
                hyprland_provider: Box::new(crate::core::hyprland::RealHyprlandProvider),
                focused_monitor: None,
            };

            let output = std::mem::MaybeUninit::<WlOutput>::uninit().assume_init();
            state.add_output(7, output);
            assert_eq!(state.outputs.len(), 1);
            assert_eq!(state.outputs[0].id, 7);
            assert_eq!(state.outputs[0].scale, 1);
            assert!(state.outputs[0].name.is_empty());

            std::mem::forget(state);
        }
    }

    #[test]
    fn test_cranky_state_remove_output_no_match() {
        let mut state = CrankyState {
            registry: ModuleRegistry::new(),
            config: Config::default(),
            render_context: crate::render::RenderContext::new(),
            error_message: None,
            compositor: None,
            shm: None,
            layer_shell: None,
            outputs: Vec::new(),
            bars: Vec::new(),
            hyprland_provider: Box::new(crate::core::hyprland::RealHyprlandProvider),
            focused_monitor: None,
        };

        state.remove_output(999);
        assert!(state.outputs.is_empty());
    }

    #[test]
    fn test_cranky_state_globals_none_when_missing_any_global() {
        let state = CrankyState {
            registry: ModuleRegistry::new(),
            config: Config::default(),
            render_context: crate::render::RenderContext::new(),
            error_message: None,
            compositor: None,
            shm: None,
            layer_shell: None,
            outputs: Vec::new(),
            bars: Vec::new(),
            hyprland_provider: Box::new(crate::core::hyprland::RealHyprlandProvider),
            focused_monitor: None,
        };

        assert!(state.globals().is_none());
    }

    #[test]
    fn test_handle_periodic_update_no_redraw_with_empty_state() {
        let mut provider = crate::core::hyprland::MockHyprlandProvider::new();
        provider
            .expect_get_workspaces()
            .times(1)
            .returning(|| Ok(Vec::new()));
        provider
            .expect_get_monitors()
            .times(1)
            .returning(|| Ok(Vec::new()));

        let state = CrankyState {
            registry: ModuleRegistry::new(),
            config: Config::default(),
            render_context: crate::render::RenderContext::new(),
            error_message: None,
            compositor: None,
            shm: None,
            layer_shell: None,
            outputs: Vec::new(),
            bars: Vec::new(),
            hyprland_provider: Box::new(provider),
            focused_monitor: None,
        };

        unsafe {
            let mut manager = WaylandManager {
                connection: std::mem::MaybeUninit::<Connection>::uninit().assume_init(),
                event_queue: std::mem::MaybeUninit::<EventQueue<CrankyState>>::uninit()
                    .assume_init(),
                state,
            };

            manager.handle_periodic_update();
            std::mem::forget(manager);
        }
    }

    #[test]
    fn test_handle_periodic_update_hyprland_errors_default_to_empty() {
        let mut provider = crate::core::hyprland::MockHyprlandProvider::new();
        provider
            .expect_get_workspaces()
            .times(1)
            .returning(|| Err(crate::core::hyprland::HyprError::NoInstance));
        provider
            .expect_get_monitors()
            .times(1)
            .returning(|| Err(crate::core::hyprland::HyprError::NoInstance));

        let state = CrankyState {
            registry: ModuleRegistry::new(),
            config: Config::default(),
            render_context: crate::render::RenderContext::new(),
            error_message: None,
            compositor: None,
            shm: None,
            layer_shell: None,
            outputs: Vec::new(),
            bars: Vec::new(),
            hyprland_provider: Box::new(provider),
            focused_monitor: None,
        };

        unsafe {
            let mut manager = WaylandManager {
                connection: std::mem::MaybeUninit::<Connection>::uninit().assume_init(),
                event_queue: std::mem::MaybeUninit::<EventQueue<CrankyState>>::uninit()
                    .assume_init(),
                state,
            };

            manager.handle_periodic_update();
            std::mem::forget(manager);
        }
    }

    #[test]
    fn test_create_bar_for_output_returns_when_output_not_registered() {
        unsafe {
            let mut state = CrankyState {
                registry: ModuleRegistry::new(),
                config: Config::default(),
                render_context: crate::render::RenderContext::new(),
                error_message: None,
                compositor: None,
                shm: None,
                layer_shell: None,
                outputs: Vec::new(),
                bars: Vec::new(),
                hyprland_provider: Box::new(crate::core::hyprland::RealHyprlandProvider),
                focused_monitor: None,
            };

            let output = std::mem::MaybeUninit::<WlOutput>::uninit().assume_init();
            let qh = std::mem::MaybeUninit::<QueueHandle<CrankyState>>::uninit().assume_init();
            state.create_bar_for_output(&output, &qh);
            assert!(state.bars.is_empty());
            std::mem::forget(state);
            std::mem::forget(output);
            std::mem::forget(qh);
        }
    }

    #[test]
    fn test_create_bar_for_output_returns_when_name_is_empty() {
        unsafe {
            let mut state = CrankyState {
                registry: ModuleRegistry::new(),
                config: Config::default(),
                render_context: crate::render::RenderContext::new(),
                error_message: None,
                compositor: None,
                shm: None,
                layer_shell: None,
                outputs: Vec::new(),
                bars: Vec::new(),
                hyprland_provider: Box::new(crate::core::hyprland::RealHyprlandProvider),
                focused_monitor: None,
            };

            let output = std::mem::MaybeUninit::<WlOutput>::uninit().assume_init();
            state.add_output(42, output);
            let qh = std::mem::MaybeUninit::<QueueHandle<CrankyState>>::uninit().assume_init();
            let output_ptr: *const WlOutput = &state.outputs[0].output;
            state.create_bar_for_output(&*output_ptr, &qh);
            assert!(state.bars.is_empty());
            std::mem::forget(state);
            std::mem::forget(qh);
        }
    }

    #[test]
    fn test_wl_output_event_name_updates_output_info() {
        unsafe {
            let mut state = CrankyState {
                registry: ModuleRegistry::new(),
                config: Config::default(),
                render_context: crate::render::RenderContext::new(),
                error_message: None,
                compositor: None,
                shm: None,
                layer_shell: None,
                outputs: Vec::new(),
                bars: Vec::new(),
                hyprland_provider: Box::new(crate::core::hyprland::RealHyprlandProvider),
                focused_monitor: None,
            };
            let output = std::mem::MaybeUninit::<WlOutput>::uninit().assume_init();
            state.add_output(1, output);

            let output_ptr: *const WlOutput = &state.outputs[0].output;
            let conn = std::mem::MaybeUninit::<Connection>::uninit().assume_init();
            let qh = std::mem::MaybeUninit::<QueueHandle<CrankyState>>::uninit().assume_init();

            <CrankyState as Dispatch<WlOutput, ()>>::event(
                &mut state,
                &*output_ptr,
                wl_output::Event::Name {
                    name: "HDMI-A-1".to_string(),
                },
                &(),
                &conn,
                &qh,
            );

            assert_eq!(state.outputs[0].name, "HDMI-A-1");
            std::mem::forget(state);
            std::mem::forget(conn);
            std::mem::forget(qh);
        }
    }

    #[test]
    fn test_wl_output_event_scale_updates_output_info() {
        unsafe {
            let mut state = CrankyState {
                registry: ModuleRegistry::new(),
                config: Config::default(),
                render_context: crate::render::RenderContext::new(),
                error_message: None,
                compositor: None,
                shm: None,
                layer_shell: None,
                outputs: Vec::new(),
                bars: Vec::new(),
                hyprland_provider: Box::new(crate::core::hyprland::RealHyprlandProvider),
                focused_monitor: None,
            };
            let output = std::mem::MaybeUninit::<WlOutput>::uninit().assume_init();
            state.add_output(1, output);

            let output_ptr: *const WlOutput = &state.outputs[0].output;
            let conn = std::mem::MaybeUninit::<Connection>::uninit().assume_init();
            let qh = std::mem::MaybeUninit::<QueueHandle<CrankyState>>::uninit().assume_init();

            <CrankyState as Dispatch<WlOutput, ()>>::event(
                &mut state,
                &*output_ptr,
                wl_output::Event::Scale { factor: 2 },
                &(),
                &conn,
                &qh,
            );

            assert_eq!(state.outputs[0].scale, 2);
            std::mem::forget(state);
            std::mem::forget(conn);
            std::mem::forget(qh);
        }
    }

    #[test]
    fn test_wl_output_event_description_and_done_paths() {
        unsafe {
            let mut state = CrankyState {
                registry: ModuleRegistry::new(),
                config: Config::default(),
                render_context: crate::render::RenderContext::new(),
                error_message: None,
                compositor: None,
                shm: None,
                layer_shell: None,
                outputs: Vec::new(),
                bars: Vec::new(),
                hyprland_provider: Box::new(crate::core::hyprland::RealHyprlandProvider),
                focused_monitor: None,
            };
            let output = std::mem::MaybeUninit::<WlOutput>::uninit().assume_init();
            state.add_output(1, output);
            let output_ptr: *const WlOutput = &state.outputs[0].output;
            let conn = std::mem::MaybeUninit::<Connection>::uninit().assume_init();
            let qh = std::mem::MaybeUninit::<QueueHandle<CrankyState>>::uninit().assume_init();

            <CrankyState as Dispatch<WlOutput, ()>>::event(
                &mut state,
                &*output_ptr,
                wl_output::Event::Description {
                    description: "Display".to_string(),
                },
                &(),
                &conn,
                &qh,
            );
            <CrankyState as Dispatch<WlOutput, ()>>::event(
                &mut state,
                &*output_ptr,
                wl_output::Event::Done,
                &(),
                &conn,
                &qh,
            );

            assert!(state.bars.is_empty());
            std::mem::forget(state);
            std::mem::forget(conn);
            std::mem::forget(qh);
        }
    }

    #[test]
    fn test_wl_registry_global_remove_without_matching_output() {
        unsafe {
            let mut state = CrankyState {
                registry: ModuleRegistry::new(),
                config: Config::default(),
                render_context: crate::render::RenderContext::new(),
                error_message: None,
                compositor: None,
                shm: None,
                layer_shell: None,
                outputs: Vec::new(),
                bars: Vec::new(),
                hyprland_provider: Box::new(crate::core::hyprland::RealHyprlandProvider),
                focused_monitor: None,
            };

            let registry = std::mem::MaybeUninit::<WlRegistry>::uninit().assume_init();
            let conn = std::mem::MaybeUninit::<Connection>::uninit().assume_init();
            let qh = std::mem::MaybeUninit::<QueueHandle<CrankyState>>::uninit().assume_init();
            <CrankyState as Dispatch<WlRegistry, ()>>::event(
                &mut state,
                &registry,
                wl_registry::Event::GlobalRemove { name: 11 },
                &(),
                &conn,
                &qh,
            );
            assert!(state.outputs.is_empty());
            std::mem::forget(state);
            std::mem::forget(registry);
            std::mem::forget(conn);
            std::mem::forget(qh);
        }
    }
}
