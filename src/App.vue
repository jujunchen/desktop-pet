<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { currentGif, onPetClick, setupPetState, showSpeechBubble, speechBubbleText } from './composables/usePetState'
import { loadScale, showPetContextMenu } from './composables/useWindowManager'

const BASE_SIZE = 180
const scale = ref(1)
const styleSize = computed(() => `${BASE_SIZE * scale.value}px`)
const bubbleStyleSize = computed(() => `${BASE_SIZE * scale.value * 0.8}px`)
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

function onMouseDown(event: MouseEvent): void {
  console.log('[鼠标] 左键按下：触发点击 + 允许拖拽')
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
      <div v-if="showSpeechBubble" class="speech-bubble" :style="{ maxWidth: bubbleStyleSize }">
        {{ speechBubbleText }}
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
  top: 50%;
  left: 0;
  transform: translateX(-120%) translateY(-50%);
  background: white;
  padding: 12px 8px;
  border-radius: 18px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.15);
  font-size: 14px;
  color: #333;
  z-index: 10;
  pointer-events: none;
  writing-mode: vertical-rl;
  text-orientation: upright;
  letter-spacing: 2px;
}

.speech-bubble::after {
  content: '';
  position: absolute;
  right: -8px;
  top: 50%;
  transform: translateY(-50%);
  border-top: 8px solid transparent;
  border-bottom: 8px solid transparent;
  border-left: 8px solid white;
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

.bubble-fade-enter-active,
.bubble-fade-leave-active {
  transition: all 0.3s ease;
}

.bubble-fade-enter-from {
  opacity: 0;
  transform: translateX(-50%) translateY(10px);
}

.bubble-fade-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-10px);
}
</style>
