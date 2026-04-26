/**
 * ASR 语音识别 Composable
 */
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ref, onMounted, onUnmounted, computed } from 'vue'

// 状态
export const isRecording = ref(false)
export const asrResult = ref('')
export const asrError = ref('')
export const microphoneAvailable = ref(false)
export const modelReady = ref(false)
export const asrInitialized = ref(false)
export const isVoiceChatting = ref(false)

// 录音按钮状态文本
export const voiceButtonText = computed(() => {
  if (!microphoneAvailable.value) return '🎙️ 无麦克风'
  if (!modelReady.value) return '🎙️ 模型未就绪'
  if (isRecording.value) return '🎙️ 正在听...'
  if (isVoiceChatting.value) return '⏳ 处理中...'
  return '🎙️ 按住说话'
})

// 录音按钮是否可点击
export const canStartVoiceChat = computed(() => {
  return microphoneAvailable.value && modelReady.value && !isRecording.value && !isVoiceChatting.value
})

// 事件监听器卸载函数
const unlistenFns: Array<() => void> = []

/**
 * 初始化 ASR 引擎
 */
export async function initAsrEngine(): Promise<void> {
  if (asrInitialized.value) {
    return
  }

  try {
    microphoneAvailable.value = await invoke('check_microphone_available')
    await invoke('init_asr_engine')
    modelReady.value = await invoke('check_asr_model_ready')
    asrInitialized.value = true
  } catch (e) {
    console.error('初始化ASR引擎失败:', e)
    asrError.value = String(e)
  }
}

/**
 * 检查模型是否已下载
 */
export async function checkModelReady(): Promise<boolean> {
  try {
    modelReady.value = await invoke('check_asr_model_ready')
    return modelReady.value
  } catch (e) {
    console.error('检查模型状态失败:', e)
    return false
  }
}

/**
 * 开始录音
 */
export async function startRecording(): Promise<void> {
  if (isRecording.value) {
    return
  }

  asrResult.value = ''
  asrError.value = ''

  try {
    await invoke('start_asr_recording')
  } catch (e) {
    console.error('开始录音失败:', e)
    asrError.value = String(e)
  }
}

/**
 * 停止录音并获取识别结果
 */
export async function stopRecording(): Promise<string> {
  if (!isRecording.value) {
    return ''
  }

  try {
    const result = await invoke<string>('stop_asr_recording')
    return result
  } catch (e) {
    console.error('停止录音失败:', e)
    asrError.value = String(e)
    return ''
  }
}

/**
 * 切换录音状态
 */
export async function toggleRecording(): Promise<string | undefined> {
  if (isRecording.value) {
    return await stopRecording()
  } else {
    await startRecording()
  }
}

/**
 * 一键语音聊天（自动录音 → 静音检测 → 识别 → 自动发送给LLM）
 */
export async function startVoiceChat(): Promise<void> {
  if (!canStartVoiceChat.value) {
    return
  }

  asrResult.value = ''
  asrError.value = ''
  isVoiceChatting.value = true

  try {
    await invoke('start_voice_chat')
  } catch (e) {
    console.error('语音聊天失败:', e)
    asrError.value = String(e)
  } finally {
    isVoiceChatting.value = false
  }
}

/**
 * 设置ASR事件监听器
 */
export async function setupAsrEventListeners(): Promise<void> {
  // 清理旧的监听器
  unlistenFns.forEach(fn => fn())
  unlistenFns.length = 0

  // 录音开始事件
  const unlistenStart = await listen('asr:recording-started', () => {
    isRecording.value = true
  })
  unlistenFns.push(unlistenStart)

  // 录音停止事件
  const unlistenStop = await listen('asr:recording-stopped', () => {
    isRecording.value = false
  })
  unlistenFns.push(unlistenStop)

  // 识别结果事件
  const unlistenResult = await listen<{ text: string }>('asr:result', (event) => {
    asrResult.value = event.payload?.text || ''
  })
  unlistenFns.push(unlistenResult)

  // 错误事件
  const unlistenError = await listen<{ message: string }>('asr:error', (event) => {
    asrError.value = event.payload?.message || ''
  })
  unlistenFns.push(unlistenError)
}

/**
 * 清理ASR资源
 */
export function cleanupAsr(): void {
  unlistenFns.forEach(fn => fn())
  unlistenFns.length = 0
  isRecording.value = false
}

/**
 * 组合式函数入口
 */
export function useAsr() {
  onMounted(async () => {
    await initAsrEngine()
    await setupAsrEventListeners()
  })

  onUnmounted(() => {
    cleanupAsr()
  })

  return {
    // 状态
    isRecording,
    asrResult,
    asrError,
    microphoneAvailable,
    modelReady,
    asrInitialized,

    // 方法
    initAsrEngine,
    checkModelReady,
    startRecording,
    stopRecording,
    toggleRecording,
  }
}
