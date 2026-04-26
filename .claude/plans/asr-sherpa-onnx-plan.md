# ASR模块改用本地Sherpa-Onnx实现计划

## Context

当前桌面宠物项目的ASR模块设计支持多提供商（Whisper本地、阿里百炼、火山引擎、FunASR等）。根据需求，现在需要改为**仅使用本地sherpa-onnx**，移除所有在线ASR选项。同时代码架构要保持良好的扩展性，方便未来添加新的ASR提供商。

### Sherpa-Onnx优势
- **纯本地运行**：无需网络，隐私安全
- **轻量快速**：比Whisper.cpp更快，CPU友好
- **多语言支持**：支持中文等多种语言
- **模型体积小**：tiny模型仅~100MB
- **Rust绑定完善**：官方支持sherpa-rs

---

## 实施步骤

### 1. 更新Cargo.toml依赖
**文件**: `src-tauri/Cargo.toml`

添加依赖：
```toml
# Sherpa ONNX 语音识别
sherpa-rs = { version = "0.1", features = ["check-download"] }

# 音频录制
cpal = "0.15"
hound = "3.5"

# 异步trait
async-trait = "0.1"

# 音频处理
rubato = "0.12"  # 重采样（如果需要）
```

---

### 2. 修改ASR配置结构
**文件**: `src-tauri/src/config.rs`

**变更内容**:
- 移除 `dashscope`, `volcengine`, `funasr` 字段
- 将 `whisper_local` 改为 `sherpa_onnx`
- 保留 `OnlineAsrConfig` 结构体（为了扩展，暂时保留）
- provider 默认值改为 `"sherpa-onnx"`

**新结构**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrConfig {
    #[serde(default = "default_asr_provider")]
    pub provider: String,  // 当前仅支持 "sherpa-onnx"
    
    #[serde(default)]
    pub sherpa_onnx: SherpaOnnxConfig,
}

