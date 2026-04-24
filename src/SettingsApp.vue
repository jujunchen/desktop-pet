<script setup lang="ts">
import { onMounted, ref } from 'vue'
import {
  type AppConfig,
  type AsrProvider,
  SCALE_MAX,
  SCALE_MIN,
  clampScale,
  hideWindow,
  loadConfig,
  saveConfig,
  showWindow
} from './composables/useWindowManager'

const config = ref<AppConfig | null>(null)
const loading = ref(true)
const saving = ref(false)
const saveMessage = ref('')
const saveMessageType = ref<'success' | 'error' | ''>('')
const showModal = ref(false)
const showLlmApiKey = ref(false)
const showAsrApiKey = ref(false)
let clearMessageTimer: number | null = null
let hideLlmApiKeyTimer: number | null = null
let hideAsrApiKeyTimer: number | null = null

const PROVIDERS: AsrProvider[] = ['whisper-local', 'dashscope', 'volcengine', 'funasr']

const MODEL_PLACEHOLDER: Record<AsrProvider, string> = {
  'whisper-local': '',
  dashscope: 'paraformer-v2',
  volcengine: 'speech-paraformer',
  funasr: 'paraformer'
}

const BASE_URL_PLACEHOLDER: Record<AsrProvider, string> = {
  'whisper-local': '',
  dashscope: 'https://dashscope.aliyuncs.com/compatible-mode/v1/audio/transcriptions',
  volcengine: 'https://ark.cn-beijing.volces.com/api/v3/audio/transcriptions',
  funasr: 'http://127.0.0.1:10095/transcriptions'
}

function withMessage(msg: string, type: 'success' | 'error' = 'success'): void {
  saveMessage.value = msg
  saveMessageType.value = type
  showModal.value = true
  if (clearMessageTimer) {
    window.clearTimeout(clearMessageTimer)
  }
  clearMessageTimer = window.setTimeout(() => {
    closeModal()
  }, 2600)
}

function closeModal(): void {
  showModal.value = false
  saveMessage.value = ''
  saveMessageType.value = ''
}

onMounted(async () => {
  try {
    config.value = await loadConfig()
  } finally {
    loading.value = false
  }
})

function modelPlaceholder(provider: AsrProvider): string {
  return MODEL_PLACEHOLDER[provider]
}

function baseUrlPlaceholder(provider: AsrProvider): string {
  return BASE_URL_PLACEHOLDER[provider]
}

async function saveAll(): Promise<void> {
  if (!config.value || saving.value) {
    return
  }
  saving.value = true
  try {
    config.value.pet.scale = clampScale(config.value.pet.scale)
    config.value = await saveConfig(config.value)
    withMessage('保存成功，重启应用后生效', 'success')
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    withMessage(`保存失败: ${message}`, 'error')
  } finally {
    saving.value = false
  }
}

function revealLlmApiKeyTemporarily(): void {
  showLlmApiKey.value = true
  if (hideLlmApiKeyTimer) {
    window.clearTimeout(hideLlmApiKeyTimer)
  }
  hideLlmApiKeyTimer = window.setTimeout(() => {
    showLlmApiKey.value = false
  }, 3000)
}

function revealAsrApiKeyTemporarily(): void {
  showAsrApiKey.value = true
  if (hideAsrApiKeyTimer) {
    window.clearTimeout(hideAsrApiKeyTimer)
  }
  hideAsrApiKeyTimer = window.setTimeout(() => {
    showAsrApiKey.value = false
  }, 3000)
}

</script>

