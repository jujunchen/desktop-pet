//! 宠物行为控制工具
//!
//! 通过 Tauri 事件与前端通信，真正触发宠物的动画和行为。
//! 支持的动作与前端 usePetState.ts 保持一致。

use super::registry::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use tauri::Emitter;

/// 宠物控制工具
pub struct PetControlTool;

const EVT_PET_ACTION: &str = "pet://action";

#[async_trait]
impl Tool for PetControlTool {
    fn name(&self) -> &str {
        "pet_control"
    }

    fn description(&self) -> &str {
        "控制桌面宠物的动画和行为。
当用户说'开心点'、'跳个舞吧'、'去接飞盘'、'睡觉吧'、'来个xx表情'时使用（xx表示某个表情名称）。
支持的动作类型：\n- happy: 开心\n- curious: 好奇\n- crazy: 惊讶\n- angry: 生气- sleeping: 睡觉- dance: 跳舞\n- frisbee: 接飞盘\n"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["happy", "curious","crazy", "angry", "sleeping", "dance", "frisbee"],
                    "description": "动作类型：\n- happy: 开心\n- curious: 好奇\n- crazy: 惊讶\n- angry: 生气- sleeping: 睡觉- dance: 跳舞\n- frisbee: 接飞盘\n"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, app: tauri::AppHandle, args: Value) -> ToolResult {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "缺少 action 参数".to_string())?;

        let result = match action {
            "happy" | "开心" => {
                let _ = app.emit(EVT_PET_ACTION, serde_json::json!({
                    "type": "action",
                    "action": "happy"
                }));
                "小白开心地摇着尾巴~汪✨🐕".to_string()
            }

            "curious" | "好奇" => {
                let _ = app.emit(EVT_PET_ACTION, serde_json::json!({
                    "type": "action",
                    "action": "curious"
                }));
                "小白好奇地歪着头看着你，一脸问号~🐕❓".to_string()
            }

            "crazy" | "惊讶" => {
                let _ = app.emit(EVT_PET_ACTION, serde_json::json!({
                    "type": "action",
                    "action": "crazy"
                }));
                "小白惊讶地瞪大了眼睛！🐕💨".to_string()
            }

            "angry" | "生气" => {
                let _ = app.emit(EVT_PET_ACTION, serde_json::json!({
                    "type": "action",
                    "action": "crazy-plus"
                }));
                "小白非常生气！汪汪汪！🐕🔥".to_string()
            }

            "sleeping" | "睡觉" => {
                let _ = app.emit(EVT_PET_ACTION, serde_json::json!({
                    "type": "action",
                    "action": "sleeping"
                }));
                "小白要睡觉了！汪汪汪！🐕💤".to_string()
            }

            "dance" | "跳舞" => {
                let _ = app.emit(EVT_PET_ACTION, serde_json::json!({
                    "type": "action",
                    "action": "dance"
                }));
                "小白开始跳舞了！🐕�".to_string()
            }

            "frisbee" | "接飞盘" => {
                let _ = app.emit(EVT_PET_ACTION, serde_json::json!({
                    "type": "action",
                    "action": "frisbee"
                }));
                "小白兴奋地跑去接飞盘！🐕🎾".to_string()
            }

            _ => return Err(format!("不支持的动作: {}", action)),
        };

        Ok(result)
    }
}
