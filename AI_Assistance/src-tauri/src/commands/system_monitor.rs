//! System Monitor — lightweight system stats via the `sysinfo` crate.
//!
//! Provides CPU, memory, and disk usage metrics for the dashboard widget.

use serde::Serialize;
use sysinfo::System;

#[derive(Debug, Clone, Serialize)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub cpu_count: usize,
    pub cpu_brand: String,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub ram_usage_pct: f64,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub disk_usage_pct: f64,
    pub uptime_hours: f64,
    pub process_count: usize,
}

#[tauri::command]
pub fn get_system_stats() -> SystemStats {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Wait briefly for CPU measurement accuracy
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_cpu_usage();

    let cpu_usage = sys.global_cpu_usage();
    let cpu_count = sys.cpus().len();
    let cpu_brand = sys.cpus().first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let ram_total = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let ram_used = sys.used_memory() as f64 / (1024.0 * 1024.0 * 1024.0);
    let ram_pct = if ram_total > 0.0 { (ram_used / ram_total) * 100.0 } else { 0.0 };

    // Disk info — get the root/main disk
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let (disk_total, disk_used) = disks.iter()
        .find(|d| d.mount_point().to_string_lossy().contains("C:") || d.mount_point().to_string_lossy() == "/")
        .or_else(|| disks.iter().next())
        .map(|d| {
            let total = d.total_space() as f64 / (1024.0 * 1024.0 * 1024.0);
            let available = d.available_space() as f64 / (1024.0 * 1024.0 * 1024.0);
            (total, total - available)
        })
        .unwrap_or((0.0, 0.0));
    let disk_pct = if disk_total > 0.0 { (disk_used / disk_total) * 100.0 } else { 0.0 };

    let uptime_secs = System::uptime();
    let uptime_hours = uptime_secs as f64 / 3600.0;

    let process_count = sys.processes().len();

    SystemStats {
        cpu_usage,
        cpu_count,
        cpu_brand,
        ram_used_gb: (ram_used * 100.0).round() / 100.0,
        ram_total_gb: (ram_total * 100.0).round() / 100.0,
        ram_usage_pct: (ram_pct * 10.0).round() / 10.0,
        disk_used_gb: (disk_used * 100.0).round() / 100.0,
        disk_total_gb: (disk_total * 100.0).round() / 100.0,
        disk_usage_pct: (disk_pct * 10.0).round() / 10.0,
        uptime_hours: (uptime_hours * 10.0).round() / 10.0,
        process_count,
    }
}
