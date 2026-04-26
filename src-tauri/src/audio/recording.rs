//! 音频录制模块
//! 使用 cpal 库进行麦克风录音，支持静音检测自动停止

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 音频录制器配置
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    /// 静音阈值（0.0-1.0，值越小越灵敏）
    pub silence_threshold: f32,
    /// 静音持续多久停止录音（毫秒）
    pub silence_duration_ms: u64,
    /// 最长录音时间（毫秒），防止一直录
    pub max_recording_duration_ms: u64,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            silence_threshold: 0.02,
            silence_duration_ms: 1500,
            max_recording_duration_ms: 30000,
        }
    }
}

/// 录音状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecordingState {
    /// 未录音
    Idle,
    /// 正在录音
    Recording,
    /// 检测到静音，准备停止
    Stopping,
}

/// 音频录制器
pub struct AudioRecorder {
    state: Arc<Mutex<RecordingState>>,
    audio_data: Arc<Mutex<Vec<f32>>>,
    stream: Option<Stream>,
    config: RecordingConfig,
    /// 开始录音时间
    start_time: Arc<Mutex<Option<Instant>>>,
    /// 最后检测到非静音的时间
    last_voice_time: Arc<Mutex<Option<Instant>>>,
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self::with_config(RecordingConfig::default())
    }

    pub fn with_config(config: RecordingConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(RecordingState::Idle)),
            audio_data: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            config,
            start_time: Arc::new(Mutex::new(None)),
            last_voice_time: Arc::new(Mutex::new(None)),
        }
    }

    /// 检查是否有可用的麦克风
    pub fn has_microphone() -> bool {
        let host = cpal::default_host();
        host.default_input_device().is_some()
    }

    /// 获取默认输入设备
    fn get_default_device() -> Result<Device, String> {
        let host = cpal::default_host();
        host.default_input_device()
            .ok_or_else(|| "未找到麦克风设备".to_string())
    }

    /// 获取支持的配置（16kHz, 单声道, f32）
    fn get_supported_config(device: &Device) -> Result<StreamConfig, String> {
        let supported_configs = device
            .supported_input_configs()
            .map_err(|e| format!("获取支持的配置失败: {}", e))?;

        // 优先选择 16kHz 配置
        for config in supported_configs {
            let sample_rate = config.min_sample_rate().0;
            if sample_rate == 16000 && config.channels() == 1 {
                return Ok(config.with_sample_rate(cpal::SampleRate(16000)).config());
            }
        }

        // 如果没有16kHz，尝试找到可用的配置
        let config = device
            .default_input_config()
            .map_err(|e| format!("获取默认配置失败: {}", e))?;

        Ok(config.config())
    }

    /// 计算音频音量（RMS）
    fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }

    /// 开始录音（带静音检测）
    /// 返回：是否检测到声音
    pub fn start_recording(&mut self) -> Result<(), String> {
        if self.is_recording() {
            return Ok(());
        }

        let device = Self::get_default_device()?;
        let config = Self::get_supported_config(&device)?;
        let sample_rate = config.sample_rate.0;

        // 清空音频数据
        self.audio_data.lock().map_err(|e| e.to_string())?.clear();

        // 重置状态
        *self.state.lock().map_err(|e| e.to_string())? = RecordingState::Recording;
        *self.start_time.lock().map_err(|e| e.to_string())? = Some(Instant::now());
        *self.last_voice_time.lock().map_err(|e| e.to_string())? = Some(Instant::now());

        let audio_data_clone = Arc::clone(&self.audio_data);
        let state_clone = Arc::clone(&self.state);
        let start_time_clone = Arc::clone(&self.start_time);
        let last_voice_time_clone = Arc::clone(&self.last_voice_time);
        let config_clone = self.config.clone();

        let err_fn = move |err| {
            eprintln!("音频流错误: {}", err);
        };

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _: &_| {
                    let mut samples = Vec::new();
                    // 重采样到 16kHz（简化处理）
                    if sample_rate == 16000 {
                        samples.extend_from_slice(data);
                    } else {
                        let ratio = sample_rate as f32 / 16000.0;
                        let mut i = 0.0;
                        while i < data.len() as f32 {
                            samples.push(data[i as usize]);
                            i += ratio;
                        }
                    }

                    // 检测音量
                    let rms = Self::calculate_rms(&samples);
                    let now = Instant::now();

                    if rms > config_clone.silence_threshold {
                        // 检测到声音，更新最后说话时间
                        *last_voice_time_clone.lock().unwrap() = Some(now);
                    }

                    // 检查静音超时或最大录音时间
                    if let (Some(last_voice), Some(start)) = (
                        *last_voice_time_clone.lock().unwrap(),
                        *start_time_clone.lock().unwrap()
                    ) {
                        let silence_elapsed = now.duration_since(last_voice);
                        let total_elapsed = now.duration_since(start);

                        // 静音超时或达到最大录音时间，准备停止
                        if silence_elapsed > Duration::from_millis(config_clone.silence_duration_ms)
                            || total_elapsed > Duration::from_millis(config_clone.max_recording_duration_ms)
                        {
                            *state_clone.lock().unwrap() = RecordingState::Stopping;
                        }
                    }

                    // 收集音频数据
                    if let Ok(mut buffer) = audio_data_clone.lock() {
                        buffer.extend(samples);
                    }
                },
                err_fn,
                None,
            ).map_err(|e| format!("创建音频流失败: {}", e))?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _: &_| {
                    // i16 转 f32，归一化到 [-1, 1]
                    let mut samples: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();

                    // 重采样到 16kHz（简化处理）
                    if sample_rate != 16000 {
                        let ratio = sample_rate as f32 / 16000.0;
                        let mut resampled = Vec::new();
                        let mut i = 0.0;
                        while i < samples.len() as f32 {
                            resampled.push(samples[i as usize]);
                            i += ratio;
                        }
                        samples = resampled;
                    }

                    // 检测音量
                    let rms = Self::calculate_rms(&samples);
                    let now = Instant::now();

                    if rms > config_clone.silence_threshold {
                        *last_voice_time_clone.lock().unwrap() = Some(now);
                    }

                    // 检查超时
                    if let (Some(last_voice), Some(start)) = (
                        *last_voice_time_clone.lock().unwrap(),
                        *start_time_clone.lock().unwrap()
                    ) {
                        let silence_elapsed = now.duration_since(last_voice);
                        let total_elapsed = now.duration_since(start);

                        if silence_elapsed > Duration::from_millis(config_clone.silence_duration_ms)
                            || total_elapsed > Duration::from_millis(config_clone.max_recording_duration_ms)
                        {
                            *state_clone.lock().unwrap() = RecordingState::Stopping;
                        }
                    }

                    if let Ok(mut buffer) = audio_data_clone.lock() {
                        buffer.extend(samples);
                    }
                },
                err_fn,
                None,
            ).map_err(|e| format!("创建音频流失败: {}", e))?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _: &_| {
                    // u16 转 f32，归一化到 [-1, 1]
                    let mut samples: Vec<f32> = data
                        .iter()
                        .map(|&s| (s as f32 - 32768.0) / 32768.0)
                        .collect();

                    // 重采样到 16kHz（简化处理）
                    if sample_rate != 16000 {
                        let ratio = sample_rate as f32 / 16000.0;
                        let mut resampled = Vec::new();
                        let mut i = 0.0;
                        while i < samples.len() as f32 {
                            resampled.push(samples[i as usize]);
                            i += ratio;
                        }
                        samples = resampled;
                    }

                    // 检测音量
                    let rms = Self::calculate_rms(&samples);
                    let now = Instant::now();

                    if rms > config_clone.silence_threshold {
                        *last_voice_time_clone.lock().unwrap() = Some(now);
                    }

                    // 检查超时
                    if let (Some(last_voice), Some(start)) = (
                        *last_voice_time_clone.lock().unwrap(),
                        *start_time_clone.lock().unwrap()
                    ) {
                        let silence_elapsed = now.duration_since(last_voice);
                        let total_elapsed = now.duration_since(start);

                        if silence_elapsed > Duration::from_millis(config_clone.silence_duration_ms)
                            || total_elapsed > Duration::from_millis(config_clone.max_recording_duration_ms)
                        {
                            *state_clone.lock().unwrap() = RecordingState::Stopping;
                        }
                    }

                    if let Ok(mut buffer) = audio_data_clone.lock() {
                        buffer.extend(samples);
                    }
                },
                err_fn,
                None,
            ).map_err(|e| format!("创建音频流失败: {}", e))?,
            _ => return Err("不支持的音频格式".to_string()),
        };

        stream.play().map_err(|e| format!("启动音频流失败: {}", e))?;

        self.stream = Some(stream);

        Ok(())
    }

    /// 检查是否应该自动停止录音
    pub fn should_stop(&self) -> bool {
        *self.state.lock().unwrap() == RecordingState::Stopping
    }

    /// 当前是否正在录音
    pub fn is_recording(&self) -> bool {
        *self.state.lock().unwrap() == RecordingState::Recording
    }

    /// 获取已录制时长（毫秒）
    pub fn get_recording_duration_ms(&self) -> Option<u64> {
        let start_time = self.start_time.lock().ok()?;
        let start = *start_time?;
        Some(start.elapsed().as_millis() as u64)
    }

    /// 停止录音并返回音频数据（16kHz, 单声道, f32）
    pub fn stop_recording(&mut self) -> Result<Vec<f32>, String> {
        // 停止流
        self.stream.take();

        *self.state.lock().map_err(|e| e.to_string())? = RecordingState::Idle;

        // 获取音频数据
        let data = self
            .audio_data
            .lock()
            .map_err(|e| e.to_string())?
            .clone();

        Ok(data)
    }

    /// 强制停止录音（不检查状态）
    pub fn force_stop(&mut self) -> Result<Vec<f32>, String> {
        self.stop_recording()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_creation() {
        let recorder = AudioRecorder::new();
        assert!(!recorder.is_recording());
    }

    #[test]
    fn test_calculate_rms() {
        let silent = &[0.0, 0.0, 0.0];
        assert_eq!(AudioRecorder::calculate_rms(silent), 0.0);

        let loud = &[0.5, 0.5, 0.5];
        assert!((AudioRecorder::calculate_rms(loud) - 0.5).abs() < 0.001);
    }
}
