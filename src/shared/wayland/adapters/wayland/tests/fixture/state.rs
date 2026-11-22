//! The fixture's `WaylandState` builder.

use super::Fake;
use crate::shared::config::domain::{Config, PopupConfig, PopupEmbed};
use crate::shared::env::domain::{
    AppEnvironment, BaseDirectories, HomeDir, RustLog, XdgCacheHome, XdgRuntimeDir,
};
use crate::shared::events::signals::SignalHub;
use crate::shared::wayland::adapters::wayland::state::WaylandState;
use std::path::PathBuf;
use std::sync::Arc;

impl Fake {
    /// A `WaylandState` with every global bound and `popup.embed = embed`.
    pub(crate) fn state(&self, embed: PopupEmbed) -> WaylandState {
        let popup = PopupConfig::default();
        let config =
            Config::default().with_popup(PopupConfig::new(popup.behavior(), popup.offset(), embed));
        let app_env = environment_with_runtime_dir(PathBuf::from("/tmp"));
        let (command_tx, _command_rx) = tokio::sync::mpsc::channel(1);
        let mut state = WaylandState::new(Arc::new(SignalHub::new(config)), command_tx, app_env);
        state.compositor = Some(self.compositor.clone());
        state.subcompositor = Some(self.subcompositor.clone());
        state.layer_shell = Some(self.layer_shell.clone());
        state.xdg_wm_base = Some(self.xdg_wm_base.clone());
        state.shm = Some(self.shm.clone());
        state
    }
}

/// Points `state`'s runtime directory at one that does not exist, so creating
/// any shared-memory buffer fails.
pub(in crate::shared::wayland::adapters::wayland) fn lose_runtime_dir(state: &mut WaylandState) {
    state.app_env = environment_with_runtime_dir(PathBuf::from("/nonexistent/cranky-runtime"));
}

/// An environment whose home and cache are `/tmp` and whose runtime directory is `runtime_dir`.
pub(in crate::shared::wayland::adapters::wayland) fn environment_with_runtime_dir(
    runtime_dir: PathBuf,
) -> Arc<AppEnvironment> {
    let tmp = || PathBuf::from("/tmp");
    Arc::new(AppEnvironment::new(
        BaseDirectories::new(
            HomeDir::new(tmp()),
            XdgCacheHome::new(tmp()),
            XdgRuntimeDir::new(runtime_dir),
        ),
        RustLog::new(String::new()),
        None,
    ))
}
