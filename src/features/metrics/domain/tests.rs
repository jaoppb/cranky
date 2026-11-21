use super::command::{CreateMetricsCommand, MemoryMetrics, NetworkMetrics};
use super::config::{CpuMode, MetricsConfig, TemperatureMode};
use super::state::MetricsState;
use super::types::{
    CpuUsage, DiskMetric, DiskName, MemoryBytes, MountPoint, NetworkSpeed, Temperature,
};

#[test]
fn test_normalize_cpu_usage_0to100() {
    let global_cpu = 25.0;
    let nproc = 4.0;
    let per_core = vec![100.0, 0.0, 0.0, 0.0];

    let (norm_global, norm_per_core) = MetricsState::normalize_cpu_usage(
        &CpuMode::Percentage0to100,
        global_cpu,
        nproc,
        per_core.clone(),
    );
    assert_eq!(norm_global, CpuUsage::new(25.0));
    let expected_per_core: Vec<CpuUsage> = per_core.into_iter().map(CpuUsage::new).collect();
    assert_eq!(norm_per_core, expected_per_core);
}

#[test]
fn test_normalize_cpu_usage_nproc() {
    let global_cpu = 25.0;
    let nproc = 4.0;
    let per_core = vec![100.0, 0.0, 0.0, 0.0];

    let (norm_global, norm_per_core) = MetricsState::normalize_cpu_usage(
        &CpuMode::PercentageNproc,
        global_cpu,
        nproc,
        per_core.clone(),
    );
    assert_eq!(norm_global, CpuUsage::new(100.0));
    let expected_per_core: Vec<CpuUsage> = per_core.into_iter().map(CpuUsage::new).collect();
    assert_eq!(norm_per_core, expected_per_core);
}

#[test]
fn test_normalize_cpu_usage_disabled() {
    let (norm_global, norm_per_core) =
        MetricsState::normalize_cpu_usage(&CpuMode::Disabled, 25.0, 4.0, vec![100.0, 0.0]);
    assert_eq!(norm_global, CpuUsage::new(0.0));
    assert_eq!(norm_per_core, vec![CpuUsage::new(0.0), CpuUsage::new(0.0)]);
}

#[test]
fn test_metrics_config() {
    let config = MetricsConfig::default();
    assert_eq!(*config.cpu(), CpuMode::Percentage0to100);
    assert_eq!(config.network(), None);
    assert_eq!(config.temperature(), None);
    assert_eq!(config.disk(), None);
    assert_eq!(config.update_interval_ms().value(), 1000);
}

#[test]
fn test_metrics_types() {
    assert!((CpuUsage::new(42.0).value() - 42.0).abs() < f32::EPSILON);
    assert_eq!(MemoryBytes::new(1024).value(), 1024);
    assert_eq!(NetworkSpeed::new(512).value(), 512);
    assert!((Temperature::new(60.0).value() - 60.0).abs() < f32::EPSILON);
    assert_eq!(DiskName::new("sda1").as_str(), "sda1");
    assert_eq!(MountPoint::new("/mnt").as_str(), "/mnt");
}

#[test]
fn test_disk_metric() {
    let disk_metric = DiskMetric::new(
        DiskName::new("nvme0n1"),
        MountPoint::new("/"),
        MemoryBytes::new(100),
        MemoryBytes::new(20),
        MemoryBytes::new(80),
    );
    assert_eq!(disk_metric.name().as_str(), "nvme0n1");
    assert_eq!(disk_metric.mount_point().as_str(), "/");
}

