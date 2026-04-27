<script setup lang="ts">
import { onMounted, ref } from 'vue'
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
  asrReady,
  checkAsrReady,
  microphoneAvailable
} from './composables/useAsr'

const config = ref<AppConfig | null>(null)
const loading = ref(true)
const saving = ref(false)
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
    await initAsrEngine()
    await checkAsrReady()
  } finally {
    loading.value = false
  }
})

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
          <input v-model="config.llm.model" placeholder="glm-4.7-flash" />
        </label>
        <label class="field">
          <span>Base URL</span>
          <input v-model="config.llm.base_url" placeholder="https://open.bigmodel.cn/api/paas/v4" />
        </label>
      </section>

      <section class="card">
        <h2>ASR 配置（系统语音识别）</h2>
        <div class="model-status">
          <div v-if="microphoneAvailable && asrReady" class="status-success">
            <span class="status-icon">✓</span>
            <span class="status-text">系统ASR可用</span>
          </div>
          <div v-else class="status-warning">
            <span class="status-icon">⚠</span>
            <span class="status-text">请检查麦克风与系统语音识别权限（macOS需开启“语音识别”授权）</span>
          </div>
        </div>
      </section>

      <section class="card">
        <h2>宠物配置</h2>
        <label class="field">
          <span>宠物名字</span>
          <input v-model="config.pet.name" placeholder="小白" maxlength="10" />
        </label>
        <label class="field">
          <span>性格设定（提示词）</span>
          <textarea
            v-model="config.pet.prompt"
            placeholder="你是一只可爱的桌面宠物，名字叫{name}..."
            rows="4"
            class="prompt-textarea"
          ></textarea>
          <small class="field-hint">使用 {name} 作为宠物名字的占位符，会自动替换</small>
        </label>
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
  background: #e6f9ef;
  color: #18a058;
}

.modal-error .modal-icon {
  background: #ffecec;
  color: #d03050;
}

.modal-message {
  margin: 0 0 16px;
  color: #22303c;
  line-height: 1.4;
  white-space: pre-wrap;
}

.modal-btn {
  border: 0;
  border-radius: 8px;
  background: #2f7bd8;
  color: #fff;
  padding: 8px 18px;
  cursor: pointer;
}

.card {
  background: #fff;
  border: 1px solid #dbe4ea;
  border-radius: 10px;
  padding: 12px;
  display: grid;
  gap: 10px;
}

.card h2 {
  margin: 0;
  font-size: 14px;
  color: #253544;
}

.field {
  display: grid;
  gap: 6px;
}

.field > span {
  font-size: 12px;
  color: #4a5c6d;
}

input,
select,
textarea {
  border: 1px solid #c9d5df;
  border-radius: 8px;
  padding: 8px 10px;
  font-size: 13px;
  color: #1d2a36;
  background: #fff;
  font-family: inherit;
}

input,
select {
  height: 32px;
}

.prompt-textarea {
  resize: vertical;
  line-height: 1.5;
}

.field-hint {
  font-size: 11px;
  color: #7d8c9b;
  margin-top: -4px;
  line-height: 1.4;
}

.secret-row {
  display: flex;
  gap: 8px;
}

.secret-row input {
  flex: 1;
}

.small-btn {
  border: 1px solid #b8c7d4;
  background: #fff;
  color: #304455;
  border-radius: 8px;
  padding: 0 10px;
  cursor: pointer;
}

.model-status {
  margin-top: 2px;
}

.status-success,
.status-warning {
  display: flex;
  gap: 8px;
  align-items: center;
  font-size: 12px;
  line-height: 1.4;
}

.status-success {
  color: #176f44;
}

.status-warning {
  color: #8f5d00;
}

.actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-bottom: 6px;
}

.btn {
  height: 34px;
  border-radius: 8px;
  border: 1px solid #b8c7d4;
  background: #fff;
  color: #2c3f50;
  padding: 0 12px;
  cursor: pointer;
}

.btn.primary {
  background: #2f7bd8;
  border-color: #2f7bd8;
  color: #fff;
}

.btn:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.placeholder {
  color: #5b6e7f;
  font-size: 13px;
}

@media (max-width: 560px) {
  .actions {
    justify-content: stretch;
  }

  .actions .btn {
    flex: 1;
  }
}
</style>
