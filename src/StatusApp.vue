<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { growthState, petMode, loadGrowthState, EVT_GROWTH_CHANGED, reincarnatePet } from './composables/usePetGrowth'
import { listen } from '@tauri-apps/api/event'

const loading = ref(true)

const stageText = computed(() => {
  if (!growthState.value) return '-'
  const stageMap: Record<string, string> = {
    Baby: '幼体',
    Adult: '成年',
    Elder: '老年',
    Dead: '已死亡'
  }
  return stageMap[growthState.value.stage] || growthState.value.stage
})

const modeText = computed(() => {
  return petMode.value === 'Assistant' ? '助手模式' : '养成模式'
})

const aliveDays = computed(() => {
  if (!growthState.value) return 0
  const now = Math.floor(Date.now() / 1000)
  return Math.floor((now - growthState.value.created_at) / 86400)
})

function formatValue(value: number): string {
  return Math.round(value).toString()
}

function getProgressColor(value: number): string {
  if (value >= 70) return '#4caf50'
  if (value >= 40) return '#ff9800'
  return '#f44336'
}

async function reincarnate() {
  try {
    await reincarnatePet(true)
  } catch (e) {
    console.error('转世失败:', e)
  }
}

onMounted(async () => {
  await loadGrowthState()
  loading.value = false

  await listen(EVT_GROWTH_CHANGED, () => {
    // 状态已由 usePetGrowth 自动更新
  })
})
</script>

<template>
  <main class="status-page">
    <header class="header">
      <span class="mode-badge" :class="petMode">{{ modeText }}</span>
    </header>

    <div v-if="loading" class="loading">加载中...</div>

    <template v-else-if="growthState">
      <section class="status-card">
        <h3>基本信息</h3>
        <div class="info-row">
          <span class="label">生命阶段</span>
          <span class="value stage" :class="growthState.stage">{{ stageText }}</span>
        </div>
        <div class="info-row">
          <span class="label">已存活</span>
          <span class="value">{{ aliveDays }} 天</span>
        </div>
        <div v-if="growthState.reincarnation_count > 0" class="info-row">
          <span class="label">转世次数</span>
          <span class="value">{{ growthState.reincarnation_count }} 世</span>
        </div>
        <div v-if="growthState.inherited_bonus > 0" class="info-row">
          <span class="label">继承加成</span>
          <span class="value bonus">+{{ formatValue(growthState.inherited_bonus) }}%</span>
        </div>
      </section>

      <section class="status-card">
        <h3>属性值</h3>

        <div class="progress-item">
          <div class="progress-header">
            <span class="label">亲密度</span>
            <span class="value">{{ formatValue(growthState.affection) }}/100</span>
          </div>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: `${growthState.affection}%`, backgroundColor: '#e91e63' }"></div>
          </div>
        </div>

        <div class="progress-item">
          <div class="progress-header">
            <span class="label">饥饿值</span>
            <span class="value">{{ formatValue(growthState.hunger) }}/100</span>
          </div>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: `${growthState.hunger}%`, backgroundColor: getProgressColor(growthState.hunger) }"></div>
          </div>
        </div>

        <div class="progress-item">
          <div class="progress-header">
            <span class="label">快乐值</span>
            <span class="value">{{ formatValue(growthState.happiness) }}/100</span>
          </div>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: `${growthState.happiness}%`, backgroundColor: '#ffeb3b' }"></div>
          </div>
        </div>

        <div class="progress-item">
          <div class="progress-header">
            <span class="label">生命值</span>
            <span class="value">{{ formatValue(growthState.health) }}/100</span>
          </div>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: `${growthState.health}%`, backgroundColor: getProgressColor(growthState.health) }"></div>
          </div>
        </div>

        <div class="progress-item">
          <div class="progress-header">
            <span class="label">成长值</span>
            <span class="value">{{ formatValue(growthState.growth) }}/100</span>
          </div>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: `${growthState.growth}%`, backgroundColor: '#9c27b0' }"></div>
          </div>
        </div>
      </section>

      <section v-if="growthState.stage === 'Dead'" class="status-card reincarnation">
        <h3>转世</h3>
        <p class="reincarnation-desc">你的宠物已经离开这个世界了...</p>
        <p class="reincarnation-hint">转世后将继承上一世 {{ formatValue(growthState.affection * 0.1) }}% 的亲密度加成</p>
        <button class="reincarnation-btn" @click="reincarnate">开始新的一世</button>
      </section>
    </template>
  </main>
</template>

<style scoped>
.status-page {
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  padding: 16px 20px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  overflow-y: auto;
}

.header {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  margin-bottom: 16px;
}

.mode-badge {
  padding: 6px 12px;
  border-radius: 20px;
  font-size: 12px;
  font-weight: 500;
}

.mode-badge.Assistant {
  background: rgba(255, 255, 255, 0.25);
  color: white;
}

.mode-badge.Growth {
  background: rgba(255, 235, 59, 0.9);
  color: #5d4037;
}

.loading {
  text-align: center;
  padding: 40px;
  color: white;
  font-size: 16px;
}

.status-card {
  background: white;
  border-radius: 16px;
  padding: 20px;
  margin-bottom: 16px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.status-card h3 {
  margin: 0 0 16px 0;
  font-size: 16px;
  font-weight: 600;
  color: #333;
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 0;
  border-bottom: 1px solid #f0f0f0;
}

.info-row:last-child {
  border-bottom: none;
}

.info-row .label {
  color: #666;
  font-size: 14px;
}

.info-row .value {
  font-weight: 600;
  color: #333;
  font-size: 15px;
}

.info-row .value.stage {
  padding: 4px 12px;
  border-radius: 12px;
  font-size: 13px;
}

.stage.Baby {
  background: #e3f2fd;
  color: #1976d2;
}

.stage.Adult {
  background: #e8f5e9;
  color: #388e3c;
}

.stage.Elder {
  background: #fff3e0;
  color: #f57c00;
}

.stage.Dead {
  background: #ffebee;
  color: #d32f2f;
}

.info-row .value.bonus {
  color: #4caf50;
}

.progress-item {
  margin-bottom: 16px;
}

.progress-item:last-child {
  margin-bottom: 0;
}

.progress-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 6px;
}

.progress-header .label {
  font-size: 14px;
  color: #555;
}

.progress-header .value {
  font-size: 13px;
  color: #888;
  font-weight: 500;
}

.progress-bar {
  height: 8px;
  background: #eee;
  border-radius: 4px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.3s ease;
}

.reincarnation {
  background: linear-gradient(135deg, #fff8e1 0%, #ffecb3 100%);
  text-align: center;
}

.reincarnation h3 {
  color: #f57c00;
}

.reincarnation-desc {
  margin: 0 0 8px 0;
  color: #5d4037;
  font-size: 15px;
}

.reincarnation-hint {
  margin: 0 0 16px 0;
  color: #795548;
  font-size: 13px;
}

.reincarnation-btn {
  width: 100%;
  padding: 12px 24px;
  background: linear-gradient(135deg, #ff9800 0%, #f57c00 100%);
  color: white;
  border: none;
  border-radius: 12px;
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.reincarnation-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(255, 152, 0, 0.4);
}

.reincarnation-btn:active {
  transform: translateY(0);
}
</style>