#[test]
fn test_metrics_state_new() {
    let mem = MemoryMetrics::new(
        MemoryBytes::new(100),
        MemoryBytes::new(200),
        MemoryBytes::new(10),
        MemoryBytes::new(20),
    );
    assert_eq!(mem.used(), MemoryBytes::new(100));
    assert_eq!(mem.total(), MemoryBytes::new(200));
    assert_eq!(mem.swap_used(), MemoryBytes::new(10));
    assert_eq!(mem.swap_total(), MemoryBytes::new(20));

    let net = NetworkMetrics::new(NetworkSpeed::new(1), NetworkSpeed::new(2));
    assert_eq!(net.tx(), NetworkSpeed::new(1));
    assert_eq!(net.rx(), NetworkSpeed::new(2));

    let cmd = CreateMetricsCommand::new(
        CpuUsage::new(10.0),
        vec![],
        mem.clone(),
        vec![],
        net.clone(),
        Some(Temperature::new(40.0)),
        MetricsConfig::default(),
    );
    assert_eq!(cmd.cpu_usage(), &CpuUsage::new(10.0));
    assert_eq!(cmd.memory(), &mem);
    assert_eq!(cmd.network(), &net);
    assert_eq!(cmd.memory_used(), MemoryBytes::new(100));
    assert_eq!(cmd.network_tx(), NetworkSpeed::new(1));

    let state = MetricsState::new(cmd);
    assert_eq!(state.cpu_usage, CpuUsage::new(10.0));
    assert_eq!(state.memory_used, MemoryBytes::new(100));
}

fn degrees(readings: &[Option<f32>], mode: &TemperatureMode) -> Option<f32> {
    Temperature::from_readings(readings.iter().copied(), mode)
        .map(|temperature| temperature.value())
}

#[test]
fn test_from_readings_averages_celsius() {
    let average = degrees(&[Some(40.0), Some(60.0)], &TemperatureMode::Celsius).unwrap();
    assert!((average - 50.0).abs() < f32::EPSILON);
}

#[test]
fn test_from_readings_converts_to_fahrenheit() {
    let boiling = degrees(&[Some(100.0)], &TemperatureMode::Fahrenheit).unwrap();
    assert!((boiling - 212.0).abs() < f32::EPSILON);
    let freezing = degrees(&[Some(0.0)], &TemperatureMode::Fahrenheit).unwrap();
    assert!((freezing - 32.0).abs() < f32::EPSILON);
}

#[test]
fn test_from_readings_keeps_negative_readings() {
    let celsius = degrees(&[Some(-10.0)], &TemperatureMode::Celsius).unwrap();
    assert!((celsius + 10.0).abs() < f32::EPSILON);
    let fahrenheit = degrees(&[Some(-10.0)], &TemperatureMode::Fahrenheit).unwrap();
    assert!((fahrenheit - 14.0).abs() < f32::EPSILON);
    let mixed = degrees(&[Some(-20.0), Some(40.0)], &TemperatureMode::Celsius).unwrap();
    assert!((mixed - 10.0).abs() < f32::EPSILON);
}

#[test]
fn test_from_readings_skips_missing_and_non_finite_readings() {
    let readings = [
        None,
        Some(f32::NAN),
        Some(f32::INFINITY),
        Some(f32::NEG_INFINITY),
        Some(30.0),
    ];
    let celsius = degrees(&readings, &TemperatureMode::Celsius).unwrap();
    assert!((celsius - 30.0).abs() < f32::EPSILON);
    let fahrenheit = degrees(&readings, &TemperatureMode::Fahrenheit).unwrap();
    assert!((fahrenheit - 86.0).abs() < f32::EPSILON);
}

#[test]
fn test_from_readings_is_absent_without_a_valid_reading() {
    for mode in [TemperatureMode::Celsius, TemperatureMode::Fahrenheit] {
        assert_eq!(degrees(&[], &mode), None);
        assert_eq!(degrees(&[None, None], &mode), None);
        assert_eq!(
            degrees(
                &[Some(f32::NAN), Some(f32::INFINITY), Some(f32::NEG_INFINITY)],
                &mode
            ),
            None
        );
    }
}

#[test]
fn test_from_readings_is_absent_when_disabled() {
    assert_eq!(degrees(&[Some(40.0)], &TemperatureMode::Disabled), None);
}

#[test]
fn test_absent_temperature_is_omitted_from_the_snapshot() {
    let absent = serde_json::to_value(MetricsState::default()).unwrap();
    assert!(absent.get("temperature").is_none());

    let present = MetricsState {
        temperature: Some(Temperature::new(45.0)),
        ..MetricsState::default()
    };
    let present = serde_json::to_value(present).unwrap();
    assert_eq!(present.get("temperature"), Some(&serde_json::json!(45.0)));
}
