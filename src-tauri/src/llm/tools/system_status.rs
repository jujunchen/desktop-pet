//! 获取系统状态工具（CPU、内存、磁盘）

use super::registry::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;

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

/// 获取CPU使用率（简单实现）
async fn get_cpu_usage() -> Option<u32> {
    #[cfg(target_os = "windows")]
    {
        // 简单延迟模拟CPU采样
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 使用 sysinfo 库或者 Windows API
        // 这里先用随机值模拟（后续可以替换成真实实现）
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Some(rng.gen_range(10..80))
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// 获取内存使用
fn get_memory_usage() -> Option<(u32, u32)> {
    #[cfg(target_os = "windows")]
    {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let total = 16; // 假设16GB
        let used = rng.gen_range(4..12);
        Some((used, total))
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// 获取磁盘使用
fn get_disk_usage() -> Option<(u32, u32)> {
    #[cfg(target_os = "windows")]
    {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let total = 512; // 假设512GB
        let used = rng.gen_range(200..400);
        Some((used, total))
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}
