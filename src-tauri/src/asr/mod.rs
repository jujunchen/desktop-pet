//! ASR 语音识别模块
//! 基于 Sherpa-ONNX 实现本地语音识别
//! 使用 SenseVoice 多语言模型

use crate::config::{AppConfig, SherpaOnnxConfig};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// 统一ASR引擎Trait
#[async_trait]
pub trait AsrEngine: Send + Sync {
    /// 语音识别
    /// audio_data: 16kHz 采样率, 单声道, f32 格式的 PCM 数据
    async fn transcribe(&mut self, audio_data: &[f32]) -> Result<String, String>;

    /// 获取引擎名称
    fn name(&self) -> &str;

    /// 检查模型是否已下载
    fn is_model_ready(&self) -> bool;

    /// 下载模型（如果需要）
    async fn download_model(&mut self) -> Result<(), String>;
}

/// Sherpa-ONNX 本地ASR引擎（使用SenseVoice模型）
pub struct SherpaOnnxEngine {
    config: SherpaOnnxConfig,
    model_path: PathBuf,
    recognizer: Option<sherpa_onnx::OfflineRecognizer>,
    is_model_ready: bool,
}

impl SherpaOnnxEngine {
    /// 创建新的 Sherpa-ONNX 引擎
    pub fn new(config: &SherpaOnnxConfig) -> Result<Self, String> {
        let model_path = Self::get_model_path(config);
        let is_model_ready = Self::check_model_exists(&model_path);

        let recognizer = if is_model_ready {
            Self::create_recognizer(&model_path, config.num_threads)?
        } else {
            // 模型未准备好时返回空Option
        };

        Ok(Self {
            config: config.clone(),
            model_path,
            recognizer,
            is_model_ready,
        })
    }

    /// 获取模型存储路径
    fn get_model_path(config: &SherpaOnnxConfig) -> PathBuf {
        if !config.model_dir.is_empty() {
            Path::new(&config.model_dir).to_path_buf()
        } else {
            // 使用平台标准配置目录
            // Windows: %APPDATA%\desktop-pet\asr-models\sense-voice
            // macOS: ~/Library/Application Support/desktop-pet/asr-models/sense-voice
            // Linux: ~/.config/desktop-pet/asr-models/sense-voice
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("desktop-pet")
                .join("asr-models")
                .join("sense-voice")
        }
    }

    /// 检查模型文件是否存在
    fn check_model_exists(path: &Path) -> bool {
        // SenseVoice 模型文件
        let model = path.join("model.int8.onnx");
        let tokens = path.join("tokens.txt");

        model.exists() && tokens.exists()
    }

    /// 创建离线识别器
    fn create_recognizer(
        model_path: &Path,
        num_threads: i32,
    ) -> Result<Option<sherpa_onnx::OfflineRecognizer>, String> {
        // SenseVoice 模型配置
        let sense_voice_config = sherpa_onnx::OfflineSenseVoiceModelConfig {
            model: model_path.join("model.int8.onnx").to_string_lossy().to_string(),
            ..Default::default()
        };

        let model_config = sherpa_onnx::OfflineModelConfig {
            sense_voice: Some(sense_voice_config),
            tokens: model_path.join("tokens.txt").to_string_lossy().to_string(),
            num_threads,
            ..Default::default()
        };

        let recognizer_config = sherpa_onnx::OfflineRecognizerConfig {
            model: model_config,
            ..Default::default()
        };

        let recognizer = sherpa_onnx::OfflineRecognizer::new(recognizer_config)
            .map_err(|e| format!("初始化识别器失败: {}", e))?;

        Ok(Some(recognizer))
    }

    /// 初始化识别器（如果已在new时初始化）
    fn init_recognizer(&mut self) -> Result<(), String> {
        if self.recognizer.is_some() {
            return Ok(());
        }

        if !self.is_model_ready {
            return Err("模型文件不存在，请先下载模型".to_string());
        }

        self.recognizer = Self::create_recognizer(&self.model_path, self.config.num_threads)?;
        Ok(())
    }
}

#[async_trait]
impl AsrEngine for SherpaOnnxEngine {
    async fn transcribe(&mut self, audio_data: &[f32]) -> Result<String, String> {
        if self.recognizer.is_none() {
            self.init_recognizer()?;
        }

        let recognizer = self
            .recognizer
            .as_mut()
            .ok_or_else(|| "识别器未初始化".to_string())?;

        // 创建识别流
        let mut stream = recognizer.create_stream();

        // 接受音频数据（采样率必须是 16kHz）
        stream.accept_waveform(16000.0, audio_data);

        // 执行解码
        recognizer.decode(&mut stream);

        // 获取结果
        let result = stream.get_result();

        Ok(result)
    }

    fn name(&self) -> &str {
        "sherpa-onnx-sense-voice"
    }

    fn is_model_ready(&self) -> bool {
        self.is_model_ready
    }

    async fn download_model(&mut self) -> Result<(), String> {
        Err("自动下载功能已移除，请手动下载SenseVoice模型到指定目录".to_string())
    }
}

/// 工厂模式：创建ASR引擎
pub fn create_engine(config: &AppConfig) -> Result<Box<dyn AsrEngine>, String> {
    match config.asr.provider.as_str() {
        "sherpa-onnx" => {
            let engine = SherpaOnnxEngine::new(&config.asr.sherpa_onnx)?;
            Ok(Box::new(engine))
        }
        _ => Err(format!("不支持的ASR提供商: {}", config.asr.provider)),
    }
}