fn default_asr_provider() -> String {
    "sherpa-onnx".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SherpaOnnxConfig {
    #[serde(default = "default_model_size")]
    pub model_size: String,  // tiny / base / small
    
    #[serde(default)]
    pub model_dir: String,   // 模型存放目录，空则用默认位置
    
    #[serde(default = "default_language")]
    pub language: String,    // zh / en
}

fn default_model_size() -> String { "tiny".to_string() }
fn default_language() -> String { "zh".to_string() }

impl Default for SherpaOnnxConfig {
    fn default() -> Self {
        Self {
            model_size: default_model_size(),
            model_dir: String::new(),
            language: default_language(),
        }
    }
}
```

---

### 3. 实现ASR引擎模块
**文件**: `src-tauri/src/asr/mod.rs` (新建)

**架构设计**:
```rust
// 统一ASR引擎Trait（保持扩展性）
#[async_trait]
pub trait AsrEngine: Send + Sync {
    async fn transcribe(&mut self, audio_data: &[f32]) -> Result<String, String>;
    fn name(&self) -> &str;
}

// Sherpa-Onnx 本地引擎实现
pub struct SherpaOnnxEngine {
    recognizer: sherpa_rs::Recognizer,
    model_path: PathBuf,
}

impl SherpaOnnxEngine {
    pub fn new(config: &SherpaOnnxConfig) -> Result<Self, String> {
        // 1. 确定模型路径
        // 2. 检查模型是否存在，不存在则自动下载
        // 3. 初始化sherpa识别器
    }
    
    fn ensure_model_exists(&mut self) -> Result<(), String> {
        // 自动从 ModelScope / HuggingFace 下载模型
        // 下载到 ~/.cache/desktop-pet/models/
    }
}

#[async_trait]
impl AsrEngine for SherpaOnnxEngine {
    async fn transcribe(&mut self, audio_data: &[f32]) -> Result<String, String> {
        // 调用sherpa进行语音识别
        // 注意：采样率必须是16kHz，单声道
    }
    
    fn name(&self) -> &str {
        "sherpa-onnx"
    }
}

// 工厂模式（保持可扩展架构）
pub fn create_engine(config: &AppConfig) -> Result<Box<dyn AsrEngine>, String> {
    match config.asr.provider.as_str() {
        "sherpa-onnx" => {
            let engine = SherpaOnnxEngine::new(&config.asr.sherpa_onnx)?;
            Ok(Box::new(engine))
        }
        // 未来添加新提供商在这里扩展
        // "openai-whisper" => Ok(Box::new(OpenAiWhisperEngine::new(...)))
        _ => Err(format!("不支持的ASR提供商: {}", config.asr.provider)),
    }
}
```

---

### 4. 实现音频录制模块
**文件**: `src-tauri/src/audio/mod.rs` (新建)
**文件**: `src-tauri/src/audio/recording.rs` (新建)

**功能**:
- 检测麦克风设备
- 开始/停止录音
- 自动重采样到16kHz单声道（sherpa要求）
- 输出f32格式的PCM数据

```rust
pub struct AudioRecorder {
    is_recording: bool,
    audio_data: Vec<f32>,
    stream: Option<cpal::Stream>,
}

impl AudioRecorder {
    pub fn new() -> Self { ... }
    
    pub fn start_recording(&mut self) -> Result<(), String> {
        // 使用cpal打开输入设备
        // 配置: 16kHz采样率, 单声道, f32格式
        // 收集音频数据到缓冲区
    }
    
    pub fn stop_recording(&mut self) -> Result<Vec<f32>, String> {
        // 停止录音流
        // 返回收集的音频数据
    }
    
    pub fn check_microphone_available() -> bool {
        // 检测系统是否有可用的麦克风
    }
}
```

---

### 5. 注册Tauri命令和状态管理
**文件**: `src-tauri/src/lib.rs`

**添加状态管理**:
```rust
// 在run()函数中管理ASR状态
use std::sync::Mutex;

pub struct AppState {
    asr_engine: Mutex<Option<Box<dyn AsrEngine>>>,
    audio_recorder: Mutex<AudioRecorder>,
}

// 在builder中管理状态
.manage(tauri::State::new(AppState {
    asr_engine: Mutex::new(None),
    audio_recorder: Mutex::new(AudioRecorder::new()),
}))
```

**注册命令**:
```rust
#[tauri::command]
async fn init_asr_engine(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // 加载配置，初始化ASR引擎
    // 懒加载：第一次调用时初始化
}

#[tauri::command]
async fn start_asr_recording(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // 开始录音，向前端发送事件
}

#[tauri::command]
async fn stop_asr_recording(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // 停止录音，执行识别
    // 返回识别结果文本
}

#[tauri::command]
fn check_microphone_available() -> bool {
    AudioRecorder::check_microphone_available()
}
```

**事件定义**:
- `asr:recording-started` - 录音开始
- `asr:recording-stopped` - 录音停止
- `asr:result` - 识别结果 `{ text: String }`
- `asr:error` - 错误信息 `{ message: String }`

---

### 6. 更新前端类型定义
**文件**: `src/composables/useWindowManager.ts`

**修改类型**:
```typescript
export type AsrProvider = 'sherpa-onnx'  // 只保留本地实现

export interface AppConfig {
  llm: { ... },
  asr: {
    provider: AsrProvider
    sherpa_onnx: {
      model_size: 'tiny' | 'base' | 'small'
      model_dir: string
      language: 'zh' | 'en'
    }
  },
  pet: { ... }
}

// 更新默认配置
export function getDefaultConfig(): AppConfig {
  return {
    // ...
    asr: {
      provider: 'sherpa-onnx',
      sherpa_onnx: {
        model_size: 'tiny',
        model_dir: '',
        language: 'zh'
      }
    },
    // ...
  }
}
```

---

### 7. 更新设置界面
**文件**: `src/SettingsApp.vue`

**变更**:
- 移除ASR提供商下拉选择（因为只有sherpa-onnx）
- 显示本地模型配置：
  - 模型大小选择（tiny/base/small）
  - 语言选择（中文/英文）
  - 模型路径（可自定义）
  - 下载状态显示（检测到模型不存在时显示下载按钮）

**简化后的ASR配置区**:
```vue
<section>
  <h3>语音识别（本地Sherpa-Onnx）</h3>
  
  <div class="provider-config">
    <div class="field-item">
      <label>模型大小</label>
      <select v-model="config.asr.sherpa_onnx.model_size">
        <option value="tiny">tiny（最快，约100MB）</option>
        <option value="base">base（平衡，约200MB）</option>
        <option value="small">small（精度高，约500MB）</option>
      </select>
    </div>
    
    <div class="field-item">
      <label>识别语言</label>
      <select v-model="config.asr.sherpa_onnx.language">
        <option value="zh">中文</option>
        <option value="en">英文</option>
      </select>
    </div>
    
    <div class="field-item">
      <label>模型目录（留空使用默认位置）</label>
      <input v-model="config.asr.sherpa_onnx.model_dir" placeholder="~/.cache/desktop-pet/models" />
    </div>
    
    <div v-if="!modelExists" class="model-status warning">
      <span>⚠️ 模型文件不存在，首次使用将自动下载</span>
      <button @click="downloadModel" class="btn-small" :disabled="isDownloading">
        {{ isDownloading ? '下载中...' : '立即下载模型' }}
      </button>
    </div>
    <div v-else class="model-status success">
      <span>✓ 模型已就绪</span>
    </div>
  </div>
</section>
```

---

### 8. 实现前端ASR Composable
**文件**: `src/composables/useAsr.ts` (新建)

```typescript
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ref, onMounted } from 'vue'

