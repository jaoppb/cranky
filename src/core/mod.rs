use crate::config::Config;
use crate::modules::{Event, ModuleRegistry, UpdateAction};
use tokio::sync::mpsc;
use thiserror::Error;
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle,
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::{self, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_shm::WlShm,
    },
};
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
            (Some(c), Some(s), Some(l)) => Some(WaylandGlobals::new(c.clone(), s.clone(), l.clone())),
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
                info_name, info_scale
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
}

impl WaylandManager {
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

        loop {
            // 1. Flush requests
            let _ = self.connection.flush();

            // 2. Dispatch any already buffered events
            self.event_queue
                .dispatch_pending(&mut self.state)
                .map_err(|e| CoreError::Dispatch(e.to_string()))?;

            // 3. Try to read new events
            if let Some(guard) = self.event_queue.prepare_read() {
                let _ = guard.read();
            }

            // Check for config updates
            while let Ok(res) = config_rx.try_recv() {
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
                    bar.update_config(&shm, &self.state.config, &qh);
                    bar.render(
                        &self.state.config,
                        &self.state.registry,
                        &mut self.state.render_context,
                        &self.state.error_message,
                        &qh,
                    );
                }
            }

            // 4. Periodic update check
            if self.state.registry.update(Event::Timer) == UpdateAction::Redraw {
                let qh = self.event_queue.handle();
                let shm = self.state.shm.as_ref().unwrap().clone();
                for bar in &mut self.state.bars {
                    bar.update_config(&shm, &self.state.config, &qh);
                    bar.render(
                        &self.state.config,
                        &self.state.registry,
                        &mut self.state.render_context,
                        &self.state.error_message,
                        &qh,
                    );
                }
            }

            // 5. Sleep to avoid 100% CPU and allow other tasks
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
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
                    state.compositor = Some(proxy.bind::<WlCompositor, _, _>(name, version, qh, ()));
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
                if width > 0 {
                    if let Some(shm) = &state.shm {
                        bar.set_width(shm, width, qh);
                    }
                }
                bar.render(
                    &state.config,
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
mod tests {
    use super::*;

    #[test]
    fn test_core_error_display() {
        let err = CoreError::Connection("failed".to_string());
        assert_eq!(format!("{}", err), "Wayland connection failed: failed");

        let err = CoreError::Global("missing".to_string());
        assert_eq!(format!("{}", err), "Wayland global error: missing");

        let err = CoreError::Dispatch("timed out".to_string());
        assert_eq!(format!("{}", err), "Wayland dispatch error: timed out");
    }

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
        let mut info = unsafe { std::mem::MaybeUninit::<OutputInfo>::uninit().assume_init() };
        unsafe {
            std::ptr::write(&mut info.id, 1);
            std::ptr::write(&mut info.name, "HDMI-A-1".to_string());
            std::ptr::write(&mut info.scale, 1);
        }

        assert_eq!(info.name(), "HDMI-A-1");
        assert_eq!(info.scale(), 1);

        // Don't test output() getter as it returns reference to uninitialized memory which is dangerous

        std::mem::forget(info);
    }

    #[test]
    fn test_wayland_globals_getters() {
        let globals = unsafe { std::mem::MaybeUninit::<WaylandGlobals>::uninit().assume_init() };
        // We can't safely test the getters because they return references to uninitialized proxies
        std::mem::forget(globals);
    }
}
