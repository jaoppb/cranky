use super::command::CreateMetricsCommand;
use super::config::{CpuMode, MetricsConfig};
use super::state::MetricsState;
use super::types::{CpuUsage, DiskMetric, DiskName, MemoryBytes, MountPoint, NetworkSpeed, Temperature};

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
    let dm = DiskMetric::new(
        DiskName::new("nvme0n1"),
        MountPoint::new("/"),
        MemoryBytes::new(100),
        MemoryBytes::new(20),
        MemoryBytes::new(80),
    );
    assert_eq!(dm.name().as_str(), "nvme0n1");
    assert_eq!(dm.mount_point().as_str(), "/");
}

#[test]
fn test_metrics_state_new() {
    let cmd = CreateMetricsCommand::new(
        CpuUsage::new(10.0),
        vec![],
        MemoryBytes::new(100),
        MemoryBytes::new(200),
        MemoryBytes::new(10),
        MemoryBytes::new(20),
        vec![],
        NetworkSpeed::new(1),
        NetworkSpeed::new(2),
        Temperature::new(40.0),
        MetricsConfig::default(),
    );
    let state = MetricsState::new(cmd);
    assert_eq!(state.cpu_usage, CpuUsage::new(10.0));
    assert_eq!(state.memory_used, MemoryBytes::new(100));
}
