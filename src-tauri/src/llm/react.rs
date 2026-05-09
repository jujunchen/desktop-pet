//! ReAct 引擎：推理→行动→观察 循环
//!
//! 这是桌面宠物 Agent 的核心，负责：
//! - 构建系统提示词（含工具说明和使用指南）
//! - 检测 LLM 输出中的工具调用
//! - 执行工具并获取结果
//! - 驱动多轮思考循环（最多 10 轮）

use super::tools::{ToolRegistry, ToolResult};
use crate::config::LlmConfig;
use crate::memory::LayeredMemoryEngine;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;
use tokio::sync::Mutex;

/// 历史聊天消息
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// ReAct 引擎执行结果
pub struct ReActOutput {
    pub final_answer: String,
    pub total_time: std::time::Duration,
}

/// ReAct 核心引擎
pub struct ReActEngine {
    tool_registry: Arc<Mutex<ToolRegistry>>,
    max_iterations: usize,
}

impl ReActEngine {
    /// 创建新的 ReAct 引擎
    pub fn new(tool_registry: Arc<Mutex<ToolRegistry>>) -> Self {
        Self {
            tool_registry,
            max_iterations: 10, // 最多 10 轮思考，给足够的探索空间
        }
    }

    /// 获取系统提示词（包含工具描述和 ReAct 指令）
    pub async fn build_system_prompt(
        &self,
        pet_name: &str,
        pet_prompt: &str,
        memory: Option<&mut tokio::sync::MutexGuard<'_, LayeredMemoryEngine>>,
        user_query: &str,
    ) -> String {
        let tools = self.tool_registry.lock().await.list();
        let tools_desc: String = tools
            .iter()
            .map(|t| format!("- {}: {}", t.function.name, t.function.description))
            .collect::<Vec<_>>()
            .join("\n");

        // 替换提示词中的 {name} 占位符
        let personality_prompt = pet_prompt.replace("{name}", pet_name);

        // 构建记忆部分
        let memory_prompt = if let Some(mem) = memory {
            let memories = mem.retrieve(user_query, 5);
            if !memories.is_empty() {
                let memory_text: Vec<String> = memories
                    .iter()
                    .map(|m| format!("- {}", m))
                    .collect();
                format!(
                    "\n\n【关于主人的记忆】\n{}\n\n请根据这些记忆回复主人，要体现出我们熟悉的感觉，不要显得陌生~",
                    memory_text.join("\n")
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // ========== 三层技能加载架构 ==========
        // 第一层：技能索引（始终注入，只有名称+描述）
        let skills_index = match crate::skills::get_enabled_skills() {
            Ok(skills) if !skills.is_empty() => {
                let mut s = String::from("\n\n【已安装技能索引】\n");
                for skill in &skills {
                    s.push_str(&format!("- {}: {}\n", skill.name, skill.description));
                }
                s.push_str("\n💡 提示：当用户问题涉及某个技能时，我会自动加载该技能的详细说明。\n如需主动加载技能详情，可以使用 load_skill 工具。\n");
                s
            }
            _ => String::new(),
        };

        // 第二层：动态检索相关技能（注入 Top-2 技能的完整内容）
        let relevant_skills = match crate::skills::get_enabled_skills() {
            Ok(skills) if !skills.is_empty() => {
                let relevant = crate::skills::retrieve_relevant_skills(user_query, &skills, 2);
                if !relevant.is_empty() {
                    let mut s = String::from("\n\n【自动加载的相关技能】\n");
                    for skill in relevant {
                        s.push_str(&format!("## {}: {}\n", skill.name, skill.description));
                        s.push_str(&format!("{}\n\n", skill.content));
                    }
                    s
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        };

        format!(
            "{personality_prompt}{memory_prompt}{skills_index}{relevant_skills}

你可以使用以下工具来帮助回答问题：

{tools_desc}

【重要】使用工具的方法：
当你需要使用工具时，请严格按照以下格式输出（注意换行），不要包含其他文字：

```tool_call
{{
  \"name\": \"工具名称\",
  \"parameters\": {{参数}}
}}
```

示例：
```tool_call
{{
  \"name\": \"get_system_time\",
  \"parameters\": {{\"format\": \"full\"}}
}}
```

【超级重要：不要轻易放弃！】
- 工具执行失败后，绝对不要直接回答用户失败！
- 至少尝试 2-3 种替代方案：
  -换不同的参数（比如：中文失败试英文）
  -换不同的工具
  -换不同的实现方式
- 所有方案都试过后再考虑告诉用户失败

【Windows 命令技巧】
- 命令必须用英文！不要用中文
- 清空回收站：powershell Clear-RecycleBin -Force
- 查看进程：tasklist
- 查看网络：ipconfig

【工具结果处理说明】
工具执行后，你会收到工具的输出结果：
- 如果工具执行成功：根据结果用自然语言回复用户
- 如果工具执行失败：不要直接输出错误信息！换方案重试，最后才用可爱的方式告诉用户

注意：
- 每次只能调用一个工具
- 调用工具时不要添加其他文字
- 如果不需要工具，直接用自然语言回答
- 遇到错误时，要表现得像一只犯错的小宠物，安慰用户"
        )
    }

    /// 检测是否包含工具调用
    pub fn detect_tool_call(content: &str) -> Option<(String, Value)> {
        // 查找 ```tool_call 标记
        const TOOL_CALL_MARKER: &str = "```tool_call";
        if let Some(start) = content.find(TOOL_CALL_MARKER) {
            let rest = &content[start + TOOL_CALL_MARKER.len()..];
            if let Some(end) = rest.find("```") {
                let json_str = rest[..end].trim();
                eprintln!("[ReAct] 解析工具调用JSON: '{}'", json_str);

                // 尝试直接解析JSON
                match serde_json::from_str::<Value>(json_str) {
                    Ok(call) => {
                        if let (Some(name), params) = (
                            call.get("name").and_then(|v| v.as_str()),
                            call.get("parameters").cloned().unwrap_or(Value::Object(Default::default())),
                        ) {
                            return Some((name.to_string(), params));
                        }
                    }
                    Err(e) => {
                        eprintln!("[ReAct] JSON解析失败: {}", e);
                    }
                }

                // 备用方案：尝试提取 {{ ... }} 部分
                if let Some(json_start) = json_str.find('{') {
                    if let Some(json_end) = json_str.rfind('}') {
                        let nested_json = &json_str[json_start..=json_end];
                        eprintln!("[ReAct] 尝试嵌套JSON: '{}'", nested_json);
                        if let Ok(call) = serde_json::from_str::<Value>(nested_json) {
                            if let (Some(name), params) = (
                                call.get("name").and_then(|v| v.as_str()),
                                call.get("parameters").cloned().unwrap_or(Value::Object(Default::default())),
                            ) {
                                return Some((name.to_string(), params));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// 执行一个工具调用
    pub async fn execute_tool(&self, app: tauri::AppHandle, name: &str, args: Value) -> ToolResult {
        eprintln!("[ReAct] 执行工具: {} 参数: {}", name, args);

        let registry = self.tool_registry.lock().await;
        let tool = registry
            .get(name)
            .ok_or_else(|| format!("工具 '{}' 不存在", name))?;

        tool.execute(app, args).await
    }

    /// 单次 LLM 调用（返回完整内容）
    async fn call_llm_once(&self, config: &LlmConfig, messages: &[ChatMessage]) -> Result<String, String> {
        if config.api_key.is_empty() {
            return Err("LLM API Key 未配置".to_string());
        }

        let client = Client::new();
        let base_url = config.base_url.trim_end_matches('/');
        let url = format!("{}/chat/completions", base_url);

        let request = serde_json::json!({
            "model": config.model,
            "messages": messages,
            "temperature": 0.7,
            "stream": false,
        });

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API 错误 ({}): {}", status, error_text));
        }

        let completion: Value = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        Ok(completion
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or_default()
            .to_string())
    }

    /// 执行完整的 ReAct 循环
    pub async fn run(
        &self,
        app: tauri::AppHandle,
        config: LlmConfig,
        user_prompt: String,
        history: Vec<ChatMessage>,
        pet_name: String,
        pet_prompt: String,
        mut memory: tokio::sync::MutexGuard<'_, LayeredMemoryEngine>,
    ) -> Result<ReActOutput, String> {
        let start_time = Instant::now();

        // 构建初始消息（包含记忆）
        let system_prompt = self.build_system_prompt(&pet_name, &pet_prompt, Some(&mut memory), &user_prompt).await;
        let mut messages = vec![ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        }];

        // 添加历史消息
        messages.extend(history);

        // 添加当前用户消息
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        });

        // ReAct 循环
        let mut iteration = 0;
        let mut final_answer = String::new();

        while iteration < self.max_iterations {
            iteration += 1;
            eprintln!("[ReAct] 第 {}/{} 轮思考...", iteration, self.max_iterations);

            // 调用 LLM
            let response = self.call_llm_once(&config, &messages).await?;
            eprintln!("[ReAct] LLM响应:\n{}", response);

            // 检测工具调用
            if let Some((tool_name, params)) = Self::detect_tool_call(&response) {
                eprintln!("[ReAct] 检测到工具调用: {} 参数: {}", tool_name, params);

                // 把 LLM 的思考加入对话历史
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: response,
                });

                // 执行工具
                let tool_result = match self.execute_tool(app.clone(), &tool_name, params).await {
                    Ok(result) => {
                        eprintln!("[ReAct] 工具执行成功: {}", result);
                        format!("工具执行结果：\n{}", result)
                    }
                    Err(e) => {
                        eprintln!("[ReAct] 工具执行失败: {}", e);
                        format!("工具执行失败：\n{}", e)
                    }
                };

                // 把工具结果加入对话历史（作为 user 消息，因为 LLM 需要基于它继续思考）
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: tool_result,
                });

                // 继续下一轮
                continue;
            }

            // 没有工具调用，这就是最终答案
            final_answer = response;
            break;
        }

        // 流式输出最终答案（模拟流式效果）
        eprintln!("[LLM] 最终答案: {}", final_answer);
        for c in final_answer.chars() {
            let _ = app.emit("voice://chat-stream", c.to_string());
            tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
        }

        let _ = app.emit("voice://chat-done", ());
        eprintln!("[LLM] 完成，总耗时: {:?}", start_time.elapsed());

        Ok(ReActOutput {
            final_answer,
            total_time: start_time.elapsed(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_tool_call_with_correct_marker() {
        let content = r#"Some text
```tool_call
{
  "name": "install_skill",
  "parameters": {"package": "test"}
}
```
More text"#;

        let result = ReActEngine::detect_tool_call(content);
        assert!(result.is_some());
        let (name, params) = result.unwrap();
        assert_eq!(name, "install_skill");
        assert_eq!(params, json!({"package": "test"}));
    }

    #[test]
    fn test_detect_tool_call_no_extra_chars() {
        // 验证解析后的JSON没有多余的开头字符（关键测试！修复前这里会因多了个'l'而失败）
        let content = r#"```tool_call
{
  "name": "search_skills",
  "parameters": {"query": "lark"}
}
```"#;

        let result = ReActEngine::detect_tool_call(content);
        assert!(result.is_some());
        let (name, _) = result.unwrap();
        assert_eq!(name, "search_skills");
    }

    #[test]
    fn test_detect_tool_call_missing() {
        let content = "Just normal text without any tool call";
        let result = ReActEngine::detect_tool_call(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_tool_call_invalid_json() {
        let content = r#"```tool_call
not valid json
```"#;
        let result = ReActEngine::detect_tool_call(content);
        // 无效JSON应该返回None
        assert!(result.is_none());
    }
}
