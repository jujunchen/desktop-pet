use crate::config::LlmConfig;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tauri::{Emitter, Runtime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub text: String,
    pub action: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    delta: Delta,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

const EVT_CHAT_STREAM: &str = "voice://chat-stream";
const EVT_CHAT_DONE: &str = "voice://chat-done";

pub async fn chat_with_llm_stream<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    config: LlmConfig,
    prompt: String,
) -> Result<(), String> {
    let result = chat_with_llm_stream_inner(&app_handle, config, prompt).await;

    let _ = app_handle.emit(EVT_CHAT_DONE, ());
    result
}

async fn chat_with_llm_stream_inner<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
    config: LlmConfig,
    prompt: String,
) -> Result<(), String> {
    let start_time = Instant::now();
    let mut first_token_time: Option<Instant> = None;

    eprintln!("[LLM] 配置 - API Key 长度: {}, 模型: {}, URL: {}",
        config.api_key.len(), config.model, config.base_url);

    if config.api_key.is_empty() {
        eprintln!("[LLM] 错误: API Key 为空");
        return Err("LLM API Key 未配置，请在设置中填写".to_string());
    }

    let client = Client::new();
    let base_url = config.base_url.trim_end_matches('/');
    let url = format!("{}/chat/completions", base_url);

    let system_prompt = "你是一只可爱的桌面宠物，名字叫小白。\
        你的性格活泼、友好、有点调皮。\
        请用简短、口语化的方式回复，不要太长。\
        回复时要像宠物一样可爱，可以用一些语气词如\"汪\"、\"呀\"、\"呢\"等。";

    let request = ChatRequest {
        model: config.model,
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        temperature: 0.7,
        stream: true,
    };

    eprintln!("[LLM] 发送请求中...");
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            eprintln!("[LLM] 请求失败: {}", e);
            format!("请求失败: {}", e)
        })?;

    eprintln!("[LLM] 响应状态: {}, 首包耗时: {:?}", response.status(), start_time.elapsed());
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        eprintln!("[LLM] API 错误响应: {}", error_text);
        return Err(format!("API 错误 ({}): {}", status, error_text));
    }

    let mut stream = response.bytes_stream();
    let mut total_content = String::new();
    let mut token_count = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| {
            eprintln!("[LLM] 读取流失败: {}", e);
            format!("读取流失败: {}", e)
        })?;
        let text = String::from_utf8_lossy(&chunk);

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if line == "data: [DONE]" {
                break;
            }
            if let Some(json_str) = line.strip_prefix("data: ") {
                if let Ok(parsed) = serde_json::from_str::<ChatCompletionChunk>(json_str) {
                    if let Some(choice) = parsed.choices.first() {
                        if let Some(content) = &choice.delta.content {
                            if first_token_time.is_none() {
                                first_token_time = Some(Instant::now());
                                eprintln!("[LLM] 首Token耗时: {:?}", first_token_time.unwrap().duration_since(start_time));
                            }
                            token_count += 1;
                            total_content.push_str(content);
                            let _ = app_handle.emit(EVT_CHAT_STREAM, content);
                        }
                    }
                }
            }
        }
    }

    let total_time = start_time.elapsed();
    let ttf = first_token_time.map(|t| t.duration_since(start_time));
    let generation_time = ttf.map(|t| total_time - t);

    eprintln!("\n[LLM] ===== 耗时统计 =====");
    eprintln!("[LLM] 总耗时: {:?}", total_time);
    if let Some(t) = ttf {
        eprintln!("[LLM] 首Token时间 (TTFT): {:?}", t);
    }
    if let Some(gt) = generation_time {
        if token_count > 0 {
            let ms_per_token = gt.as_millis() as f64 / token_count as f64;
            let tokens_per_sec = token_count as f64 / gt.as_secs_f64();
            eprintln!("[LLM] 生成耗时: {:?}", gt);
            eprintln!("[LLM] Token数量: {}", token_count);
            eprintln!("[LLM] 速度: {:.2} ms/token, {:.2} tokens/s", ms_per_token, tokens_per_sec);
        }
    }
    eprintln!("[LLM] ==================");

    Ok(())
}
