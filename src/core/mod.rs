use crate::config::Config;
use crate::modules::{Event, ModuleRegistry, UpdateAction};
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
    fn get_output_info(&mut self, output: &WlOutput) -> &mut OutputInfo {
        if let Some(index) = self.outputs.iter().position(|i| &i.output == output) {
            &mut self.outputs[index]
        } else {
            self.outputs.push(OutputInfo {
                output: output.clone(),
                name: String::new(),
                scale: 1,
            });
            self.outputs.last_mut().unwrap()
        }
    }
}

pub struct CrankyState {
    registry: ModuleRegistry,
    config: Config,
    render_context: crate::render::RenderContext,

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

    pub async fn run(mut self) -> Result<()> {
        // Initial dispatch to get globals and output info
        self.event_queue
            .roundtrip(&mut self.state)
            .map_err(|e| CoreError::Dispatch(e.to_string()))?;
        self.event_queue
            .roundtrip(&mut self.state)
            .map_err(|e| CoreError::Dispatch(e.to_string()))?;

        let qh = self.event_queue.handle();

        let compositor = self
            .state
            .compositor
            .as_ref()
            .ok_or_else(|| CoreError::Global("Missing wl_compositor".to_string()))?;
        let shm = self
            .state
            .shm
            .as_ref()
            .ok_or_else(|| CoreError::Global("Missing wl_shm".to_string()))?;
        let layer_shell = self
            .state
            .layer_shell
            .as_ref()
            .ok_or_else(|| CoreError::Global("Missing zwlr_layer_shell_v1".to_string()))?;

        let globals = WaylandGlobals::new(compositor.clone(), shm.clone(), layer_shell.clone());

        for info in &self.state.outputs {
            let bar = bar::Bar::new(info, &globals, &self.state.config, &qh)
                .map_err(|e| CoreError::Global(e.to_string()))?;
            self.state.bars.push(bar);
        }

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

            // 4. Periodic update check
            if self.state.registry.update(Event::Timer) == UpdateAction::Redraw {
                let qh = self.event_queue.handle();
                for bar in &mut self.state.bars {
                    bar.render(
                        &self.state.config,
                        &self.state.registry,
                        &mut self.state.render_context,
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
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match &interface[..] {
                "wl_compositor" => {
                    state.compositor =
                        Some(proxy.bind::<WlCompositor, _, _>(name, version, qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(proxy.bind::<WlShm, _, _>(name, version, qh, ()));
                }
                "wl_output" => {
                    proxy.bind::<WlOutput, _, _>(name, version, qh, ());
                }
                "zwlr_layer_shell_v1" => {
                    state.layer_shell =
                        Some(proxy.bind::<ZwlrLayerShellV1, _, _>(name, version, qh, ()));
                }
                _ => {}
            }
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
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_output::Event::Name { name } => {
                log::info!("Output name: {}", name);
                state.get_output_info(proxy).name = name;
            }
            wl_output::Event::Description { description } => {
                log::info!("Output description: {}", description);
            }
            wl_output::Event::Scale { factor } => {
                log::info!("Output scale factor for {:?}: {}", proxy, factor);
                state.get_output_info(proxy).scale = factor;
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
