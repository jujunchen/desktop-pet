//! 获取系统状态工具（CPU、内存、磁盘）

use super::registry::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use sysinfo::{System, Disks, Disk};

/// 系统状态工具
pub struct SystemStatusTool;

#[async_trait]
impl Tool for SystemStatusTool {
    fn name(&self) -> &str {
        "get_system_status"
    }

    fn description(&self) -> &str {
        "获取电脑的CPU使用率、内存占用、磁盘使用率等系统状态信息。
        当用户问'电脑卡不卡'、'CPU多少'、'内存还剩多少'、'磁盘满了没'时使用。"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "detail": {
                    "type": "string",
                    "enum": ["all", "cpu", "memory", "disk"],
                    "default": "all",
                    "description": "获取哪些信息：all=全部，cpu=仅CPU，memory=仅内存，disk=仅磁盘"
                }
            }
        })
    }

    async fn execute(&self, _app: tauri::AppHandle, args: Value) -> ToolResult {
        let detail = args
            .get("detail")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        let mut result = Vec::new();

        match detail {
            "all" | "cpu" => {
                if let Some(cpu) = get_cpu_usage().await {
                    result.push(format!("CPU 使用率: {}%", cpu));
                }
            }
            _ => {}
        }

        match detail {
            "all" | "memory" => {
                if let Some((used, total)) = get_memory_usage() {
                    let percent = (used as f64 / total as f64 * 100.0) as u32;
                    result.push(format!(
                        "内存使用: {} / {} GB ({}%)",
                        used, total, percent
                    ));
                }
            }
            _ => {}
        }

        match detail {
            "all" | "disk" => {
                if let Some((used, total)) = get_disk_usage() {
                    let percent = (used as f64 / total as f64 * 100.0) as u32;
                    result.push(format!(
                        "磁盘使用: {} / {} GB ({}%)",
                        used, total, percent
                    ));
                }
            }
            _ => {}
        }

        if result.is_empty() {
            return Err("无法获取系统状态信息".to_string());
        }

        Ok(format!("系统状态信息：\n{}", result.join("\n")))
    }
}

/// 获取CPU使用率
async fn get_cpu_usage() -> Option<u32> {
    let mut sys = System::new();
    sys.refresh_cpu();
    tokio::time::sleep(Duration::from_millis(200)).await;
    sys.refresh_cpu();

    let cpus = sys.cpus();
    if cpus.is_empty() {
        None
    } else {
        let avg_usage: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32;
        Some(avg_usage.round() as u32)
    }
}

/// 获取内存使用（GB）
fn get_memory_usage() -> Option<(u32, u32)> {
    let mut sys = System::new();
    sys.refresh_memory();

    let total_gb = (sys.total_memory() / 1024 / 1024 / 1024) as u32;
    let used_gb = (sys.used_memory() / 1024 / 1024 / 1024) as u32;

    if total_gb > 0 {
        Some((used_gb, total_gb))
    } else {
        None
    }
}

/// 获取磁盘使用（GB）- 返回主磁盘的使用情况
fn get_disk_usage() -> Option<(u32, u32)> {
    let disks = Disks::new_with_refreshed_list();

    // 找到主磁盘（通常是最大的那个）
    let main_disk: Option<&Disk> = disks.iter().max_by_key(|d| d.total_space());

    if let Some(disk) = main_disk {
        let total_gb = (disk.total_space() / 1024 / 1024 / 1024) as u32;
        let available_gb = (disk.available_space() / 1024 / 1024 / 1024) as u32;
        let used_gb = total_gb.saturating_sub(available_gb);

        if total_gb > 0 {
            Some((used_gb, total_gb))
        } else {
            None
        }
    } else {
        None
    }
}
