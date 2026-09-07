#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Local;
    use futures_util::StreamExt;

    use crate::features::metrics::domain::MetricsState;
    use crate::features::systray::domain::SystrayState;
    use crate::shared::config::domain::Config;
    use crate::shared::dbus::domain::DBusState;
    use crate::shared::events::core::{PointerEvent, SurfaceKind};
    use crate::shared::events::signals::hub::SignalHub;
    use crate::shared::events::signals::hyprland::HyprlandState;
    use crate::shared::events::signals::kind::SignalKind;
    use crate::shared::primitives::geometry::Scale;
    use crate::shared::primitives::{ModuleId, MonitorId};

    #[tokio::test]
    async fn test_signal_hub_config_propagation() {
        let hub = SignalHub::new(Config::default());
        let config_rx = hub.config_rx();
        let config_tx = hub.config_tx();

        let new_config = Config::default();
        config_tx.send(new_config).unwrap();

        assert!(config_rx.has_changed().unwrap());
    }

    #[tokio::test]
    async fn test_signal_hub_hyprland_propagation() {
        let hub = SignalHub::new(Config::default());
        let hypr_rx = hub.hyprland_rx();
        let hypr_tx = hub.hyprland_tx();

        let new_state = HyprlandState::new(
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
            None,
        );
        hypr_tx.send(new_state).unwrap();

        assert!(hypr_rx.has_changed().unwrap());
    }

    #[tokio::test]
    async fn test_signal_hub_time_propagation() {
        let hub = SignalHub::new(Config::default());
        let mut time_rx = hub.time_rx();
        let time_tx = hub.time_tx();

        let now = Local::now();
        time_tx.send(now).unwrap();

        assert!(time_rx.changed().await.is_ok());
    }

    #[tokio::test]
    async fn test_signal_hub_other_propagation() {
        let hub = SignalHub::new(Config::default());

        let dbus_tx = hub.dbus_tx();
        let mut dbus_rx = hub.dbus_rx();
        dbus_tx.send(DBusState::default()).unwrap();
        assert!(dbus_rx.changed().await.is_ok());

        let systray_tx = hub.systray_tx();
        let mut systray_rx = hub.systray_rx();
        systray_tx.send(SystrayState::default()).unwrap();
        assert!(systray_rx.changed().await.is_ok());

        let metrics_tx = hub.metrics_tx();
        let mut metrics_rx = hub.metrics_rx();
        metrics_tx.send(MetricsState::default()).unwrap();
        assert!(metrics_rx.changed().await.is_ok());

        let ptr_tx = hub.pointer_tx();
        let mut ptr_rx = hub.pointer_rx();
        ptr_tx
            .send((
                ModuleId::new(1),
                MonitorId::new("1"),
                crate::shared::events::core::InteractionEvent::Pointer(
                    PointerEvent::PointerLeave {
                        surface: SurfaceKind::Bar,
                    },
                ),
            ))
            .unwrap();
        assert!(ptr_rx.recv().await.is_ok());
    }

    #[tokio::test]
    async fn test_signal_hub_subscribe_streams() {
        let hub = SignalHub::new(Config::default());
        let subs = vec![
            SignalKind::Time,
            SignalKind::Systray,
            SignalKind::DBus,
            SignalKind::Time,
        ];
        let mut stream = hub.subscribe_streams(&subs);

        let first = stream.next().await;
        assert!(first.is_some());
        let second = stream.next().await;
        assert!(second.is_some());
        let third = stream.next().await;
        assert!(third.is_some());

        hub.time_tx().send(Local::now()).unwrap();
        let sig = stream.next().await;
        assert_eq!(sig, Some(SignalKind::Time));
    }

    #[tokio::test]
    async fn test_signal_to_stream_all_kinds() {
        let hub = SignalHub::new(Config::default());
        let kinds = [
            SignalKind::Time,
            SignalKind::Hyprland,
            SignalKind::DBus,
            SignalKind::Systray,
            SignalKind::Metrics,
            SignalKind::Mpris,
        ];

        for kind in kinds {
            let mut stream = hub.signal_to_stream(&kind);
            let initial = stream.next().await;
            assert_eq!(initial, Some(kind));
        }
    }

    #[tokio::test]
    async fn test_signal_hub_monitor_scales_propagation() {
        let hub = SignalHub::new(Config::default());
        let mut scales_rx = hub.monitor_scales_rx();
        let scales_tx = hub.monitor_scales_tx();

        let mut map = HashMap::new();
        map.insert(MonitorId::new("DP-1"), Scale::new(2.0));
        scales_tx.send(map.clone()).unwrap();

        assert!(scales_rx.changed().await.is_ok());
        assert_eq!(
            scales_rx.borrow().get(&MonitorId::new("DP-1")),
            Some(&Scale::new(2.0))
        );
    }
}
