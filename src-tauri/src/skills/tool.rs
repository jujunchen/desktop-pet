use super::manager::{install_skill, search_skills};
use crate::llm::tools::registry::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use tauri::AppHandle;

pub struct SkillSearchTool;

#[async_trait]
impl Tool for SkillSearchTool {
    fn name(&self) -> &str {
        "search_skills"
    }

    fn description(&self) -> &str {
        "搜索可用的技能包。当用户想要安装新技能或询问有什么技能可用时，先调用此工具搜索相关技能。"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "搜索关键词，如 'vercel'、'翻译'、'代码' 等"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, _app: AppHandle, args: Value) -> ToolResult {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| "Missing 'query' parameter".to_string())?;

        let results = search_skills(query).await?;

        if results.is_empty() {
            return Ok("没有找到相关技能。".to_string());
        }

        let mut response = String::from("找到以下技能包：\n");
        for result in results {
            response.push_str(&format!(
                "- {} ({}): {}\n",
                result.package,
                result.name,
                result.description
            ));
        }

        Ok(response)
    }
}

pub struct SkillInstallTool;

#[async_trait]
impl Tool for SkillInstallTool {
    fn name(&self) -> &str {
        "install_skill"
    }

    fn description(&self) -> &str {
        "安装指定的技能包。在用户确认要安装某个技能后调用此工具。"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "package": {
                    "type": "string",
                    "description": "技能包名称，如 'vercel-labs/agent-skills'"
                }
            },
            "required": ["package"]
        })
    }

    async fn execute(&self, _app: AppHandle, args: Value) -> ToolResult {
        let package = args["package"]
            .as_str()
            .ok_or_else(|| "Missing 'package' parameter".to_string())?;

        let skill = install_skill(package).await?;

        Ok(format!(
            "✅ 技能安装成功！已启用 '{}'。\n描述: {}\n现在你可以使用这个技能了。",
            skill.name, skill.description
        ))
    }
}

pub fn register_skill_tools(registry: &mut crate::llm::tools::registry::ToolRegistry) {
    registry.register(Box::new(SkillSearchTool));
    registry.register(Box::new(SkillInstallTool));
}