<template>
  <main class="settings-page">
    <header class="header">
      <h1>配置管理</h1>
      <span class="msg">{{ saving ? '保存中...' : '' }}</span>
    </header>

    <div v-if="showModal" class="modal-overlay" @click.self="closeModal">
      <div class="modal" :class="saveMessageType === 'error' ? 'modal-error' : 'modal-success'">
        <div class="modal-icon">
          <span v-if="saveMessageType === 'success'">✓</span>
          <span v-else>✕</span>
        </div>
        <p class="modal-message">{{ saveMessage }}</p>
        <button class="modal-btn" @click="closeModal">确定</button>
      </div>
    </div>

    <section v-if="loading" class="placeholder">加载中...</section>

    <template v-else-if="config">
      <section class="card">
        <h2>LLM 配置</h2>
        <label class="field">
          <span>API Key</span>
          <div class="secret-row">
            <input v-model="config.llm.api_key" :type="showLlmApiKey ? 'text' : 'password'" placeholder="sk-..." />
            <button class="small-btn secret-btn" type="button" title="临时显示 3 秒" @click="revealLlmApiKeyTemporarily">临时查看</button>
          </div>
        </label>
        <label class="field">
          <span>模型</span>
          <input v-model="config.llm.model" placeholder="gpt-4o-mini" />
        </label>
        <label class="field">
          <span>Base URL</span>
          <input v-model="config.llm.base_url" placeholder="https://api.openai.com/v1" />
        </label>
      </section>

      <section class="card">
        <h2>ASR 配置</h2>
        <label class="field">
          <span>提供商</span>
          <select v-model="config.asr.provider">
            <option value="whisper-local">本地 Whisper</option>
            <option value="dashscope">阿里百炼</option>
            <option value="volcengine">火山方舟</option>
            <option value="funasr">FunASR 私有化</option>
          </select>
        </label>

        <div v-if="config.asr.provider === 'whisper-local'" class="provider-box">
          <label class="field">
            <span>模型大小</span>
            <select v-model="config.asr.whisper_local.model_size">
              <option value="tiny">tiny</option>
              <option value="base">base</option>
              <option value="small">small</option>
              <option value="medium">medium</option>
            </select>
          </label>
        </div>

        <div v-if="PROVIDERS.includes(config.asr.provider) && config.asr.provider !== 'whisper-local'" class="provider-box">
          <label class="field">
            <span>API Key</span>
            <div class="secret-row">
              <input v-model="config.asr[config.asr.provider].api_key" :type="showAsrApiKey ? 'text' : 'password'" placeholder="sk-..." />
              <button class="small-btn secret-btn" type="button" title="临时显示 3 秒" @click="revealAsrApiKeyTemporarily">临时查看</button>
            </div>
          </label>
          <label class="field">
            <span>模型</span>
            <input v-model="config.asr[config.asr.provider].model" :placeholder="modelPlaceholder(config.asr.provider)" />
          </label>
          <label class="field">
            <span>Base URL</span>
            <input v-model="config.asr[config.asr.provider].base_url" :placeholder="baseUrlPlaceholder(config.asr.provider)" />
          </label>
        </div>
      </section>

      <section class="card">
        <h2>宠物配置</h2>
        <label class="field">
          <span>宠物种类</span>
          <input v-model="config.pet.current" placeholder="dog" />
        </label>
        <label class="field">
          <span>显示大小：{{ Math.round(config.pet.scale * 100) }}%</span>
          <input
            v-model.number="config.pet.scale"
            type="range"
            :min="SCALE_MIN"
            :max="SCALE_MAX"
            step="0.1"
          />
        </label>
      </section>

      <footer class="actions">
        <button class="btn" @click="showWindow">显示宠物</button>
        <button class="btn" @click="hideWindow">隐藏宠物</button>
        <button class="btn primary" :disabled="saving" @click="saveAll">
          {{ saving ? '保存中...' : '保存设置' }}
        </button>
      </footer>
    </template>
  </main>
</template>

<style scoped>
.settings-page {
  height: 100vh;
  box-sizing: border-box;
  padding: 14px;
  display: grid;
  align-content: start;
  gap: 12px;
  overflow-y: auto;
  overflow-x: hidden;
  background: #f4f6f8;
  color: #17202a;
  font-family: "Microsoft YaHei UI", "Segoe UI", sans-serif;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

h1 {
  margin: 0;
  font-size: 16px;
}

.msg {
  font-size: 12px;
  color: #3c6e71;
}

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: #fff;
  border-radius: 12px;
  padding: 24px;
  min-width: 280px;
  max-width: 340px;
  text-align: center;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
  animation: modalIn 0.2s ease;
}

@keyframes modalIn {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

.modal-icon {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 12px;
  font-size: 24px;
}

.modal-success .modal-icon {
  background: #edf9f0;
  color: #2e9e58;
}

.modal-error .modal-icon {
  background: #fdeeee;
  color: #d14038;
}

.modal-message {
  margin: 0 0 16px;
  font-size: 14px;
  color: #17202a;
}

.modal-btn {
  width: 100%;
  border: none;
  border-radius: 8px;
  padding: 10px 16px;
  font-size: 14px;
  cursor: pointer;
  transition: background-color 0.15s ease;
}

.modal-success .modal-btn {
  background: #264653;
  color: #fff;
}

.modal-success .modal-btn:hover {
  background: #1d3640;
}

.modal-error .modal-btn {
  background: #d14038;
  color: #fff;
}

.modal-error .modal-btn:hover {
  background: #b3352e;
}

.card {
  border: 1px solid #d9e2ec;
  border-radius: 10px;
  background: #ffffff;
  padding: 12px;
  display: grid;
  gap: 10px;
}

h2 {
  margin: 0;
  font-size: 14px;
}

.field {
  display: grid;
  gap: 6px;
  font-size: 12px;
}

.secret-row {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 8px;
  align-items: center;
}

input,
select,
button {
  font: inherit;
}

input,
select {
  width: 100%;
  box-sizing: border-box;
  border: 1px solid #cdd5df;
  border-radius: 8px;
  padding: 7px 9px;
  background: #fff;
}

.provider-box {
  margin-top: 2px;
  border: 1px solid #e6edf3;
  border-radius: 8px;
  padding: 10px;
  background: #fafcfe;
  display: grid;
  gap: 8px;
}

.small-btn {
  border: 1px solid #c3d0dc;
  background: #eef4fa;
  border-radius: 8px;
  padding: 6px 8px;
}

.secret-btn {
  min-width: 74px;
  white-space: nowrap;
  border-color: #d6dde6;
  background: #f7f9fc;
  color: #4e5f73;
  font-size: 12px;
  padding: 6px 10px;
  transition: background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease;
}

.secret-btn:hover {
  background: #f0f4f8;
  border-color: #c8d2de;
  color: #3f5166;
}

.secret-btn:active {
  background: #e8edf4;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.btn {
  border: 1px solid rgba(23, 32, 42, 0.25);
  background: #fff;
  color: #17202a;
  border-radius: 8px;
  font-size: 12px;
  padding: 7px 10px;
}

.btn.primary {
  border-color: #264653;
  background: #264653;
  color: #fff;
}

.placeholder {
  font-size: 13px;
  color: #52606d;
}
</style>
