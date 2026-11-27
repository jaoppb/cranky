use super::BootError;
use crate::app::adapter_supervisor::AdapterSupervisor;
use crate::features::systray::adapters::SniAdapter;
use crate::features::systray::ports::SniPort;
use crate::shared::dbus::ports::DbusConnectionPort;
use crate::shared::env::domain::AppEnvironment;
use crate::shared::events::signals::{SignalHub, SignalKind};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::error;

/// The signals whose adapters the supervisor starts at boot, in the order it starts them.
const SUPERVISED_AT_BOOT: [SignalKind; 4] = [
    SignalKind::Mpris,
    SignalKind::Metrics,
    SignalKind::Hyprland,
    SignalKind::Time,
];

/// Opens the `DBus` connections (an unreachable bus is logged, not fatal),
/// builds the on-demand adapter supervisor around them and the hub, whose
/// configuration is still the initial one, and eagerly
/// `ensure`s every signal already active from the initial config load. Systray is handled separately: its
/// `start()` needs `&mut self` on the very instance `CrankyApp::run` later
/// drives via `SniPort::trigger_action`, so it isn't part of the supervisor
/// — see `SPEC.md`'s Phase 3 write-up.
pub(super) async fn init_secondary_adapters(
    hub: &Arc<SignalHub>,
    app_env: &Arc<AppEnvironment>,
    active_signals: &HashSet<SignalKind>,
) -> Result<
    (
        crate::app::state::BusAdapters<SniAdapter>,
        AdapterSupervisor,
    ),
    BootError,
> {
    let conn = connect_bus().await?;

    let dbus_manager =
        crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(conn.clone(), hub);

    let mut supervisor = AdapterSupervisor::new(hub.clone(), app_env.clone(), conn.clone());
    start_supervised_adapters(&mut supervisor, active_signals).await;

    let sni_adapter = start_systray(hub, &mut supervisor, active_signals).await;

    Ok((
        crate::app::state::BusAdapters::new(dbus_manager, sni_adapter),
        supervisor,
    ))
}

/// Opens the `DBus` connections the secondary adapters share.
///
/// # Errors
///
/// Returns [`BootError::DbusConnection`] when the connection cannot be set up.
async fn connect_bus() -> Result<Arc<dyn DbusConnectionPort>, BootError> {
    let conn_adapter = crate::shared::dbus::adapters::connection::ZbusConnectionAdapter::new()
        .connect()
        .await
        .map_err(BootError::DbusConnection)?;
    Ok(Arc::new(conn_adapter))
}

/// Asks `supervisor` to start, in [`SUPERVISED_AT_BOOT`] order, the adapter of every supervised
/// signal in `active_signals`.
async fn start_supervised_adapters(
    supervisor: &mut AdapterSupervisor,
    active_signals: &HashSet<SignalKind>,
) {
    for kind in SUPERVISED_AT_BOOT {
        if active_signals.contains(&kind) {
            supervisor.ensure(kind).await;
        }
    }
}

/// Builds the SNI adapter and, when some module subscribes to the systray, marks it started
/// on `supervisor` and starts its watcher (a watcher that cannot start is logged).
async fn start_systray(
    hub: &Arc<SignalHub>,
    supervisor: &mut AdapterSupervisor,
    active_signals: &HashSet<SignalKind>,
) -> SniAdapter {
    let mut sni_adapter = SniAdapter::new(hub.clone());
    if claim_systray(supervisor, active_signals)
        && let Err(error) = sni_adapter.start().await
    {
        log_sni_start_failed(&error);
    }
    sni_adapter
}

/// Logs that the SNI watcher could not start.
fn log_sni_start_failed(error: &impl std::fmt::Debug) {
    error!("Failed to start SNI Watcher: {error:?}");
}

/// Whether the systray is active, marking it started on `supervisor` when it is, so the
/// supervisor never starts a second watcher.
fn claim_systray(supervisor: &mut AdapterSupervisor, active_signals: &HashSet<SignalKind>) -> bool {
    let active = active_signals.contains(&SignalKind::Systray);
    if active {
        supervisor.mark_started(SignalKind::Systray);
    }
    active
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::Config;
    use crate::shared::dbus::ports::DisconnectedBus;

    /// A supervisor over a fresh hub and a bus that refuses every call.
    fn supervisor(home: &crate::test_utils::TestHome) -> AdapterSupervisor {
        let hub = Arc::new(SignalHub::new(Config::default()));
        AdapterSupervisor::new(hub, home.env(), Arc::new(DisconnectedBus))
    }

    #[test]
    fn test_boot_starts_only_the_active_supervised_signals() {
        crate::test_utils::block_on(async {
            let home = crate::test_utils::TestHome::new("secondary");
            let mut supervisor = supervisor(&home);
            let active = HashSet::from([SignalKind::Metrics, SignalKind::Time, SignalKind::DBus]);

            start_supervised_adapters(&mut supervisor, &active).await;

            for kind in [SignalKind::Metrics, SignalKind::Time] {
                assert!(supervisor.has_started(kind), "{kind:?}");
            }
            for kind in [
                SignalKind::Mpris,
                SignalKind::Hyprland,
                SignalKind::Systray,
                SignalKind::DBus,
            ] {
                assert!(!supervisor.has_started(kind), "{kind:?}");
            }
        });
    }

    #[test]
    fn test_boot_supervises_mpris_metrics_hyprland_and_time_in_that_order() {
        assert_eq!(
            SUPERVISED_AT_BOOT,
            [
                SignalKind::Mpris,
                SignalKind::Metrics,
                SignalKind::Hyprland,
                SignalKind::Time,
            ]
        );
    }

    #[test]
    fn test_an_active_systray_is_claimed_from_the_supervisor() {
        let home = crate::test_utils::TestHome::new("secondary");
        let mut supervisor = supervisor(&home);

        assert!(claim_systray(
            &mut supervisor,
            &HashSet::from([SignalKind::Systray])
        ));
        assert!(supervisor.has_started(SignalKind::Systray));
    }

    #[test]
    fn test_an_inactive_systray_is_left_alone() {
        crate::test_utils::block_on(async {
            let home = crate::test_utils::TestHome::new("secondary");
            let hub = Arc::new(SignalHub::new(Config::default()));
            let mut supervisor = supervisor(&home);
            let active = HashSet::from([SignalKind::Time]);

            assert!(!claim_systray(&mut supervisor, &active));
            let _sni = start_systray(&hub, &mut supervisor, &active).await;
            assert!(!supervisor.has_started(SignalKind::Systray));
        });
    }
}
