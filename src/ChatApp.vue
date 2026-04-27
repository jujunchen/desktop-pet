<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch, computed } from 'vue'
import { sendTextMessage, isLoading, errorMessage, currentResponse, cleanupVoiceAssistant } from './composables/useVoiceAssistant'
import { enterProcessingState, enterSpeakingState } from './composables/usePetState'
import {
  useAsr,
  isRecording,
  asrResult,
  asrError,
  startVoiceChat,
  voiceButtonText,
  canStartVoiceChat
} from './composables/useAsr'
import { loadConfig, CONFIG_CHANGED_EVENT, type AppConfig } from './composables/useWindowManager'
import { listen } from '@tauri-apps/api/event'
import { onChatCompleted, growthState, loadGrowthState, petMode } from './composables/usePetGrowth'

interface ChatMessage {
  id: number
  role: 'user' | 'pet'
  text: string
  timestamp: number
}

const messages = ref<ChatMessage[]>([])
const inputText = ref('')
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const messagesContainerRef = ref<HTMLDivElement | null>(null)
let messageIdCounter = 0
let currentStreamingId: number | null = null
let pendingText = ''

// 宠物配置
const petName = ref('小白')
const petAvatar = computed(() => petName.value.charAt(0) || '白')
let unlistenConfig: (() => void) | null = null

// 判断是否可以执行指令（幼体阶段不能执行）
const canExecuteCommands = computed(() => {
  if (petMode.value === 'Assistant') return true
  if (!growthState.value) return true
  return growthState.value.stage !== 'Baby' && growthState.value.stage !== 'Dead'
})

// 检测是否包含指令类请求关键词
function containsCommandRequest(text: string): boolean {
  const commandKeywords = [
    '打开', '关闭', '启动', '运行', '执行', '点击', '输入', '搜索',
    '帮我', '给我', '控制', '操作', '设置', '调整', '清空', '删除',
    'command', 'execute', 'run', 'open', 'close', 'delete', 'control'
  ]
  return commandKeywords.some(keyword => text.includes(keyword))
}

// 初始化ASR
useAsr()

// 检测到ASR识别结果，自动添加到输入框
watch(asrResult, (result) => {
  if (result.trim()) {
    inputText.value = result.trim()
    // 自动发送
    setTimeout(() => sendMessage(), 300)
  }
})

// 显示ASR错误
watch(asrError, (error) => {
  if (error) {
    console.error('ASR错误:', error)
  }
})

watch(currentResponse, (newText) => {
  if (currentStreamingId !== null && newText) {
    updateMessage(currentStreamingId, newText)
    pendingText = newText
  }
})

watch(isLoading, (loading) => {
  if (!loading && currentStreamingId !== null) {
    if (pendingText.trim() === '') {
      updateMessage(currentStreamingId, '（没有收到回复，请检查API配置）')
    }
    enterSpeakingState(pendingText)
    // 聊天完成，增加亲密度
    void onChatCompleted()
    currentStreamingId = null
    pendingText = ''
    // 回复完成后自动聚焦输入框
    nextTick(() => {
      textareaRef.value?.focus()
    })
  }
})

onMounted(async () => {
  // 加载当前配置
  const config = await loadConfig()
  petName.value = config.pet.name || '小白'

  // 加载成长状态
  await loadGrowthState()

  // 监听配置变更事件，实时更新
  unlistenConfig = await listen<AppConfig>(CONFIG_CHANGED_EVENT, (event) => {
    const newName = event.payload.pet.name || '小白'
    if (newName !== petName.value) {
      petName.value = newName
    }
  })

  // 根据生命阶段显示不同的欢迎语
  if (!canExecuteCommands.value) {
    addMessage('pet', `你好呀！我是${petName.value}，我还是个小宝宝呢~ 你可以陪我聊天、玩耍，等我长大成年后就能帮你做很多事情啦！🐾`)
  } else {
    addMessage('pet', `你好呀！我是${petName.value}，有什么想聊的吗？`)
  }
  nextTick(() => {
    textareaRef.value?.focus()
  })
})

onUnmounted(() => {
  currentStreamingId = null
  pendingText = ''
  cleanupVoiceAssistant()
  if (unlistenConfig) unlistenConfig()
})

function addMessage(role: 'user' | 'pet', text: string) {
  messages.value.push({
    id: ++messageIdCounter,
    role,
    text,
    timestamp: Date.now()
  })
  scrollToBottom()
}

function updateMessage(id: number, text: string) {
  const msg = messages.value.find(m => m.id === id)
  if (msg) {
    msg.text = text
    scrollToBottom()
  }
}

function scrollToBottom() {
  nextTick(() => {
    if (messagesContainerRef.value) {
      messagesContainerRef.value.scrollTop = messagesContainerRef.value.scrollHeight
    }
  })
}

