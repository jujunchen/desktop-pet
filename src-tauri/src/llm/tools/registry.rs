//! 工具注册和管理核心

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// 工具执行结果
pub type ToolResult = Result<String, String>;

/// 工具信息（用于序列化给LLM）
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolInfo {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: FunctionInfo,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FunctionInfo {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// 工具 Trait 定义
/// 所有工具都需要实现这个 trait
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具唯一名称（给LLM调用用）
    fn name(&self) -> &str;

    /// 工具描述（告诉LLM这个工具是做什么的）
    fn description(&self) -> &str;

    /// 参数定义（JSON Schema 格式）
    fn parameters(&self) -> Value;

    /// 执行工具
    async fn execute(&self, app: tauri::AppHandle, args: Value) -> ToolResult;
}

/// 工具注册表
/// 管理所有可用的工具
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// 创建空的注册表
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// 注册一个工具
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        eprintln!("[Tool] 注册工具: {}", name);
        self.tools.insert(name, tool);
    }

    /// 注销一个工具
    pub fn unregister(&mut self, name: &str) {
        self.tools.remove(name);
    }

    /// 获取工具
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| b.as_ref())
    }

    /// 获取所有工具的描述列表（给LLM用）
    pub fn list(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .map(|tool| ToolInfo {
                type_: "function".to_string(),
                function: FunctionInfo {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    parameters: tool.parameters(),
                },
            })
            .collect()
    }

    /// 检查工具是否存在
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// 获取已注册工具数量
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// 批量注册工具的宏
///
/// # 示例
/// ```
/// register_tools!(registry,
///     SystemTimeTool,
///     SystemStatusTool,
///     OpenAppTool,
/// );
/// ```
#[macro_export]
macro_rules! register_tools {
    ($registry:expr, $($tool:expr),* $(,)?) => {
        $(
            $registry.register(Box::new($tool));
        )*
    };
}
