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
