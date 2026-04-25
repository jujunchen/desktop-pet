//! 获取系统时间的工具

use super::registry::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

/// 系统时间工具
pub struct SystemTimeTool;

#[async_trait]
impl Tool for SystemTimeTool {
    fn name(&self) -> &str {
        "get_system_time"
    }

    fn description(&self) -> &str {
        "获取当前系统的日期和时间。当用户问现在几点、今天几号、什么时间等问题时使用。"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "enum": ["full", "date", "time"],
                    "default": "full",
                    "description": "返回格式：full=完整日期时间，date=仅日期，time=仅时间"
                }
            }
        })
    }

    async fn execute(&self, _app: tauri::AppHandle, args: Value) -> ToolResult {
        let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("full");

        let now = chrono::offset::Local::now();
        let result = match format {
            "date" => now.format("%Y年%m月%d日").to_string(),
            "time" => now.format("%H:%M:%S").to_string(),
            _ => now.format("%Y年%m月%d日 %H:%M:%S").to_string(),
        };

        Ok(format!("当前系统时间：{}", result))
    }
}
