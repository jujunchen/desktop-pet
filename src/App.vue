<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { currentGif, onPetClick, setupPetState } from './composables/usePetState'
import { loadScale, showPetContextMenu } from './composables/useWindowManager'

const BASE_SIZE = 180
const scale = ref(1)
const styleSize = computed(() => `${BASE_SIZE * scale.value}px`)
let teardownPetState: null | (() => void) = null

onMounted(async () => {
  scale.value = await loadScale()
  teardownPetState = setupPetState()
})

onUnmounted(() => {
  if (teardownPetState) {
    teardownPetState()
    teardownPetState = null
  }
})

async function startDrag(): Promise<void> {
  await getCurrentWindow().startDragging()
}

async function onPetContextMenu(event: MouseEvent): Promise<void> {
  await showPetContextMenu(event.clientX, event.clientY)
}
</script>

<template>
  <main class="app">
    <img
      class="pet"
      :src="currentGif"
      alt="pet"
      :style="{ width: styleSize, height: styleSize }"
      @click.stop="onPetClick"
      @mousedown.left="startDrag"
      @contextmenu.prevent="onPetContextMenu"
    />
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
  background: transparent;
  border: none;
  outline: none;
  box-shadow: none;
}

.pet:active {
  cursor: grabbing;
}
</style>
