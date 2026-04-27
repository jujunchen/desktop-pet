//! 音频录制模块
//! 使用 cpal 库进行麦克风录音，支持静音检测自动停止

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig, SupportedStreamConfig};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
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
    recording_thread: Option<JoinHandle<()>>,
    stop_signal: Option<mpsc::Sender<()>>,
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
            recording_thread: None,
            stop_signal: None,
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
    fn get_supported_config(device: &Device) -> Result<SupportedStreamConfig, String> {
        let supported_configs = device
            .supported_input_configs()
            .map_err(|e| format!("获取支持的配置失败: {}", e))?;

        // 优先选择 16kHz 配置
        for config in supported_configs {
            let sample_rate = config.min_sample_rate().0;
            if sample_rate == 16000 && config.channels() == 1 {
                return Ok(config.with_sample_rate(cpal::SampleRate(16000)));
            }
        }

        // 如果没有16kHz，尝试找到可用的配置
        let config = device
            .default_input_config()
            .map_err(|e| format!("获取默认配置失败: {}", e))?;

        Ok(config)
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

        // 预检查设备与配置，尽早返回错误
        let device = Self::get_default_device()?;
        let supported_config = Self::get_supported_config(&device)?;
        let sample_rate = supported_config.sample_rate().0;
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.config();
        drop(device);

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
        let (stop_tx, stop_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();

        let handle = thread::spawn(move || {
            let device = match Self::get_default_device() {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("录音启动失败: {}", e);
                    let _ = ready_tx.send(Err(e));
                    if let Ok(mut s) = state_clone.lock() {
                        *s = RecordingState::Idle;
                    }
                    return;
                }
            };

            let err_fn = move |err| {
                eprintln!("音频流错误: {}", err);
            };
            let state_f32 = Arc::clone(&state_clone);
            let state_i16 = Arc::clone(&state_clone);
            let state_u16 = Arc::clone(&state_clone);

            let stream = match sample_format {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    &config,
                    move |data: &[f32], _: &_| {
                        let mut samples = Vec::new();
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

                        let rms = Self::calculate_rms(&samples);
                        let now = Instant::now();

                        if rms > config_clone.silence_threshold {
                            *last_voice_time_clone.lock().unwrap() = Some(now);
                        }

                        if let (Some(last_voice), Some(start)) = (
                            *last_voice_time_clone.lock().unwrap(),
                            *start_time_clone.lock().unwrap(),
                        ) {
                            let silence_elapsed = now.duration_since(last_voice);
                            let total_elapsed = now.duration_since(start);

                            if silence_elapsed > Duration::from_millis(config_clone.silence_duration_ms)
                                || total_elapsed > Duration::from_millis(config_clone.max_recording_duration_ms)
                            {
                                *state_f32.lock().unwrap() = RecordingState::Stopping;
                            }
                        }

                        if let Ok(mut buffer) = audio_data_clone.lock() {
                            buffer.extend(samples);
                        }
                    },
                    err_fn,
                    None,
                ),
                cpal::SampleFormat::I16 => device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &_| {
                        let mut samples: Vec<f32> =
                            data.iter().map(|&s| s as f32 / 32768.0).collect();

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

                        let rms = Self::calculate_rms(&samples);
                        let now = Instant::now();

                        if rms > config_clone.silence_threshold {
                            *last_voice_time_clone.lock().unwrap() = Some(now);
                        }

                        if let (Some(last_voice), Some(start)) = (
                            *last_voice_time_clone.lock().unwrap(),
                            *start_time_clone.lock().unwrap(),
                        ) {
                            let silence_elapsed = now.duration_since(last_voice);
                            let total_elapsed = now.duration_since(start);

                            if silence_elapsed > Duration::from_millis(config_clone.silence_duration_ms)
                                || total_elapsed > Duration::from_millis(config_clone.max_recording_duration_ms)
                            {
                                *state_i16.lock().unwrap() = RecordingState::Stopping;
                            }
                        }

                        if let Ok(mut buffer) = audio_data_clone.lock() {
                            buffer.extend(samples);
                        }
                    },
                    err_fn,
                    None,
                ),
                cpal::SampleFormat::U16 => device.build_input_stream(
                    &config,
                    move |data: &[u16], _: &_| {
                        let mut samples: Vec<f32> = data
                            .iter()
                            .map(|&s| (s as f32 - 32768.0) / 32768.0)
                            .collect();

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

                        let rms = Self::calculate_rms(&samples);
                        let now = Instant::now();

                        if rms > config_clone.silence_threshold {
                            *last_voice_time_clone.lock().unwrap() = Some(now);
                        }

                        if let (Some(last_voice), Some(start)) = (
                            *last_voice_time_clone.lock().unwrap(),
                            *start_time_clone.lock().unwrap(),
                        ) {
                            let silence_elapsed = now.duration_since(last_voice);
                            let total_elapsed = now.duration_since(start);

                            if silence_elapsed > Duration::from_millis(config_clone.silence_duration_ms)
                                || total_elapsed > Duration::from_millis(config_clone.max_recording_duration_ms)
                            {
                                *state_u16.lock().unwrap() = RecordingState::Stopping;
                            }
                        }

                        if let Ok(mut buffer) = audio_data_clone.lock() {
                            buffer.extend(samples);
                        }
                    },
                    err_fn,
                    None,
                ),
                _ => {
                    let _ = ready_tx.send(Err("不支持的音频采样格式".to_string()));
                    if let Ok(mut s) = state_clone.lock() {
                        *s = RecordingState::Idle;
                    }
                    return;
                }
            };

            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("创建音频流失败: {}", e);
                    let _ = ready_tx.send(Err(format!("创建音频流失败: {}", e)));
                    if let Ok(mut s) = state_clone.lock() {
                        *s = RecordingState::Idle;
                    }
                    return;
                }
            };

            if let Err(e) = stream.play() {
                eprintln!("启动音频流失败: {}", e);
                let _ = ready_tx.send(Err(format!(
                    "启动音频流失败，请检查麦克风权限: {}",
                    e
                )));
                if let Ok(mut s) = state_clone.lock() {
                    *s = RecordingState::Idle;
                }
                return;
            }

            let _ = ready_tx.send(Ok(()));

            loop {
                if stop_rx.try_recv().is_ok() {
                    break;
                }

                let should_stop = state_clone
                    .lock()
                    .map(|s| *s == RecordingState::Stopping || *s == RecordingState::Idle)
                    .unwrap_or(true);

                if should_stop {
                    break;
                }

                thread::sleep(Duration::from_millis(50));
            }
        });

        match ready_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(())) => {
                self.stop_signal = Some(stop_tx);
                self.recording_thread = Some(handle);
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = handle.join();
                Err(e)
            }
            Err(_) => {
                let _ = stop_tx.send(());
                let _ = handle.join();
                *self.state.lock().map_err(|e| e.to_string())? = RecordingState::Idle;
                Err("启动录音超时，请检查麦克风权限后重试".to_string())
            }
        }
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
        let start = (*start_time)?;
        Some(start.elapsed().as_millis() as u64)
    }

    /// 停止录音并返回音频数据（16kHz, 单声道, f32）
    pub fn stop_recording(&mut self) -> Result<Vec<f32>, String> {
        // 通知后台线程停止并等待回收
        if let Some(tx) = self.stop_signal.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.recording_thread.take() {
            let _ = handle.join();
        }

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
