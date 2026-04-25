import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ref } from 'vue'

export const isLoading = ref(false)
export const errorMessage = ref('')
export const currentResponse = ref('')

let unlistenStream: (() => void) | null = null
let unlistenDone: (() => void) | null = null

function cleanupListeners() {
  if (unlistenStream) {
    unlistenStream()
    unlistenStream = null
  }
  if (unlistenDone) {
    unlistenDone()
    unlistenDone = null
  }
}

export function cleanupVoiceAssistant() {
  cleanupListeners()
  isLoading.value = false
  errorMessage.value = ''
  currentResponse.value = ''
}

export async function sendTextMessage(text: string): Promise<void> {
  if (!text.trim() || isLoading.value) return

  console.log('[DEBUG] 开始发送消息:', text)
  cleanupListeners()
  isLoading.value = true
  errorMessage.value = ''
  currentResponse.value = ''

  unlistenStream = await listen<string>('voice://chat-stream', (event) => {
    console.log('[DEBUG] 收到流式内容:', event.payload)
    currentResponse.value += event.payload
  })

  unlistenDone = await listen<{}>('voice://chat-done', () => {
    console.log('[DEBUG] 聊天完成，最终回复:', currentResponse.value)
    isLoading.value = false
    cleanupListeners()
  })

  try {
    console.log('[DEBUG] 调用后端命令 chat_with_llm_stream, prompt:', text.trim())
    const result = await invoke('chat_with_llm_stream', { prompt: text.trim() })
    console.log('[DEBUG] 后端命令调用完成, result:', result)
  } catch (err: any) {
    const msg = err?.message || String(err)
    console.error('[DEBUG] 调用出错, 完整错误对象:', err)
    console.error('[DEBUG] 错误信息:', msg)
    errorMessage.value = msg
    isLoading.value = false
    cleanupListeners()
  }
}
