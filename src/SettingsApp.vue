<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { onMounted, onUnmounted, ref, watch } from 'vue'
import {
  SCALE_CHANGED_EVENT,
  SCALE_MAX,
  SCALE_MIN,
  clampScale,
  hideWindow,
  loadScale,
  setMainWindowScale,
  showWindow
} from './composables/useWindowManager'

const scale = ref(1)
let saveTimer: number | null = null
let unlisten: null | (() => void) = null

onMounted(async () => {
  scale.value = await loadScale()
  unlisten = await listen<number>(SCALE_CHANGED_EVENT, (event) => {
    const value = Number(event.payload)
    if (Number.isFinite(value)) {
      scale.value = clampScale(value)
    }
  })
})

onUnmounted(() => {
  if (saveTimer) {
    window.clearTimeout(saveTimer)
  }
  if (unlisten) {
    unlisten()
  }
})

watch(scale, (next) => {
  const normalized = clampScale(next)
  if (normalized !== next) {
    scale.value = normalized
    return
  }

  if (saveTimer) {
    window.clearTimeout(saveTimer)
  }

  saveTimer = window.setTimeout(() => {
    void setMainWindowScale(normalized)
  }, 80)
})
</script>

<template>
  <main class="settings-page">
    <h1>窗口设置</h1>

    <label class="field">
      <span>宠物大小：{{ Math.round(scale * 100) }}%</span>
      <input v-model.number="scale" type="range" :min="SCALE_MIN" :max="SCALE_MAX" step="0.1" />
    </label>

    <div class="actions">
      <button @click="showWindow">显示宠物</button>
      <button @click="hideWindow">隐藏宠物</button>
    </div>
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
  background: #f7fafc;
  color: #0f172a;
  font-family: "Microsoft YaHei UI", "Segoe UI", sans-serif;
}

h1 {
  margin: 0;
  font-size: 15px;
}

.field {
  display: grid;
  gap: 8px;
  font-size: 13px;
}

input[type='range'] {
  width: 100%;
}

.actions {
  display: flex;
  gap: 8px;
}

button {
  border: 1px solid rgba(26, 32, 44, 0.25);
  background: #fff;
  color: #0f172a;
  border-radius: 8px;
  font-size: 12px;
  padding: 6px 10px;
}
</style>
