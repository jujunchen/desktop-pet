# 语音唤醒+语音助手 功能开发计划

## Context

当前桌面宠物已实现基础的状态机（sitting/sleeping）、系统空闲检测（Windows/macOS）、配置管理等功能。现在需要新增**语音唤醒+语音助手**功能：

- 用户叫宠物名字 → 宠物被唤醒 → 接收语音指令 → ASR转文字 → LLM对话 → 宠物回复+动作
- 无麦克风时自动降级为文本输入模式

## 推荐实现方案

### 整体架构

```
前端 (Vue)                          后端 (Rust)
┌──────────────────┐  IPC  ┌──────────────────────┐
│ 状态机扩展       │──────▶│  音频录制 (cpal)     │
│ 麦克风检测       │◀──────│  ASR 语音转文字      │
│ 文本输入UI       │       │  LLM 聊天API         │
└──────────────────┘       └──────────────────────┘
```

### Phase 1: 基础文本对话（高优先级，先做）

**目标：先实现文本模式对话，验证完整LLM流程**

#### 1. 后端 LLM 模块
- 文件：`src-tauri/src/llm/mod.rs`（新增）
- 功能：
  - 从配置读取LLM参数（api_key, model, base_url）
  - 调用OpenAI兼容API
  - 返回回复文本和可选动作指令
- 复用现有：`config.rs` 中的 `LlmConfig`

#### 2. 后端注册 Command
- 文件：`src-tauri/src/lib.rs`（修改）
- 新增：`chat_with_llm(prompt: String) -> Result<ChatResponse, String>`

#### 3. 前端语音助手 Composable
- 文件：`src/composables/useVoiceAssistant.ts`（新增）
- 功能：
  - 麦克风状态检测
  - 封装 `chat_with_llm` 调用
  - 文本模式 `sendTextMessage(text: string)`

#### 4. 文本输入 Modal 组件
- 文件：`src/components/TextInputModal.vue`（新增）
- UI设计：
  - 居中弹出框，带标题、多行输入框、取消/发送按钮
  - Enter发送，Shift+Enter换行，ESC关闭

#### 5. 扩展状态机
- 文件：`src/composables/usePetState.ts`（修改）
- 新增：
  - `enterProcessingState()` - 思考状态（复用tilt-head）
  - `enterSpeakingState(text, duration)` - 说话状态（显示文字气泡）
  - `showSpeech(text, duration)` - 显示聊天气泡

#### 6. 集成到 App.vue
- 文件：`src/App.vue`（修改）
- 功能：
  - 右键菜单新增「文本对话」选项
  - 双击宠物打开文本输入框（无麦克风时自动）

---

### Phase 2: 麦克风+ASR（中优先级，二期做）

**目标：实现语音录制和语音转文字**

#### 1. 音频录制模块
- 文件：`src-tauri/src/audio/recording.rs`（新增）
- 依赖：`cpal`
- 功能：
  - `check_microphone_available()` - 检测麦克风
  - `start_recording()` - 开始录音
  - `stop_recording()` - 返回WAV音频数据

#### 2. ASR 模块
- 文件：`src-tauri/src/asr/mod.rs`（新增）
- 依赖：`whisper-rs` 或在线API
- 功能：
  - `transcribe_audio(audio_data, config)` - 语音转文字
  - 支持本地Whisper和在线API（阿里百炼/火山方舟等）

#### 3. 新增聆听状态GIF
- 资源：`src/assets/pets/dog/listening.gif`
- 在 `usePetState.ts` 中注册 listening 状态

---

### Phase 3: 唤醒词检测（低优先级，三期做）

**目标：实现"叫名字唤醒"功能**

#### 1. 唤醒词检测模块
- 文件：`src-tauri/src/audio/wakeword.rs`（新增）
- 依赖：`rustpotter`（推荐）或 `webrtc-vad` + 首段ASR匹配
- 功能：
  - 持续监听麦克风
  - 检测到"小白"时发送事件到前端
  - 自动开始录制后续指令

#### 2. 完整语音流程
- 唤醒 → 显示"在呢" → 聆听 → 语音转文字 → LLM → 回复
- 超时未说话自动回到待机

---

## 关键文件清单

| 路径 | 说明 | 修改类型 |
|------|------|----------|
| `src-tauri/Cargo.toml` | 添加依赖（cpal, whisper-rs, reqwest等） | 修改 |
| `src-tauri/src/lib.rs` | 注册新 commands | 修改 |
| `src-tauri/src/llm/mod.rs` | LLM 聊天 API | 新增 |
| `src-tauri/src/audio/mod.rs` | 音频模块入口 | 新增 |
| `src-tauri/src/audio/recording.rs` | 麦克风录制 | 新增 |
| `src-tauri/src/asr/mod.rs` | ASR 语音转文字 | 新增 |
| `src/composables/usePetState.ts` | 扩展状态机 | 修改 |
| `src/composables/useVoiceAssistant.ts` | 语音助手逻辑 | 新增 |
| `src/components/TextInputModal.vue` | 文本输入UI | 新增 |
| `src/App.vue` | 集成语音助手 | 修改 |
| `src/SettingsApp.vue` | 新增ASR配置 | 修改 |

## 测试验证

### Phase 1 验收
1. [ ] 右键 → 文本对话 → 弹出输入框
2. [ ] 输入"你好" → 宠物显示思考状态 → 收到LLM回复
3. [ ] 回复显示在文字气泡，宠物播放happy动作
4. [ ] ESC键可关闭输入框

### Phase 2 验收
1. [ ] 启动应用自动检测麦克风状态
2. [ ] 点击语音按钮开始录音，显示listening GIF
3. [ ] 说话后停止录音，正确转成文字
4. [ ] 发送给LLM并收到回复

### Phase 3 验收
1. [ ] 喊"小白" → 宠物被唤醒，显示"在呢"
2. [ ] 接着说指令 → 转文字 → LLM回复
3. [ ] 10秒无说话自动回到待机状态
4. [ ] 无麦克风时自动提示使用文本模式

## 无麦克风降级策略

1. 应用启动时检测麦克风
2. 无麦克风/无权限时：
   - 右键菜单显示「文本对话」而非「语音对话」
   - 双击宠物直接打开文本输入框
   - 设置页显示当前麦克风状态
3. 设置项：「始终使用文本模式」开关

---

## 开发顺序建议

**第一阶段（本周）**：只做Phase 1文本对话，验证从输入到LLM回复的完整流程

**第二阶段（下周）**：Phase 2麦克风+ASR，实现语音转文字

**第三阶段（后续）**：Phase 3唤醒词检测，实现真正的"叫名字唤醒"

这样可以分阶段验证，每阶段都有可用功能。
