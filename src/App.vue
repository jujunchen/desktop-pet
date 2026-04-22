<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import {
  clampScale,
  hideWindow,
  loadScale,
  saveScale,
  showWindow
} from './composables/useWindowManager'

const BASE_SIZE = 180

const scale = ref(1)
const isSettingsOpen = ref(false)
const styleSize = computed(() => `${BASE_SIZE * scale.value}px`)

let saveTimer: number | null = null
let unlisten: null | (() => void) = null

onMounted(async () => {
  scale.value = await loadScale()
  await applyWindowSize(scale.value)
  unlisten = await listen('m1://open-settings', () => {
    isSettingsOpen.value = true
    void showWindow()
  })
})

onUnmounted(() => {
  if (unlisten) {
    unlisten()
  }
})

watch(scale, async (next) => {
  const normalized = clampScale(next)
  if (normalized !== next) {
    scale.value = normalized
    return
  }

  await applyWindowSize(normalized)
  if (saveTimer) window.clearTimeout(saveTimer)
  saveTimer = window.setTimeout(() => {
    void saveScale(normalized)
  }, 150)
})

async function applyWindowSize(nextScale: number): Promise<void> {
  const size = BASE_SIZE * nextScale
  const appWindow = getCurrentWindow()
  await appWindow.setSize(new LogicalSize(size, size))
}

function onWheel(event: WheelEvent): void {
  const step = event.deltaY > 0 ? -0.1 : 0.1
  scale.value = clampScale(Number((scale.value + step).toFixed(2)))
}

async function startDrag(): Promise<void> {
  await getCurrentWindow().startDragging()
}

function openSettingsPanel(): void {
  isSettingsOpen.value = true
}

function closeSettingsPanel(): void {
  isSettingsOpen.value = false
}
</script>

<template>
  <main class="app" @mousedown.left="startDrag" @wheel.prevent="onWheel">
    <img
      class="pet"
      src="https://dummyimage.com/180x180/37a16b/ffffff.png&text=PET"
      alt="pet"
      :style="{ width: styleSize, height: styleSize }"
    />

    <div v-if="isSettingsOpen" class="settings" @mousedown.stop>
      <div class="settings-title">窗口设置</div>
      <label class="field">
        <span>宠物大小：{{ Math.round(scale * 100) }}%</span>
        <input v-model.number="scale" type="range" min="0.5" max="3" step="0.1" />
      </label>
      <div class="actions">
        <button @click="hideWindow">隐藏</button>
        <button @click="showWindow">显示</button>
        <button @click="closeSettingsPanel">关闭</button>
      </div>
    </div>

    <div class="toolbar" @mousedown.stop>
      <button @click="openSettingsPanel">设置</button>
    </div>
  </main>
</template>

<style scoped>
.app {
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  background: transparent;
  user-select: none;
}

.pet {
  object-fit: contain;
  cursor: grab;
}

.pet:active {
  cursor: grabbing;
}

.toolbar {
  position: absolute;
  right: 8px;
  bottom: 8px;
  display: flex;
  gap: 6px;
}

.toolbar button,
.actions button {
  border: 1px solid rgba(26, 32, 44, 0.25);
  background: rgba(255, 255, 255, 0.85);
  color: #0f172a;
  border-radius: 8px;
  font-size: 12px;
  padding: 4px 8px;
}

.settings {
  position: absolute;
  left: 8px;
  right: 8px;
  top: 8px;
  padding: 10px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.95);
  border: 1px solid rgba(26, 32, 44, 0.12);
  display: grid;
  gap: 10px;
}

.settings-title {
  font-size: 12px;
  font-weight: 600;
}

.field {
  display: grid;
  gap: 6px;
  font-size: 12px;
}

.actions {
  display: flex;
  gap: 6px;
}
</style>
