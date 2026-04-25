<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { sendTextMessage, isLoading, errorMessage, currentResponse, cleanupVoiceAssistant } from './composables/useVoiceAssistant'
import { enterProcessingState, enterSpeakingState } from './composables/usePetState'

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
    currentStreamingId = null
    pendingText = ''
  }
})

onMounted(() => {
  addMessage('pet', '你好呀！我是小白，有什么想聊的吗？')
  nextTick(() => {
    textareaRef.value?.focus()
  })
})

onUnmounted(() => {
  currentStreamingId = null
  pendingText = ''
  cleanupVoiceAssistant()
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
  console.log('[ChatApp] 点击发送, text:', text, 'isLoading:', isLoading.value)
  if (!text || isLoading.value) return

  addMessage('user', text)
  inputText.value = ''

  enterProcessingState()
  addMessage('pet', '')
  currentStreamingId = messageIdCounter
  pendingText = ''

  console.log('[ChatApp] 调用 sendTextMessage...')
  await sendTextMessage(text)
  console.log('[ChatApp] sendTextMessage 完成')
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
    <div class="chat-header">
      <h3>🐕 和小白聊天</h3>
    </div>

    <div ref="messagesContainerRef" class="messages-container">
      <div
        v-for="msg in messages"
        :key="msg.id"
        :class="['message', msg.role]"
      >
        <div class="message-avatar">{{ msg.role === 'user' ? '你' : '白' }}</div>
        <div class="message-content">
          <div class="message-text">{{ msg.text }}</div>
          <div class="message-time">{{ formatTime(msg.timestamp) }}</div>
        </div>
      </div>

      <div v-if="isLoading" class="message pet typing">
        <div class="message-avatar">白</div>
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
      <div v-if="errorMessage" class="error-message">{{ errorMessage }}</div>
      <textarea
        ref="textareaRef"
        v-model="inputText"
        class="chat-input"
        placeholder="输入消息，按 Enter 发送..."
        :disabled="isLoading"
        @keydown="handleKeydown"
        rows="2"
      ></textarea>
      <button
        class="send-btn"
        :disabled="isLoading || !inputText.trim()"
        @click="sendMessage"
      >
        {{ isLoading ? '...' : '发送' }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.chat-app {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f7f8fa;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

.chat-header {
  padding: 16px 20px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  box-shadow: 0 2px 8px rgba(102, 126, 234, 0.2);
}

.chat-header h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.messages-container {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  scroll-behavior: smooth;
}

.message {
  display: flex;
  gap: 10px;
  margin-bottom: 16px;
  max-width: 85%;
}

.message.user {
  margin-left: auto;
  flex-direction: row-reverse;
}

.message-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  font-weight: 600;
  flex-shrink: 0;
}

.message.user .message-avatar {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.message.pet .message-avatar {
  background: #e8eaed;
  color: #5f6368;
}

.message-content {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.message-text {
  padding: 10px 14px;
  border-radius: 18px;
  font-size: 14px;
  line-height: 1.5;
  word-break: break-word;
}

.message.user .message-text {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  border-bottom-right-radius: 4px;
}

.message.pet .message-text {
  background: white;
  color: #333;
  border-bottom-left-radius: 4px;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
}

.message-time {
  font-size: 11px;
  color: #999;
  padding: 0 6px;
}

.message.user .message-time {
  text-align: right;
}

.typing .typing-dots {
  display: flex;
  gap: 4px;
  padding: 8px 0;
}

.typing .typing-dots span {
  width: 8px;
  height: 8px;
  background: #bbb;
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
  padding: 12px 16px;
  background: white;
  border-top: 1px solid #eee;
}

.error-message {
  margin-bottom: 10px;
  padding: 8px 12px;
  background: #fff2f2;
  border-radius: 6px;
  color: #e53e3e;
  font-size: 13px;
}

.chat-input {
  width: 100%;
  padding: 10px 14px;
  border: 1px solid #e0e0e0;
  border-radius: 20px;
  resize: none;
  font-size: 14px;
  font-family: inherit;
  box-sizing: border-box;
  transition: border-color 0.2s;
}

.chat-input:focus {
  outline: none;
  border-color: #667eea;
}

.chat-input:disabled {
  background: #f9f9f9;
  cursor: not-allowed;
}

.send-btn {
  margin-top: 10px;
  width: 100%;
  padding: 10px 16px;
  border: none;
  border-radius: 20px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.send-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.send-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.messages-container::-webkit-scrollbar {
  width: 6px;
}

.messages-container::-webkit-scrollbar-track {
  background: transparent;
}

.messages-container::-webkit-scrollbar-thumb {
  background: #ddd;
  border-radius: 3px;
}

.messages-container::-webkit-scrollbar-thumb:hover {
  background: #ccc;
}
</style>
