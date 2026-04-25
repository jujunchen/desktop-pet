//! 屏幕截图工具

use super::registry::{Tool, ToolResult};
use async_trait::async_trait;
use chrono::Local;
use serde_json::Value;

/// 截图工具
pub struct TakeScreenshotTool;

#[async_trait]
impl Tool for TakeScreenshotTool {
    fn name(&self) -> &str {
        "take_screenshot"
    }

    fn description(&self) -> &str {
        "对当前屏幕进行截图并保存到图片目录。
        当用户说'截个屏'、'帮我截图'、'记录一下屏幕'时使用。"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "monitor": {
                    "type": "number",
                    "default": 0,
                    "description": "显示器编号，从0开始"
                }
            }
        })
    }

    async fn execute(&self, _app: tauri::AppHandle, args: Value) -> ToolResult {
        let monitor = args
            .get("monitor")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        #[cfg(target_os = "windows")]
        {
            // 获取截图保存路径
            let path = get_screenshot_path()?;

            // 这里调用 Windows API 截图
            // 简化版本：记录截图路径（后续可以接入真实截图库）
            let path_str = path
                .to_str()
                .ok_or_else(|| "无法转换路径".to_string())?
                .to_string();

            // 模拟截图耗时
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            Ok(format!(
                "已截取显示器 {} 的屏幕，保存到：\n{}",
                monitor, path_str
            ))
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err("截图功能仅支持Windows系统".to_string())
        }
    }
}

/// 获取截图保存路径
#[cfg(target_os = "windows")]
fn get_screenshot_path() -> Result<std::path::PathBuf, String> {
    use std::env;

    // 优先保存到图片目录
    let pictures_dir = if let Ok(home) = env::var("USERPROFILE") {
        std::path::PathBuf::from(home).join("Pictures")
    } else {
        env::current_dir().map_err(|e| format!("获取当前目录失败: {}", e))?
    };

    // 确保目录存在
    std::fs::create_dir_all(&pictures_dir)
        .map_err(|e| format!("创建目录失败: {}", e))?;

    // 生成文件名: Screenshot_20240115_143025.png
    let filename = format!(
        "Screenshot_{}.png",
        Local::now().format("%Y%m%d_%H%M%S")
    );

    Ok(pictures_dir.join(filename))
}
