<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  type AppConfig,
  SCALE_MAX,
  SCALE_MIN,
  clampScale,
  hideWindow,
  loadConfig,
  saveConfig,
  showWindow
} from './composables/useWindowManager'
import {
  initAsrEngine,
  modelReady,
  checkModelReady
} from './composables/useAsr'

const config = ref<AppConfig | null>(null)
const loading = ref(true)
const saving = ref(false)
const defaultModelPath = ref('')
const saveMessage = ref('')
const saveMessageType = ref<'success' | 'error' | ''>('')
const showModal = ref(false)
const showLlmApiKey = ref(false)
let clearMessageTimer: number | null = null
let hideLlmApiKeyTimer: number | null = null

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
    defaultModelPath.value = await invoke('get_default_asr_model_path')
    await initAsrEngine()
    await checkModelReady()
  } finally {
    loading.value = false
  }
})

async function openModelDir(): Promise<void> {
  const path = config.value?.asr.sherpa_onnx.model_dir?.trim()
    ? config.value.asr.sherpa_onnx.model_dir
    : defaultModelPath.value
  await invoke('open_directory_in_explorer', { path })
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
        <h2>ASR 配置（SenseVoice 本地语音识别）</h2>
        <label class="field">
          <span>识别线程数</span>
          <select v-model.number="config.asr.sherpa_onnx.num_threads">
            <option :value="1">1 线程</option>
            <option :value="2">2 线程</option>
            <option :value="4">4 线程</option>
            <option :value="8">8 线程</option>
          </select>
        </label>
        <label class="field">
          <span>模型目录</span>
          <div class="path-row">
            <input
              v-model="config.asr.sherpa_onnx.model_dir"
              :placeholder="defaultModelPath"
              readonly
            />
            <button class="small-btn" type="button" @click="openModelDir">
              打开目录
            </button>
          </div>
        </label>
        <div class="model-status">
          <div v-if="modelReady" class="status-success">
            <span class="status-icon">✓</span>
            <span class="status-text">模型已就绪</span>
          </div>
          <div v-else class="status-warning">
            <span class="status-icon">⚠</span>
            <span class="status-text">请下载模型文件到上述目录：model.int8.onnx、tokens.txt</span>
          </div>
        </div>
        <div class="model-download-hint">
          <p><strong>下载地址：</strong></p>
          <code>https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-15.tar.bz2</code>
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

.path-row {
  display: flex;
  gap: 8px;
  flex: 1;
}

.path-row input {
  flex: 1;
  font-size: 12px;
  background: #f5f5f5;
}

.model-status {
  margin-top: 8px;
  padding: 10px;
  border-radius: 6px;
}

.status-success {
  background: #edf9f0;
  display: flex;
  align-items: center;
  gap: 8px;
  color: #2e9e58;
  font-size: 13px;
}

.status-warning {
  background: #fff8e6;
  display: flex;
  align-items: center;
  gap: 8px;
  color: #d97706;
  font-size: 13px;
}

.status-icon {
  font-size: 16px;
}

.model-download-hint {
  margin-top: 12px;
  padding: 10px;
  background: #f0f7ff;
  border-radius: 6px;
  font-size: 12px;
  color: #374151;
}

.model-download-hint p {
  margin: 0 0 6px;
  font-weight: 500;
}

.model-download-hint code {
  display: block;
  padding: 6px 8px;
  background: #fff;
  border-radius: 4px;
  font-family: monospace;
  word-break: break-all;
  color: #1f2937;
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

.model-status {
  margin-top: 4px;
  padding: 10px;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.status-success {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #edf9f0;
  border-radius: 6px;
  color: #2e9e58;
}

.status-warning {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #fff8e6;
  border-radius: 6px;
  color: #b36b00;
}

.status-icon {
  font-size: 14px;
  font-weight: bold;
}

.status-text {
  flex: 1;
  font-size: 12px;
}

.download-btn {
  background: #264653;
  color: #fff;
  border-color: #264653;
  font-size: 11px;
  padding: 4px 8px;
}

.download-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
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
