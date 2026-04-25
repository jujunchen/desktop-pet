//! LLM 聊天模块
//!
//! 使用 ReAct 模式驱动 Agent 与用户交互。
//! 核心逻辑在 `react.rs` 中。

mod react;
pub mod tools;

pub use react::{ChatMessage, ReActEngine};
pub use tools::{register_builtin_tools, ToolRegistry};

use crate::config::LlmConfig;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 全局 ReAct 引擎状态
#[derive(Clone)]
pub struct GlobalReActEngine {
    inner: Arc<Mutex<ReActEngine>>,
}

impl GlobalReActEngine {
    pub fn new(registry: Arc<Mutex<ToolRegistry>>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ReActEngine::new(registry))),
        }
    }
}

impl Default for GlobalReActEngine {
    fn default() -> Self {
        let mut registry = ToolRegistry::new();
        register_builtin_tools(&mut registry);

        Self::new(Arc::new(Mutex::new(registry)))
    }
}

/// LLM 聊天入口（ReAct 模式）
pub async fn chat_with_llm_stream(
    app: tauri::AppHandle,
    config: LlmConfig,
    prompt: String,
    history: Vec<ChatMessage>,
    engine: tauri::State<'_, GlobalReActEngine>,
) -> Result<(), String> {
    let engine = engine.inner.lock().await;
    engine.run(app, config, prompt, history).await?;
    Ok(())
}