async function sendMessage() {
  const text = inputText.value.trim()
  if (!text) return

  // 如果正在加载，给予提示但不阻止输入
  if (isLoading.value) {
    console.log(`${petName.value}正在思考中，请稍等～`)
    return
  }

  // 幼体阶段不能执行指令类请求
  if (!canExecuteCommands.value && containsCommandRequest(text)) {
    addMessage('user', text)
    inputText.value = ''

    const babyHint = `我还太小了，还不会帮你做这些事情呢~ 等我长大成年后就能帮你执行指令啦！你可以多陪我聊聊天、喂喂我，我会快快长大的 🐾`
    addMessage('pet', babyHint)
    return
  }

  addMessage('user', text)
  inputText.value = ''

  enterProcessingState()
  addMessage('pet', '')
  currentStreamingId = messageIdCounter
  pendingText = ''

  // 清空后自动聚焦
  nextTick(() => {
    textareaRef.value?.focus()
  })

  // 构造对话历史（不包括刚添加的空消息）
  const history = messages.value
    .slice(0, -1)
    .map(msg => ({
      role: msg.role === 'user' ? 'user' as const : 'assistant' as const,
      content: msg.text
    }))

  await sendTextMessage(text, history)
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    sendMessage()
  }
}

function formatTime(timestamp: number): string {
  const date = new Date(timestamp)
  return `${date.getHours().toString().padStart(2, '0')}:${date.getMinutes().toString().padStart(2, '0')}`
}
</script>

