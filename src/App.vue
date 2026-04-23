<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, onMounted, ref } from 'vue'
import { loadScale, openSettings, showPetContextMenu } from './composables/useWindowManager'

const BASE_SIZE = 180
const scale = ref(1)
const styleSize = computed(() => `${BASE_SIZE * scale.value}px`)

onMounted(async () => {
  scale.value = await loadScale()
})

async function startDrag(): Promise<void> {
  await getCurrentWindow().startDragging()
}

async function onPetContextMenu(event: MouseEvent): Promise<void> {
  await showPetContextMenu(event.clientX, event.clientY)
}

async function openSettingsWindow(): Promise<void> {
  await openSettings()
}
</script>

<template>
  <main class="app">
    <img
      class="pet"
      src="https://dummyimage.com/180x180/37a16b/ffffff.png&text=PET"
      alt="pet"
      :style="{ width: styleSize, height: styleSize }"
      @mousedown.left="startDrag"
      @contextmenu.prevent="onPetContextMenu"
    />

    <!-- <div class="toolbar">
      <button @click="openSettingsWindow">设置</button>
    </div> -->
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
  pointer-events: none;
}

.pet {
  object-fit: contain;
  cursor: grab;
  pointer-events: auto;
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
  pointer-events: auto;
}

.toolbar button {
  border: 1px solid rgba(26, 32, 44, 0.25);
  background: rgba(255, 255, 255, 0.85);
  color: #0f172a;
  border-radius: 8px;
  font-size: 12px;
  padding: 4px 8px;
}
</style>
