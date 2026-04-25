<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { currentGif, onPetClick, setupPetState, setupPetStateEventListeners, showSpeechBubble, truncatedSpeechText } from './composables/usePetState'
import { loadScale, showPetContextMenu } from './composables/useWindowManager'

const BASE_SIZE = 180
const scale = ref(1)
const styleSize = computed(() => `${BASE_SIZE * scale.value}px`)
const speechFontSize = computed(() => `${Math.max(10, Math.round(14 * scale.value))}px`)
const speechMaxHeight = computed(() => 'calc(100vh - 16px)')
const formattedSpeechText = computed(() => truncatedSpeechText.value ?? '')
let teardownPetState: null | (() => void) = null

let teardownEventListeners: (() => void) | null = null

onMounted(async () => {
  scale.value = await loadScale()
  teardownPetState = setupPetState()
  teardownEventListeners = await setupPetStateEventListeners()
})

onUnmounted(() => {
  if (teardownPetState) {
    teardownPetState()
    teardownPetState = null
  }
  if (teardownEventListeners) {
    teardownEventListeners()
    teardownEventListeners = null
  }
})

function onMouseDown(event: MouseEvent): void {
  onPetClick()
  void getCurrentWindow().startDragging()
}

async function onPetContextMenu(event: MouseEvent): Promise<void> {
  await showPetContextMenu(event.clientX, event.clientY)
}
</script>

<template>
  <main class="app">
    <!-- 泡泡文字 -->
    <Transition name="bubble-fade">
      <div
        v-if="showSpeechBubble"
        class="speech-bubble"
        :style="{
          '--speech-font-size': speechFontSize,
          '--speech-max-height': speechMaxHeight
        }"
      >
        {{ formattedSpeechText }}
      </div>
    </Transition>
    <Transition name="pet-fade" mode="out-in">
      <img
        :key="currentGif"
        class="pet"
        :src="currentGif"
        alt="pet"
        :style="{ width: styleSize, height: styleSize }"
        @mousedown.left="onMouseDown"
        @contextmenu.prevent="onPetContextMenu"
      />
    </Transition>
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

.speech-bubble {
  position: absolute;
  top: 0%;
  left: 0px;
  font-size: var(--speech-font-size, 8px);
  color: #333;
  z-index: 10;
  pointer-events: none;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(0, 0, 0, 0.45);
  border-radius: 10px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
  padding: 6px 8px;
  max-height: var(--speech-max-height, calc(100vh - 16px));
  max-width: calc(3em + 20px);
  overflow: hidden;
  white-space: pre-wrap;
  writing-mode: vertical-rl;
  text-orientation: upright;
  line-height: 1.2;
  letter-spacing: 2px;
  word-break: break-all;
  overflow-wrap: anywhere;
}

.bubble-fade-enter-active,
.bubble-fade-leave-active {
  transition: opacity 0.25s ease;
}

.bubble-fade-enter-from {
  opacity: 0;
}

.bubble-fade-leave-to {
  opacity: 0;
}

.bubble-fade-enter-to,
.bubble-fade-leave-from {
  opacity: 1;
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

.pet-fade-enter-active,
.pet-fade-leave-active {
  transition: opacity 0.3s ease;
}

.pet-fade-enter-from,
.pet-fade-leave-to {
  opacity: 0;
}

</style>
