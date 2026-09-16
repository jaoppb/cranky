use crate::features::metrics::adapters::SysinfoAdapter;
use crate::features::metrics::domain::MetricsConfig;
use crate::features::workspaces::adapters::hyprland::HyprlandAdapter;
use crate::shared::dbus::ports::DbusConnectionPort;
use crate::shared::env::domain::AppEnvironment;
use crate::shared::events::signals::{SignalHub, SignalKind};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::Instrument;

/// Starts secondary adapters (Mpris, Metrics, Hyprland, Time) on demand
/// instead of only once at boot, so a module spawned mid-session gets its
/// signal source without a restart.
///
/// `Systray` and `DBus` are not covered: the `DBus` connection itself is
/// established once at boot and stays fatal if it fails (decision 3), and
/// on-demand Systray needs a live handle shared with the run loop's own
/// `trigger_action` path, which `ensure` alone can't provide without a
/// larger ownership change — see `SPEC.md`'s Phase 3 write-up.
pub struct AdapterSupervisor {
    hub: Arc<SignalHub>,
    app_env: Arc<AppEnvironment>,
    metrics_config: MetricsConfig,
    dbus_conn: Arc<dyn DbusConnectionPort>,
    started: HashSet<SignalKind>,
}

impl AdapterSupervisor {
    #[must_use]
    pub fn new(
        hub: Arc<SignalHub>,
        app_env: Arc<AppEnvironment>,
        metrics_config: MetricsConfig,
        dbus_conn: Arc<dyn DbusConnectionPort>,
    ) -> Self {
        Self {
            hub,
            app_env,
            metrics_config,
            dbus_conn,
            started: HashSet::new(),
        }
    }

    /// Marks `kind` as already running at boot, so a later `ensure` for the
    /// same kind is a no-op instead of starting a second watcher.
    pub fn mark_started(&mut self, kind: SignalKind) {
        self.started.insert(kind);
    }

    /// Starts the adapter behind `kind` the first time it's asked for, and
    /// does nothing on every later call — including a failed first attempt,
    /// which is treated as terminal for the session rather than retried on
    /// every render (see `SPEC.md`'s Risks: a signal whose adapter refuses
    /// to start leaves its subscriber silently blank).
    pub async fn ensure(&mut self, kind: SignalKind) {
        if !self.started.insert(kind) {
            return;
        }

        match kind {
            SignalKind::Mpris => {
                let adapter = crate::features::mpris::adapters::zbus::ZbusMprisAdapter::new(
                    self.dbus_conn.clone(),
                    &self.hub,
                );
                if let Err(e) = adapter.start_watching().await {
                    tracing::error!("Failed to start MPRIS watcher on demand: {e}");
                }
            }
            SignalKind::Metrics => {
                let adapter = SysinfoAdapter::new(self.metrics_config.clone(), self.hub.clone());
                adapter.start();
            }
            SignalKind::Hyprland => {
                let adapter = HyprlandAdapter::new(self.app_env.clone());
                let hub = self.hub.clone();
                tokio::spawn(
                    async move {
                        adapter.run(hub).await;
                    }
                    .instrument(tracing::info_span!("hyprland_adapter")),
                );
            }
            SignalKind::Time => {
                let hub = self.hub.clone();
                tokio::spawn(
                    async move {
                        loop {
                            let now = chrono::Local::now();
                            let ms_until_next_sec = 1000_u64
                                .saturating_sub(u64::from(now.timestamp_subsec_millis()));
                            tokio::time::sleep(std::time::Duration::from_millis(
                                ms_until_next_sec,
                            ))
                            .await;
                            let _ = hub.time_tx().send(chrono::Local::now());
                        }
                    }
                    .instrument(tracing::info_span!("time_adapter")),
                );
            }
            SignalKind::Systray | SignalKind::DBus => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::Config;
    use crate::shared::dbus::ports::MockDbusConnectionPort;

    fn test_env() -> Arc<AppEnvironment> {
        Arc::new(AppEnvironment::new(
            crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::RustLog::new(String::new()),
            None,
        ))
    }

    #[tokio::test]
    async fn test_ensure_metrics_is_idempotent() {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let mut supervisor = AdapterSupervisor::new(
            hub,
            test_env(),
            MetricsConfig::default(),
            Arc::new(MockDbusConnectionPort::new()),
        );

        assert!(!supervisor.started.contains(&SignalKind::Metrics));
        supervisor.ensure(SignalKind::Metrics).await;
        assert!(supervisor.started.contains(&SignalKind::Metrics));
        // Second call must not start a second poller — nothing observable
        // to assert on directly, but it must not panic or double-insert.
        supervisor.ensure(SignalKind::Metrics).await;
        assert_eq!(supervisor.started.len(), 1);
    }

    #[tokio::test]
    async fn test_mark_started_prevents_later_ensure() {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let mut supervisor = AdapterSupervisor::new(
            hub,
            test_env(),
            MetricsConfig::default(),
            Arc::new(MockDbusConnectionPort::new()),
        );

        supervisor.mark_started(SignalKind::Time);
        // No task should be spawned for a kind marked started at boot.
        supervisor.ensure(SignalKind::Time).await;
        assert_eq!(supervisor.started.len(), 1);
    }
}