<template>
  <div class="chat-app">
    <div ref="messagesContainerRef" class="messages-container">
      <div
        v-for="msg in messages"
        :key="msg.id"
        :class="['message', msg.role]"
        v-show="msg.id !== currentStreamingId || msg.text"
      >
        <div class="message-avatar">{{ msg.role === 'user' ? '你' : petAvatar }}</div>
        <div class="message-content">
          <div class="message-text">{{ msg.text }}</div>
          <div class="message-time">{{ formatTime(msg.timestamp) }}</div>
        </div>
      </div>

      <div v-if="isLoading" class="message pet typing">
        <div class="message-avatar">{{ petAvatar }}</div>
        <div class="message-content">
          <div class="typing-dots">
            <span></span>
            <span></span>
            <span></span>
          </div>
        </div>
      </div>
    </div>

    <div class="input-area">
      <div v-if="errorMessage || asrError" class="error-message">{{ errorMessage || asrError }}</div>

      <div class="input-row">
        <textarea
          ref="textareaRef"
          v-model="inputText"
          class="chat-input"
          placeholder="输入消息，按 Enter 发送..."
          @keydown="handleKeydown"
          rows="2"
        ></textarea>

        <button
          class="voice-btn"
          :class="{ recording: isRecording, disabled: !canStartVoiceChat }"
          :disabled="!canStartVoiceChat"
          @click="startVoiceChat"
          title="点击开始语音聊天，说完后自动识别"
        >
          <span>{{ voiceButtonText }}</span>
        </button>
      </div>

      <button
        class="send-btn"
        :disabled="isLoading || (!inputText.trim() && !isRecording)"
        @click="sendMessage"
      >
        {{ isLoading ? petName + '思考中...' : '发送' }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.chat-app {
  --accent: #4a6078;
  --accent-strong: #3c5066;
  --accent-soft: #e9edf2;
  --surface: #ffffff;
  --surface-soft: #ffffff;
  --text-main: #1f2733;
  --text-sub: #657080;
  --border-soft: #d9e0e8;
  --shadow-soft: 0 8px 20px rgba(21, 35, 52, 0.08);

  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  position: relative;
  background: linear-gradient(180deg, #f5f7fa 0%, #f2f4f7 100%);
  font-family: 'PingFang SC', 'Microsoft YaHei UI', 'Segoe UI', sans-serif;
}

.messages-container {
  flex: 1;
  overflow-y: auto;
  margin: 12px 18px 0;
  padding: 18px 20px 16px;
  background: rgba(255, 255, 255, 0.82);
  border: 1px solid #e3e8ee;
  border-radius: 22px 22px 0 0;
  box-shadow: var(--shadow-soft);
  scroll-behavior: smooth;
  position: relative;
  z-index: 1;
}

.message {
  display: flex;
  gap: 18px;
  margin-bottom: 20px;
  max-width: min(78%, 560px);
}

.message.user {
  margin-left: auto;
  flex-direction: row-reverse;
}

.message-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 15px;
  font-weight: 700;
  flex-shrink: 0;
  box-shadow: 0 6px 18px rgba(76, 99, 136, 0.15);
}

.message.user .message-avatar {
  background: #dfe5ec;
  color: #435265;
}

.message.pet .message-avatar {
  background: #eef2f6;
  color: #55667b;
  border: 1px solid #dde4eb;
}

.message-content {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.message-text {
  padding: 12px 15px;
  border-radius: 16px;
  font-size: 15px;
  line-height: 1.58;
  letter-spacing: 0.15px;
  word-break: break-word;
}

.message.user .message-text {
  background: #52667e;
  color: #f4f7fb;
  border-bottom-right-radius: 6px;
  box-shadow: 0 6px 12px rgba(38, 54, 74, 0.16);
}

.message.pet .message-text {
  background: var(--surface-soft);
  color: var(--text-main);
  border: 1px solid var(--border-soft);
  border-bottom-left-radius: 6px;
  box-shadow: 0 4px 10px rgba(48, 62, 79, 0.08);
  backdrop-filter: none;
}

.message-time {
  font-size: 12px;
  color: #7b879b;
  padding: 0 6px;
}

.message.user .message-time {
  text-align: right;
}

.typing .typing-dots {
  display: flex;
  gap: 6px;
  padding: 11px 4px;
}

.typing .typing-dots span {
  width: 7px;
  height: 7px;
  background: #9aa8ba;
  border-radius: 50%;
  animation: typing 1.4s infinite;
}

.typing .typing-dots span:nth-child(2) {
  animation-delay: 0.2s;
}

.typing .typing-dots span:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes typing {
  0%, 60%, 100% {
    transform: translateY(0);
    opacity: 0.4;
  }
  30% {
    transform: translateY(-4px);
    opacity: 1;
  }
}

.input-area {
  padding: 12px 18px 14px;
  background: #f8fafc;
  border-top: 1px solid #dde3ea;
  backdrop-filter: none;
  position: relative;
  z-index: 2;
}

.error-message {
  margin-bottom: 10px;
  padding: 8px 12px;
  background: #fff5f5;
  border-radius: 10px;
  border: 1px solid #ffd7d7;
  color: #d63636;
  font-size: 13px;
}

.input-row {
  display: flex;
  gap: 12px;
  align-items: flex-end;
}

.chat-input {
  flex: 1;
  height: 56px;
  min-height: 56px;
  max-height: 56px;
  padding: 16px 15px;
  border: 1px solid #c7d0da;
  border-radius: 18px;
  resize: none;
  font-size: 15px;
  color: #2d3847;
  background: #ffffff;
  font-family: inherit;
  box-sizing: border-box;
  line-height: 1.4;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.chat-input::placeholder {
  color: #8b97a7;
}

.voice-btn {
  padding: 0 16px;
  border: 1px solid #c7d0da;
  border-radius: 17px;
  background: #ffffff;
  color: var(--accent);
  height: 56px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
  white-space: nowrap;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.voice-btn:hover:not(:disabled) {
  background: #f0f3f7;
  color: var(--accent-strong);
  border-color: #aebac8;
  box-shadow: none;
}

.voice-btn.recording {
  background: #8a4a4a;
  color: white;
  border-color: #8a4a4a;
  animation: pulse 1.5s infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}

.voice-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.chat-input:focus {
  outline: none;
  border-color: #97a5b6;
  box-shadow: 0 0 0 3px rgba(125, 141, 160, 0.14);
}

.chat-input:disabled {
  background: #f4f6fb;
  cursor: not-allowed;
}

.send-btn {
  margin-top: 10px;
  width: 100%;
  padding: 12px 16px;
  border: none;
  border-radius: 16px;
  background: #4a6078;
  color: #ffffff;
  font-size: 17px;
  letter-spacing: 0.6px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.24s ease;
  box-shadow: 0 8px 16px rgba(38, 55, 78, 0.2);
}

.send-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 10px 18px rgba(38, 55, 78, 0.24);
}

.send-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
  box-shadow: none;
}

.messages-container::-webkit-scrollbar {
  width: 8px;
}

.messages-container::-webkit-scrollbar-track {
  background: transparent;
}

.messages-container::-webkit-scrollbar-thumb {
  background: rgba(131, 146, 165, 0.45);
  border-radius: 10px;
}

.messages-container::-webkit-scrollbar-thumb:hover {
  background: rgba(116, 133, 154, 0.58);
}

@media (max-width: 768px) {
  .messages-container {
    margin: 10px 10px 10px;
    padding: 14px 12px 10px;
    border-radius: 16px 16px 16px 16px;
  }

  .message {
    max-width: 90%;
  }

  .message-avatar {
    width: 34px;
    height: 34px;
    font-size: 13px;
  }

  .message-text {
    font-size: 14px;
    padding: 10px 12px;
  }

  .input-row {
    gap: 8px;
  }

  .voice-btn {
    height: 50px;
    padding: 0 12px;
    font-size: 13px;
  }

  .send-btn {
    font-size: 16px;
  }
}
</style>
