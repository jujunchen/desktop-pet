import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ref } from 'vue'
import { addChatMemory } from './useMemory'

export interface ChatHistoryMessage {
  role: 'user' | 'assistant'
  content: string
}

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

export async function sendTextMessage(text: string, history: ChatHistoryMessage[] = []): Promise<void> {
  if (!text.trim() || isLoading.value) return

  console.log('[DEBUG] 发送消息:', text, '历史消息数:', history.length)
  cleanupListeners()
  isLoading.value = true
  errorMessage.value = ''
  currentResponse.value = ''

  unlistenStream = await listen<string>('voice://chat-stream', (event) => {
    currentResponse.value += event.payload
  })

  unlistenDone = await listen<{}>('voice://chat-done', async () => {
    console.log('[DEBUG] 聊天完成，最终回复:', currentResponse.value)

    // 保存这一轮对话到记忆
    if (text.trim() && currentResponse.value.trim()) {
      try {
        await addChatMemory(text, currentResponse.value)
        console.log('[DEBUG] 聊天记录保存成功')
      } catch (err) {
        console.error('[DEBUG] 保存聊天记录失败:', err)
      }
    }

    isLoading.value = false
    cleanupListeners()
  })

  try {
    const formattedHistory = history.map(h => ({
      role: h.role,
      content: h.content
    }))

    await invoke('chat_with_llm_stream', {
      prompt: text.trim(),
      history: formattedHistory
    })
  } catch (err: any) {
    const msg = err?.message || String(err)
    console.error('[DEBUG] 调用出错:', err)
    errorMessage.value = msg
    isLoading.value = false
    cleanupListeners()
  }
}
