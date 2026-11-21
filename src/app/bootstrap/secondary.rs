use super::BootError;
use crate::app::adapter_supervisor::AdapterSupervisor;
use crate::features::systray::adapters::SniAdapter;
use crate::features::systray::ports::SniPort;
use crate::shared::env::domain::AppEnvironment;
use crate::shared::events::signals::{SignalHub, SignalKind};
use std::sync::Arc;
use tracing::error;

/// Opens the `DBus` connections (an unreachable bus is logged, not fatal),
/// builds the on-demand adapter supervisor around them, and eagerly `ensure`s every signal already
/// active from the initial config load. Systray is handled separately: its
/// `start()` needs `&mut self` on the very instance `CrankyApp::run` later
/// drives via `SniPort::trigger_action`, so it isn't part of the supervisor
/// — see `SPEC.md`'s Phase 3 write-up.
pub(super) async fn init_secondary_adapters(
    hub: &Arc<SignalHub>,
    app_env: &Arc<AppEnvironment>,
    metrics_config: &crate::features::metrics::domain::MetricsConfig,
    active_signals: &std::collections::HashSet<SignalKind>,
) -> Result<
    (
        crate::shared::dbus::subscription_manager::DbusSubscriptionManager,
        SniAdapter,
        AdapterSupervisor,
    ),
    BootError,
> {
    let conn_adapter = crate::shared::dbus::adapters::connection::ZbusConnectionAdapter::new()
        .connect()
        .await
        .map_err(BootError::DbusConnection)?;
    let conn: Arc<dyn crate::shared::dbus::ports::DbusConnectionPort> = Arc::new(conn_adapter);

    let dbus_manager =
        crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(conn.clone(), hub);

    let mut supervisor = AdapterSupervisor::new(
        hub.clone(),
        app_env.clone(),
        metrics_config.clone(),
        conn.clone(),
    );
    for kind in [
        SignalKind::Mpris,
        SignalKind::Metrics,
        SignalKind::Hyprland,
        SignalKind::Time,
    ] {
        if active_signals.contains(&kind) {
            supervisor.ensure(kind).await;
        }
    }

    let mut sni_adapter = SniAdapter::new(hub.clone());
    if active_signals.contains(&SignalKind::Systray) {
        supervisor.mark_started(SignalKind::Systray);
        if let Err(error) = sni_adapter.start().await {
            error!("Failed to start SNI Watcher: {error:?}");
        }
    }

    Ok((dbus_manager, sni_adapter, supervisor))
}