export const isRecording = ref(false)
export const asrResult = ref('')
export const asrError = ref('')
export const microphoneAvailable = ref(false)

export function useAsr() {
  async function init() {
    microphoneAvailable.value = await invoke('check_microphone_available')
    await invoke('init_asr_engine')
    
    // 监听ASR事件
    await listen('asr:recording-started', () => {
      isRecording.value = true
    })
    
    await listen('asr:recording-stopped', () => {
      isRecording.value = false
    })
    
    await listen('asr:result', (event: any) => {
      asrResult.value = event.payload.text
    })
    
    await listen('asr:error', (event: any) => {
      asrError.value = event.payload.message
    })
  }
  
  async function startRecording() {
    asrResult.value = ''
    asrError.value = ''
    await invoke('start_asr_recording')
  }
  
  async function stopRecording(): Promise<string> {
    return await invoke('stop_asr_recording')
  }
  
  async function toggleRecording() {
    if (isRecording.value) {
      return await stopRecording()
    } else {
      await startRecording()
    }
  }
  
  onMounted(() => {
    init()
  })
  
  return {
    isRecording,
    asrResult,
    asrError,
    microphoneAvailable,
    startRecording,
    stopRecording,
    toggleRecording,
  }
}
```

---

## 架构保持扩展性的关键点

### 1. Trait-based 设计
- `AsrEngine` trait 定义统一接口
- 所有ASR实现都遵循同一接口
- 添加新提供商只需实现trait并在工厂注册

### 2. 工厂模式
- `create_engine()` 统一创建引擎实例
- 配置驱动，无需修改调用代码

### 3. 统一配置结构
- `OnlineAsrConfig` 结构保留（暂时不用但保留）
- 未来添加在线ASR可以直接复用

### 4. 事件驱动架构
- 前端与后端通过事件通信
- 不绑定具体实现细节

---

## 验收标准

### 功能验收
- [ ] 设置界面只显示Sherpa-Onnx本地配置
- [ ] 首次使用自动下载模型（tiny模型~100MB）
- [ ] 点击录音按钮开始录制，宠物显示聆听状态
- [ ] 停止录音后正确识别中文语音
- [ ] 识别结果自动发送给LLM进行对话
- [ ] 无网络环境下ASR功能正常工作

### 架构验收
- [ ] `AsrEngine` trait 定义清晰
- [ ] `SherpaOnnxEngine` 完整实现trait
- [ ] 工厂模式 `create_engine()` 工作正常
- [ ] 代码结构支持未来扩展新ASR提供商
- [ ] 配置持久化正常工作

---

## 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src-tauri/Cargo.toml` | 修改 | 添加sherpa-rs, cpal, hound等依赖 |
| `src-tauri/src/config.rs` | 修改 | 移除在线ASR配置，添加sherpa_onnx配置 |
| `src-tauri/src/asr/mod.rs` | 新建 | ASR引擎模块，trait定义和SherpaOnnx实现 |
| `src-tauri/src/audio/mod.rs` | 新建 | 音频模块入口 |
| `src-tauri/src/audio/recording.rs` | 新建 | 麦克风录音实现 |
| `src-tauri/src/lib.rs` | 修改 | 注册ASR命令和状态管理 |
| `src/composables/useWindowManager.ts` | 修改 | 更新ASR配置类型定义 |
| `src/SettingsApp.vue` | 修改 | 简化ASR配置界面 |
| `src/composables/useAsr.ts` | 新建 | 前端ASR调用封装 |
