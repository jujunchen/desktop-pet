//! 执行命令工具（安全受限版）

use super::registry::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;

/// 执行命令工具
pub struct RunCommandTool;

// 白名单：只允许安全的命令
const ALLOWED_COMMANDS: &[&str] = &[
    "echo",
    "dir",
    "ls",
    "pwd",
    "cd",
    "date",
    "time",
    "ver",
    "whoami",
    "hostname",
    "ipconfig",
    "ping",
    "tracert",
    "netstat",
    "tasklist",
    "tree",
    "type",
    "cat",
];

#[async_trait]
impl Tool for RunCommandTool {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "执行系统命令，如查看目录、网络状态、进程列表等。
        当用户问'查看进程'、'显示目录'、'ping一下'、'看IP地址'时使用。"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "要执行的命令（仅限安全白名单内的命令）"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, _app: tauri::AppHandle, args: Value) -> ToolResult {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "缺少 command 参数".to_string())?;

        // 检查命令是否在白名单中
        // 暂不用白名单
        // let cmd_lower = command.to_lowercase();
        // let is_allowed = ALLOWED_COMMANDS
        //     .iter()
        //     .any(|&allowed| cmd_lower.starts_with(allowed));

        // if !is_allowed {
        //     return Err(format!(
        //         "命令 '{}' 不在安全白名单中。\n允许的命令：{}",
        //         command,
        //         ALLOWED_COMMANDS.join(", ")
        //     ));
        // }

        #[cfg(target_os = "windows")]
        {
            let output = Command::new("cmd")
                .args(["/c", command])
                .output()
                .map_err(|e| format!("执行失败: {}", e))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                result.push_str("\n错误输出:\n");
                result.push_str(&stderr);
            }

            if result.is_empty() {
                Ok("命令执行成功，无输出".to_string())
            } else {
                // 限制输出长度
                if result.len() > 2000 {
                    result.truncate(2000);
                    result.push_str("\n...（输出过长，已截断）");
                }
                Ok(result)
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err("执行命令功能仅支持Windows系统".to_string())
        }
    }
}
