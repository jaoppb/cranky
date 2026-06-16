use std::sync::Arc;
use tokio::sync::RwLock;
use sysinfo::{System, Networks, Components, Disks};
use crate::domain::metrics::{MetricsConfig, MetricsState, CpuMode, DiskMetric};
use crate::domain::signals::SignalHub;

pub struct SysinfoAdapter {
    config: MetricsConfig,
    hub: Arc<SignalHub>,
}

impl SysinfoAdapter {
    pub fn new(config: MetricsConfig, hub: Arc<SignalHub>) -> Self {
        Self { config, hub }
    }

    pub async fn start(&self) {
        let config = self.config.clone();
        let hub = self.hub.clone();

        tokio::task::spawn_blocking(move || {
            let mut sys = System::new_all();
            let mut networks = Networks::new_with_refreshed_list();
            let mut disks = Disks::new_with_refreshed_list();
            let mut components = Components::new_with_refreshed_list();

            loop {
                sys.refresh_cpu_usage();
                sys.refresh_memory();
                networks.refresh(true);
                disks.refresh(true);
                components.refresh(true);

                // CPU
                let nproc = sys.cpus().len() as f32;
                let global_cpu = sys.global_cpu_usage();
                
                let per_core_raw: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
                let (cpu_usage, per_core) = MetricsState::normalize_cpu_usage(&config.cpu, global_cpu, nproc, per_core_raw);

                // Network
                let mut network_tx: u64 = 0;
                let mut network_rx: u64 = 0;
                if config.network.is_some() {
                    for (_interface_name, data) in &networks {
                        network_tx += data.transmitted();
                        network_rx += data.received();
                    }
                }

                // Disks
                let mut disk_metrics = Vec::new();
                if config.disk.is_some() {
                    for disk in &disks {
                        disk_metrics.push(DiskMetric {
                            name: disk.name().to_string_lossy().to_string(),
                            mount_point: disk.mount_point().to_string_lossy().to_string(),
                            total_bytes: disk.total_space(),
                            available_bytes: disk.available_space(),
                            used_bytes: disk.total_space().saturating_sub(disk.available_space()),
                        });
                    }
                }

                // Temperature
                let mut temp = 0.0;
                if config.temperature.is_some() {
                    let mut count = 0;
                    for component in &components {
                        if let Some(t) = component.temperature() {
                            temp += t;
                            count += 1;
                        }
                    }
                    if count > 0 {
                        temp /= count as f32;
                    }
                    
                    if config.temperature == Some(crate::domain::metrics::TemperatureMode::Fahrenheit) {
                        temp = (temp * 9.0 / 5.0) + 32.0;
                    }
                }

                let state = MetricsState {
                    cpu_usage,
                    per_core,
                    memory_used: sys.used_memory(),
                    memory_total: sys.total_memory(),
                    swap_used: sys.used_swap(),
                    swap_total: sys.total_swap(),
                    disks: disk_metrics,
                    network_tx,
                    network_rx,
                    temperature: temp,
                    config: config.clone(),
                };

                let _ = hub.metrics_tx().send(state);
                
                std::thread::sleep(std::time::Duration::from_millis(config.update_interval_ms));
            }
        });
    }
}
