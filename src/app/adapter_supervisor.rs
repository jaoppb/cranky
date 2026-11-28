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
pub(super) struct AdapterSupervisor {
    hub: Arc<SignalHub>,
    app_env: Arc<AppEnvironment>,
    /// The `[metrics]` table of the configuration the hub holds when the
    /// supervisor is built, which at boot is the initial configuration.
    metrics_config: MetricsConfig,
    dbus_conn: Arc<dyn DbusConnectionPort>,
    started: HashSet<SignalKind>,
}

impl AdapterSupervisor {
    #[must_use]
    pub(crate) fn new(
        hub: Arc<SignalHub>,
        app_env: Arc<AppEnvironment>,
        dbus_conn: Arc<dyn DbusConnectionPort>,
    ) -> Self {
        let metrics_config = hub.config_rx().borrow().metrics().clone();
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
    pub(crate) fn mark_started(&mut self, kind: SignalKind) {
        self.started.insert(kind);
    }

    /// Whether `kind` was started, or marked started, already.
    #[cfg(test)]
    pub(crate) fn has_started(&self, kind: SignalKind) -> bool {
        self.started.contains(&kind)
    }

    /// Starts the adapter behind `kind` the first time it's asked for, and
    /// does nothing on every later call — including a failed first attempt,
    /// which is treated as terminal for the session rather than retried on
    /// every render (see `SPEC.md`'s Risks: a signal whose adapter refuses
    /// to start leaves its subscriber silently blank).
    pub(crate) async fn ensure(&mut self, kind: SignalKind) {
        if !self.started.insert(kind) {
            return;
        }

        match kind {
            SignalKind::Mpris => self.start_mpris().await,
            SignalKind::Metrics => self.start_metrics(),
            SignalKind::Hyprland => self.spawn_hyprland(),
            SignalKind::Time => self.spawn_time(),
            SignalKind::Systray | SignalKind::DBus => {}
        }
    }

    /// Starts the MPRIS watcher on the shared bus, logging a watcher that cannot start.
    async fn start_mpris(&self) {
        let adapter = crate::features::mpris::adapters::zbus::ZbusMprisAdapter::new(
            self.dbus_conn.clone(),
            &self.hub,
        );
        if let Err(error) = adapter.start_watching().await {
            log_mpris_start_failed(&error);
        }
    }

    /// Starts the metrics sampler with the `[metrics]` settings the supervisor was built with.
    fn start_metrics(&self) {
        let adapter = SysinfoAdapter::new(self.metrics_config.clone(), self.hub.clone());
        adapter.start();
    }

    /// Spawns the Hyprland event loop that feeds the hub, in its own `hyprland_adapter` span.
    fn spawn_hyprland(&self) {
        let adapter = HyprlandAdapter::new(self.app_env.clone());
        let hub = self.hub.clone();
        tokio::spawn(
            async move {
                adapter.run(hub).await;
            }
            .instrument(tracing::info_span!("hyprland_adapter")),
        );
    }

    /// Spawns the time adapter that feeds the hub, in its own `time_adapter` span.
    fn spawn_time(&self) {
        let hub = self.hub.clone();
        tokio::spawn(super::time_adapter::run(hub).instrument(tracing::info_span!("time_adapter")));
    }
}

/// Logs that the MPRIS watcher could not start on demand.
fn log_mpris_start_failed(error: &crate::shared::dbus::ports::DbusConnectionError) {
    tracing::error!("Failed to start MPRIS watcher on demand: {error}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::Config;
    use crate::shared::dbus::ports::DisconnectedBus;

    fn test_env() -> Arc<AppEnvironment> {
        Arc::new(AppEnvironment::new(
            crate::shared::env::domain::BaseDirectories::new(
                crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/tmp")),
                crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
                crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
            ),
            crate::shared::env::domain::RustLog::new(String::new()),
            None,
        ))
    }

    #[test]
    fn test_ensure_metrics_is_idempotent() {
        crate::test_utils::block_on(async {
            let hub = Arc::new(SignalHub::new(Config::default()));
            let mut supervisor = AdapterSupervisor::new(hub, test_env(), Arc::new(DisconnectedBus));

            assert!(!supervisor.started.contains(&SignalKind::Metrics));
            supervisor.ensure(SignalKind::Metrics).await;
            assert!(supervisor.started.contains(&SignalKind::Metrics));
            // Second call must not start a second poller — nothing observable
            // to assert on directly, but it must not panic or double-insert.
            supervisor.ensure(SignalKind::Metrics).await;
            assert_eq!(supervisor.started.len(), 1);
        });
    }

    #[test]
    fn test_mark_started_prevents_later_ensure() {
        crate::test_utils::block_on(async {
            let hub = Arc::new(SignalHub::new(Config::default()));
            let mut supervisor = AdapterSupervisor::new(hub, test_env(), Arc::new(DisconnectedBus));

            supervisor.mark_started(SignalKind::Time);
            // No task should be spawned for a kind marked started at boot.
            supervisor.ensure(SignalKind::Time).await;
            assert_eq!(supervisor.started.len(), 1);
        });
    }

    /// An on-demand metrics sampler is built with the `[metrics]` settings of the configuration
    /// the bar started with.
    #[test]
    fn test_supervisor_keeps_the_configured_metrics_settings() {
        let metrics: MetricsConfig = toml::from_str("update_interval_ms = 2500").unwrap();
        let hub = Arc::new(SignalHub::new(Config::default().with_metrics(metrics)));

        let supervisor = AdapterSupervisor::new(hub, test_env(), Arc::new(DisconnectedBus));

        assert_eq!(supervisor.metrics_config.update_interval_ms().value(), 2500);
    }

    /// A supervisor over a hub with the default configuration and a bus that refuses every call.
    fn disconnected_supervisor(hub: &Arc<SignalHub>) -> AdapterSupervisor {
        AdapterSupervisor::new(hub.clone(), test_env(), Arc::new(DisconnectedBus))
    }

    /// The tasks alive on the current runtime, to tell whether `ensure` spawned one.
    fn alive_tasks() -> usize {
        tokio::runtime::Handle::current()
            .metrics()
            .num_alive_tasks()
    }

    /// Metrics, Hyprland and Time each start as one spawned task. Nothing awaits after the
    /// Hyprland one, so it is dropped with the runtime before it ever polls the real socket.
    #[test]
    fn test_ensure_spawns_one_task_per_polled_adapter() {
        crate::test_utils::block_on(async {
            let hub = Arc::new(SignalHub::new(Config::default()));
            let mut supervisor = disconnected_supervisor(&hub);
            let before = alive_tasks();

            supervisor.ensure(SignalKind::Metrics).await;
            assert_eq!(alive_tasks().checked_sub(before), Some(1), "metrics");
            supervisor.ensure(SignalKind::Time).await;
            assert_eq!(alive_tasks().checked_sub(before), Some(2), "time");
            supervisor.ensure(SignalKind::Hyprland).await;
            assert_eq!(alive_tasks().checked_sub(before), Some(3), "hyprland");
        });
    }

    #[test]
    fn test_ensure_systray_and_dbus_start_nothing() {
        crate::test_utils::block_on(async {
            let hub = Arc::new(SignalHub::new(Config::default()));
            let mut supervisor = disconnected_supervisor(&hub);
            let before = alive_tasks();

            supervisor.ensure(SignalKind::Systray).await;
            supervisor.ensure(SignalKind::DBus).await;

            assert_eq!(alive_tasks(), before);
            assert!(supervisor.started.contains(&SignalKind::Systray));
            assert!(supervisor.started.contains(&SignalKind::DBus));
        });
    }

    #[test]
    fn test_ensure_time_feeds_the_hub() {
        crate::test_utils::block_on(async {
            let hub = Arc::new(SignalHub::new(Config::default()));
            let mut time_rx = hub.time_rx();
            let mut supervisor = disconnected_supervisor(&hub);

            supervisor.ensure(SignalKind::Time).await;

            tokio::time::timeout(std::time::Duration::from_secs(3), time_rx.changed())
                .await
                .expect("a tick within three seconds")
                .expect("the hub is alive");
        });
    }

    /// Asks a supervisor over a disconnected bus for MPRIS, and asserts the kind counts as started.
    async fn ensure_mpris_and_check_it_counts_as_started() {
        let hub = Arc::new(SignalHub::new(Config::default()));
        let mut supervisor = disconnected_supervisor(&hub);
        supervisor.ensure(SignalKind::Mpris).await;
        assert!(supervisor.started.contains(&SignalKind::Mpris));
    }

    /// An MPRIS watcher that cannot start is logged, and the kind still counts as started, so
    /// it is not retried.
    #[test]
    fn test_ensure_mpris_logs_a_watcher_that_cannot_start() {
        let logs = crate::features::module_runtime::test_support::capture_logs(|| {
            crate::test_utils::block_on(ensure_mpris_and_check_it_counts_as_started());
        });
        assert!(
            logs.contains(
                "ERROR cranky::app::adapter_supervisor: Failed to start MPRIS watcher on demand: "
            ),
            "{logs}"
        );
    }
}
