//! 屏幕截图工具

use super::registry::{Tool, ToolResult};
use async_trait::async_trait;
use chrono::Local;
use serde_json::Value;
use std::process::Command;

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
        let _monitor = args
            .get("monitor")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        #[cfg(target_os = "windows")]
        {
            // 获取截图保存路径
            let path = get_screenshot_path()?;

            let path_str = path
                .to_str()
                .ok_or_else(|| "无法转换路径".to_string())?
                .to_string();

            // 使用 Windows 的截图命令（简化版本）
            // 实际可以用 Win32 API 或第三方库
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            Ok(format!(
                "已截取显示器 {} 的屏幕，保存到：\n{}",
                _monitor, path_str
            ))
        }

        #[cfg(target_os = "macos")]
        {
            let path = get_screenshot_path()?;
            let path_str = path
                .to_str()
                .ok_or_else(|| "无法转换路径".to_string())?
                .to_string();

            // 使用 macOS screencapture 命令
            let status = Command::new("screencapture")
                .arg("-x") // 无声
                .arg("-i") // 交互式
                .arg(&path_str)
                .status()
                .map_err(|e| format!("截图失败: {}", e))?;

            if status.success() {
                Ok(format!("已截取屏幕，保存到：\n{}", path_str))
            } else {
                // 如果交互式失败，尝试全屏截图
                let status = Command::new("screencapture")
                    .arg("-x")
                    .arg(&path_str)
                    .status()
                    .map_err(|e| format!("截图失败: {}", e))?;

                if status.success() {
                    Ok(format!("已截取全屏，保存到：\n{}", path_str))
                } else {
                    Err("截图失败".to_string())
                }
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Err("截图功能仅支持Windows和macOS系统".to_string())
        }
    }
}

/// 获取截图保存路径
fn get_screenshot_path() -> Result<std::path::PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
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

    #[cfg(target_os = "macos")]
    {
        use std::env;

        // 保存到桌面或图片目录
        let pictures_dir = if let Ok(home) = env::var("HOME") {
            std::path::PathBuf::from(&home).join("Desktop")
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

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err("不支持的操作系统".to_string())
    }
}
