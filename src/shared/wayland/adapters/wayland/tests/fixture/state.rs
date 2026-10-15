//! The fixture's `WaylandState` builder.

use crate::shared::wayland::adapters::wayland::dispatch_pointer_button::PointerGrab;
use super::Fake;
use crate::features::metrics::domain::MetricsConfig;
use crate::shared::config::domain::{
    Config, ModulesConfig, PopupConfig, PopupEmbed, RenderingMode, RootConfig, TooltipConfig,
};
use crate::shared::env::domain::{AppEnvironment, HomeDir, RustLog, XdgCacheHome, XdgRuntimeDir};
use crate::shared::events::signals::SignalHub;
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

impl Fake {
    /// A `WaylandState` with every global bound and `popup.embed = embed`.
    pub fn state(&self, embed: PopupEmbed) -> WaylandState {
        let popup = PopupConfig::default();
        let config = Config::new(
            RootConfig::default(),
            ModulesConfig::default(),
            RenderingMode::default(),
            MetricsConfig::default(),
            TooltipConfig::default(),
            PopupConfig::new(popup.behavior(), popup.offset(), embed),
        );
        let tmp = || PathBuf::from("/tmp");
        let app_env = Arc::new(AppEnvironment::new(
            HomeDir::new(tmp()),
            XdgCacheHome::new(tmp()),
            XdgRuntimeDir::new(tmp()),
            RustLog::new(String::new()),
            None,
        ));
        let (command_tx, _command_rx) = tokio::sync::mpsc::channel(1);
        WaylandState {
            hub: Arc::new(SignalHub::new(config)),
            compositor: Some(self.compositor.clone()),
            subcompositor: Some(self.subcompositor.clone()),
            layer_shell: Some(self.layer_shell.clone()),
            xdg_wm_base: Some(self.xdg_wm_base.clone()),
            shm: Some(self.shm.clone()),
            outputs: Vec::new(),
            bars: Vec::new(),
            seat: None,
            pointer: None,
            command_tx,
            surface_to_id: HashMap::new(),
            pointer_surface: None,
            pointer_pos: (0.0, 0.0),
            pointer_target: None,
            pointer_grab: PointerGrab::default(),
            floating_surfaces: HashMap::new(),
            pending_children: HashMap::new(),
            last_button_serial: None,
            app_env,
        }
    }
}
