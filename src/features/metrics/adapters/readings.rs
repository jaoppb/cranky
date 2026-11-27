//! The pure steps of a sampling pass: which device lists a config enables, and how the plain
//! values sysinfo reports become the CPU, network and temperature readings. They are kept
//! apart from the reading itself so each one can be checked with injected values.
use crate::features::metrics::domain::{
    CpuMetrics, CpuMode, CpuSample, DiskMode, MetricsConfig, NetworkMetrics, NetworkMode,
    NetworkSpeed, Temperature, TemperatureMode,
};

/// Whether `config` sets a network mode other than `disabled`.
pub(super) fn network_enabled(config: &MetricsConfig) -> bool {
    config
        .network()
        .is_some_and(|mode| *mode != NetworkMode::Disabled)
}

/// Whether `config` sets a disk mode other than `disabled`.
pub(super) fn disk_enabled(config: &MetricsConfig) -> bool {
    config
        .disk()
        .is_some_and(|mode| *mode != DiskMode::Disabled)
}

/// Whether `config` sets a temperature mode other than `disabled`.
pub(super) fn temperature_enabled(config: &MetricsConfig) -> bool {
    config
        .temperature()
        .is_some_and(|mode| *mode != TemperatureMode::Disabled)
}

/// The CPU usage `mode` asks for, from the overall usage and each core's usage as sysinfo
/// reports them. The core count `mode` may scale by is the number of per-core readings, or 1
/// when that number does not fit a `u16`.
pub(super) fn cpu_metrics(global: f32, per_core: Vec<f32>, mode: &CpuMode) -> CpuMetrics {
    let nproc = f32::from(u16::try_from(per_core.len()).unwrap_or(1));
    CpuSample::new(global, nproc, per_core).normalize(mode)
}

/// The bytes all interfaces sent and received since the previous sample, from one
/// `(transmitted, received)` pair per interface, each total saturating at `u64::MAX`. Zero
/// both ways, without reading `pairs`, when `config` leaves network off.
pub(super) fn network_metrics(
    config: &MetricsConfig,
    pairs: impl IntoIterator<Item = (u64, u64)>,
) -> NetworkMetrics {
    let (tx, rx) = if network_enabled(config) {
        pairs
            .into_iter()
            .fold((0_u64, 0_u64), |(tx, rx), (sent, received)| {
                (tx.saturating_add(sent), rx.saturating_add(received))
            })
    } else {
        (0, 0)
    };
    NetworkMetrics::new(NetworkSpeed::new(tx), NetworkSpeed::new(rx))
}

/// The average of the sensor `readings` (degrees Celsius) in the scale `config` sets, absent
/// when temperature is not configured, is `disabled`, or no reading is usable.
pub(super) fn temperature(
    config: &MetricsConfig,
    readings: impl IntoIterator<Item = Option<f32>>,
) -> Option<Temperature> {
    config
        .temperature()
        .and_then(|mode| Temperature::from_readings(readings, mode))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::metrics::domain::CpuUsage;

    /// A config with only `key` set to `mode`, every other setting at its default.
    fn config_with(key: &str, mode: &str) -> MetricsConfig {
        serde_json::from_value(serde_json::json!({ key: mode })).unwrap()
    }

    /// `predicate` on a config without `key`, with `key` disabled, and with `key` `enabled`.
    fn gate(predicate: fn(&MetricsConfig) -> bool, key: &str, enabled: &str) -> [bool; 3] {
        [
            predicate(&MetricsConfig::default()),
            predicate(&config_with(key, "disabled")),
            predicate(&config_with(key, enabled)),
        ]
    }

    #[test]
    fn test_each_device_is_enabled_only_by_a_mode_other_than_disabled() {
        let expected = [false, false, true];
        assert_eq!(gate(network_enabled, "network", "tx_rx"), expected);
        assert_eq!(gate(disk_enabled, "disk", "absolute"), expected);
        assert_eq!(gate(disk_enabled, "disk", "percentual"), expected);
        assert_eq!(
            gate(temperature_enabled, "temperature", "celsius"),
            expected
        );
        assert_eq!(
            gate(temperature_enabled, "temperature", "fahrenheit"),
            expected
        );
    }

    /// The overall and per-core usages of `cpu_metrics` over three cores in `mode`.
    fn three_cores(mode: &CpuMode) -> (CpuUsage, Vec<CpuUsage>) {
        cpu_metrics(30.0, vec![10.0, 20.0, 60.0], mode).into_parts()
    }

    #[test]
    fn test_cpu_metrics_scales_by_the_core_count_in_each_mode() {
        let cores = |values: [f32; 3]| values.map(CpuUsage::new).to_vec();
        let as_reported = (CpuUsage::new(30.0), cores([10.0, 20.0, 60.0]));
        assert_eq!(three_cores(&CpuMode::Percentage0to100), as_reported);
        let scaled = (CpuUsage::new(90.0), cores([10.0, 20.0, 60.0]));
        assert_eq!(three_cores(&CpuMode::PercentageNproc), scaled);
        let zeroed = (CpuUsage::new(0.0), cores([0.0; 3]));
        assert_eq!(three_cores(&CpuMode::Disabled), zeroed);
    }

    /// The `(tx, rx)` totals of `network_metrics` for `pairs` under `config`.
    fn totals(config: &MetricsConfig, pairs: &[(u64, u64)]) -> (u64, u64) {
        let metrics = network_metrics(config, pairs.iter().copied());
        (metrics.tx().value(), metrics.rx().value())
    }

    #[test]
    fn test_network_metrics_sums_each_direction_when_enabled() {
        let pairs = [(100, 7), (20, 3000), (5, 0)];
        assert_eq!(
            totals(&config_with("network", "tx_rx"), &pairs),
            (125, 3007)
        );
        let overflowing = [(u64::MAX, 1), (1, 2)];
        let enabled = config_with("network", "tx_rx");
        assert_eq!(totals(&enabled, &overflowing), (u64::MAX, 3));
    }

    #[test]
    fn test_network_metrics_is_zero_when_network_is_off() {
        let pairs = [(100, 7), (20, 3000)];
        assert_eq!(totals(&MetricsConfig::default(), &pairs), (0, 0));
        assert_eq!(totals(&config_with("network", "disabled"), &pairs), (0, 0));
    }

    /// The `temperature` of `readings` under `config`.
    fn degrees(config: &MetricsConfig, readings: &[Option<f32>]) -> Option<Temperature> {
        temperature(config, readings.iter().copied())
    }

    #[test]
    fn test_temperature_averages_the_readings_in_the_configured_scale() {
        let readings = [Some(30.0), None, Some(50.0)];
        let celsius = config_with("temperature", "celsius");
        assert_eq!(degrees(&celsius, &readings), Some(Temperature::new(40.0)));
        let fahrenheit = config_with("temperature", "fahrenheit");
        assert_eq!(
            degrees(&fahrenheit, &readings),
            Some(Temperature::new(104.0))
        );
    }

    #[test]
    fn test_temperature_is_absent_when_temperature_is_off() {
        let readings = [Some(30.0), Some(50.0)];
        assert_eq!(degrees(&MetricsConfig::default(), &readings), None);
        let disabled = config_with("temperature", "disabled");
        assert_eq!(degrees(&disabled, &readings), None);
    }
}
