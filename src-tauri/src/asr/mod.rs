//! ASR 语音识别模块
//! 基于系统原生语音识别能力（macOS / Windows）

use crate::config::AppConfig;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn asr_transcribe_wav(wav_path: *const std::os::raw::c_char) -> *mut std::os::raw::c_char;
    fn asr_string_free(ptr: *mut std::os::raw::c_char);
}

#[cfg(target_os = "macos")]
pub fn map_macos_asr_error(err: &str) -> String {
    if err.is_empty() {
        "macOS 系统语音识别失败".to_string()
    } else if err.contains("timeout") {
        "语音识别超时，请重试".to_string()
    } else if err.contains("authorization denied status=1")
        || err.contains("authorization denied status=2")
    {
        "系统语音识别权限被拒绝，请在 系统设置 > 隐私与安全性 > 语音识别 中允许当前应用".to_string()
    } else if err.contains("authorization denied status=3") {
        "设备受限，无法使用系统语音识别（restricted）".to_string()
    } else if err.contains("isAvailable=false") || err.contains("speech recognizer unavailable") {
        "当前系统语音识别服务不可用，请稍后重试".to_string()
    } else if err.contains("No speech detected")
        || err.contains("Code=1110")
        || err.contains("code=1110")
    {
        "未检测到可识别语音，请说话更清晰并靠近麦克风".to_string()
    } else if (err.contains("domain=AVFoundationErrorDomain") && err.contains("code=-11800"))
        || err.contains("unknown error occurred (-17913)")
    {
        "macOS 语音服务不可用（AVFoundation -11800/-17913）。请确认已在 系统设置 > 隐私与安全性 > 语音识别 中允许 desktop-pet，并重启应用后重试".to_string()
    } else if err.contains("domain=kAFAssistantErrorDomain")
        || err.contains("domain=SFSpeechErrorDomain")
        || err.contains("domain=NSURLErrorDomain")
    {
        format!("macOS 系统语音识别失败（系统服务错误）: {}", err)
    } else {
        format!("macOS 系统语音识别失败: {}", err)
    }
}

/// 统一ASR引擎Trait
pub trait AsrEngine: Send {
    /// 语音识别
    /// audio_data: 16kHz 采样率, 单声道, f32 格式的 PCM 数据
    fn transcribe(&mut self, audio_data: &[f32]) -> Result<String, String>;

    /// 获取引擎名称
    fn name(&self) -> &str;

    /// 系统ASR默认就绪（可用性在调用时判定）
    fn is_model_ready(&self) -> bool;

    /// 系统ASR不需要下载模型
    fn download_model(&mut self) -> Result<(), String>;
}

pub struct SystemAsrEngine;

impl SystemAsrEngine {
    pub fn new() -> Self {
        Self
    }

    fn write_temp_wav(audio_data: &[f32]) -> Result<PathBuf, String> {
        let mut path = std::env::temp_dir();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();
        path.push(format!("desktop-pet-asr-{}.wav", ts));

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(&path, spec)
            .map_err(|e| format!("创建临时音频文件失败: {}", e))?;

        for &sample in audio_data {
            let clamped = sample.clamp(-1.0, 1.0);
            let s = (clamped * i16::MAX as f32) as i16;
            writer
                .write_sample(s)
                .map_err(|e| format!("写入临时音频失败: {}", e))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("完成音频文件失败: {}", e))?;

        Ok(path)
    }

    #[cfg(target_os = "macos")]
    fn transcribe_macos(wav_path: &PathBuf) -> Result<String, String> {
        if !wav_path.exists() {
            return Err("临时音频文件不存在".to_string());
        }
        let meta = fs::metadata(wav_path).map_err(|e| format!("读取临时音频信息失败: {}", e))?;
        if meta.len() == 0 {
            return Err("临时音频文件为空".to_string());
        }

        let c_path = std::ffi::CString::new(wav_path.to_string_lossy().to_string())
            .map_err(|e| format!("音频路径包含非法字符: {}", e))?;

        let raw = unsafe { asr_transcribe_wav(c_path.as_ptr()) };
        if raw.is_null() {
            return Err("macOS 系统语音识别失败：桥接返回空结果".to_string());
        }

        let result = unsafe {
            let s = std::ffi::CStr::from_ptr(raw).to_string_lossy().into_owned();
            asr_string_free(raw);
            s
        };

        if let Some(err) = result.strip_prefix("ERR:") {
            return Err(map_macos_asr_error(err.trim()));
        }

        let text = result
            .strip_prefix("OK:")
            .unwrap_or(&result)
            .trim()
            .to_string();
        if text.is_empty() {
            return Err("未识别到有效语音，请重试".to_string());
        }

        Ok(text)
    }

    #[cfg(target_os = "windows")]
    fn transcribe_windows(wav_path: &PathBuf) -> Result<String, String> {
        let escaped = wav_path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            r#"
Add-Type -AssemblyName System.Speech
$engine = New-Object System.Speech.Recognition.SpeechRecognitionEngine([System.Globalization.CultureInfo]::GetCultureInfo("zh-CN"))
$engine.LoadGrammar((New-Object System.Speech.Recognition.DictationGrammar))
$engine.SetInputToWaveFile("{}")
$result = $engine.Recognize()
if ($null -eq $result) {{ exit 4 }}
Write-Output $result.Text
"#,
            escaped
        );

        let output = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(script)
            .output()
            .map_err(|e| format!("执行系统语音识别失败: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if err.is_empty() {
                "Windows 系统语音识别失败".to_string()
            } else {
                format!("Windows 系统语音识别失败: {}", err)
            });
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            return Err("未识别到有效语音，请重试".to_string());
        }

        Ok(text)
    }
}

impl AsrEngine for SystemAsrEngine {
    fn transcribe(&mut self, audio_data: &[f32]) -> Result<String, String> {
        let wav_path = Self::write_temp_wav(audio_data)?;

        let result = {
            #[cfg(target_os = "macos")]
            {
                Self::transcribe_macos(&wav_path)
            }

            #[cfg(target_os = "windows")]
            {
                Self::transcribe_windows(&wav_path)
            }

            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            {
                Err("当前平台不支持系统ASR".to_string())
            }
        };

        let _ = fs::remove_file(&wav_path);
        result
    }

    fn name(&self) -> &str {
        "system-asr"
    }

    fn is_model_ready(&self) -> bool {
        true
    }

    fn download_model(&mut self) -> Result<(), String> {
        Err("系统ASR不需要下载模型".to_string())
    }
}

/// 工厂模式：创建ASR引擎
pub fn create_engine(config: &AppConfig) -> Result<Box<dyn AsrEngine>, String> {
    if config.asr.provider != "system" {
        return Err(format!(
            "不支持的ASR提供商: {}（当前仅支持 system）",
            config.asr.provider
        ));
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        Ok(Box::new(SystemAsrEngine::new()))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("当前平台不支持系统ASR".to_string())
    }
}
