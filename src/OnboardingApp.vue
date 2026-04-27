<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'

type PetMode = 'Growth' | 'Assistant'

const step = ref(1)
const selectedMode = ref<PetMode>('Assistant')
const petName = ref('小白')
const petPrompt = ref('你是一只可爱的桌面宠物，名字叫{name}。你的性格活泼、友好、有点调皮。请用简短、口语化的方式回复，不要太长。回复时要像宠物一样可爱，可以用一些语气词如"汪"、"呀"、"呢"等。')
const saving = ref(false)

const personalityPresets = [
  {
    name: '活泼调皮',
    prompt: '你是一只活泼调皮的桌面宠物，名字叫{name}。喜欢玩耍，经常搞点小恶作剧，说话充满活力，喜欢用"汪！"、"嘿嘿"等语气词。'
  },
  {
    name: '温柔体贴',
    prompt: '你是一只温柔体贴的桌面宠物，名字叫{name}。性格安静温暖，会关心主人，说话轻柔，喜欢用"呢"、"呀"等温和的语气词。'
  },
  {
    name: '高冷傲娇',
    prompt: '你是一只高冷傲娇的桌面宠物，名字叫{name}。表面冷漠但内心关心主人，说话带点傲娇，喜欢用"哼"、"切"等语气词。'
  },
  {
    name: '呆萌可爱',
    prompt: '你是一只呆萌可爱的桌面宠物，名字叫{name}。反应有点慢，经常发呆，说话傻乎乎的，喜欢用"啊？"、"唔"等语气词。'
  }
]

const modeDescription = computed(() => {
  if (selectedMode.value === 'Growth') {
    return {
      title: '养成模式',
      desc: '宠物从幼体开始成长，需要你喂食、互动陪伴。长大后才能帮你执行指令。体验真实的养宠乐趣，见证它的成长与衰老。',
      features: ['幼体无法执行指令', '需要喂食和互动', '亲密度随时间增长', '会经历成长、衰老、死亡', '可转世继承属性']
    }
  }
  return {
    title: '助手模式',
    desc: '直接获得成年宠物，拥有完整功能。可以立即让它帮你执行系统指令、控制电脑等操作，同时仍能通过互动增加亲密度。',
    features: ['立即使用全部功能', '可执行系统指令', '不会衰老死亡', '亲密度仍会增长', '适合追求效率']
  }
})

function selectMode(mode: PetMode): void {
  selectedMode.value = mode
}

function selectPersonality(index: number): void {
  petPrompt.value = personalityPresets[index].prompt
}

function nextStep(): void {
  if (step.value < 3) {
    step.value++
  }
}

function prevStep(): void {
  if (step.value > 1) {
    step.value--
  }
}

async function completeSetup(): Promise<void> {
  if (saving.value) return
  saving.value = true

  try {
    const config = await invoke<any>('load_config')
    config.pet.name = petName.value.trim() || '小白'
    config.pet.prompt = petPrompt.value.replace('{name}', petName.value.trim() || '小白')
    config.pet.mode = selectedMode.value

    // 如果是养成模式，重置为幼体状态
    if (selectedMode.value === 'Growth') {
      await invoke('reset_pet_growth')
    }

    await invoke('save_config', { config })

    // 标记已完成引导
    await invoke('set_onboarding_completed')

    // 关闭引导窗口，显示主窗口
    await invoke('show_main_window')
    await getCurrentWindow().close()
  } catch (e) {
    console.error('保存配置失败:', e)
  } finally {
    saving.value = false
  }
}

onMounted(() => {
  // 设置窗口大小
  getCurrentWindow().setSize({ width: 480, height: 640 })
})
</script>

<template>
  <main class="onboarding-page">
    <!-- 进度指示器 -->
    <div class="progress-indicator">
      <div class="progress-step" :class="{ active: step >= 1, completed: step > 1 }">
        <span class="step-number">1</span>
        <span class="step-label">选择模式</span>
      </div>
      <div class="progress-line" :class="{ active: step > 1 }"></div>
      <div class="progress-step" :class="{ active: step >= 2, completed: step > 2 }">
        <span class="step-number">2</span>
        <span class="step-label">宠物信息</span>
      </div>
      <div class="progress-line" :class="{ active: step > 2 }"></div>
      <div class="progress-step" :class="{ active: step >= 3 }">
        <span class="step-number">3</span>
        <span class="step-label">完成</span>
      </div>
    </div>

    <!-- Step 1: 模式选择 -->
    <Transition name="slide-fade" mode="out-in">
      <div v-if="step === 1" class="step-content" key="step1">
        <h1 class="step-title">欢迎使用桌面宠物</h1>
        <p class="step-desc">请选择你喜欢的养宠模式</p>

        <div class="mode-cards">
          <div
            class="mode-card"
            :class="{ selected: selectedMode === 'Assistant' }"
            @click="selectMode('Assistant')"
          >
            <div class="mode-icon">🚀</div>
            <h3>{{ modeDescription.title }}</h3>
            <p class="mode-desc-text">{{ modeDescription.desc }}</p>
            <ul class="mode-features">
              <li v-for="(feat, idx) in modeDescription.features" :key="idx">{{ feat }}</li>
            </ul>
            <div class="mode-check" v-if="selectedMode === 'Assistant'">✓</div>
          </div>

          <div
            class="mode-card"
            :class="{ selected: selectedMode === 'Growth' }"
            @click="selectMode('Growth')"
          >
            <div class="mode-icon">🌱</div>
            <h3>{{ modeDescription.title }}</h3>
            <p class="mode-desc-text">{{ modeDescription.desc }}</p>
            <ul class="mode-features">
              <li v-for="(feat, idx) in modeDescription.features" :key="idx">{{ feat }}</li>
            </ul>
            <div class="mode-check" v-if="selectedMode === 'Growth'">✓</div>
          </div>
        </div>

        <div class="step-actions">
          <button class="btn-primary" @click="nextStep">下一步 →</button>
        </div>
      </div>

      <!-- Step 2: 宠物信息 -->
      <div v-else-if="step === 2" class="step-content" key="step2">
        <h1 class="step-title">设置宠物信息</h1>
        <p class="step-desc">给你的宠物起个名字，设定它的性格</p>

        <div class="form-group">
          <label class="form-label">宠物名字</label>
          <input
            v-model="petName"
            type="text"
            class="form-input"
            placeholder="请输入宠物名字"
            maxlength="10"
          />
        </div>

        <div class="form-group">
          <label class="form-label">选择性格（可自定义下方描述）</label>
          <div class="personality-grid">
            <button
              v-for="(preset, idx) in personalityPresets"
              :key="idx"
              class="personality-btn"
              :class="{ active: petPrompt === preset.prompt }"
              @click="selectPersonality(idx)"
            >
              {{ preset.name }}
            </button>
          </div>
        </div>

        <div class="form-group">
          <label class="form-label">宠物性格描述</label>
          <textarea
            v-model="petPrompt"
            class="form-textarea"
            rows="4"
            placeholder="描述你希望宠物具备的性格特点..."
          ></textarea>
        </div>

        <div class="step-actions two-buttons">
          <button class="btn-secondary" @click="prevStep">← 上一步</button>
          <button class="btn-primary" @click="nextStep">下一步 →</button>
        </div>
      </div>

      <!-- Step 3: 完成 -->
      <div v-else-if="step === 3" class="step-content step-final" key="step3">
        <div class="final-icon">🎉</div>
        <h1 class="step-title">设置完成！</h1>
        <p class="step-desc">
          你即将领养一只名为 <strong class="pet-name-highlight">{{ petName }}</strong> 的可爱宠物
        </p>

        <div class="summary-card">
          <div class="summary-item">
            <span class="summary-label">模式</span>
            <span class="summary-value">{{ selectedMode === 'Growth' ? '🌱 养成模式' : '🚀 助手模式' }}</span>
          </div>
          <div class="summary-item">
            <span class="summary-label">名字</span>
            <span class="summary-value">{{ petName }}</span>
          </div>
          <div class="summary-item">
            <span class="summary-label">初始状态</span>
            <span class="summary-value">{{ selectedMode === 'Growth' ? '🐣 幼体阶段' : '🐕 成年阶段' }}</span>
          </div>
        </div>

        <p class="final-hint">
          {{ selectedMode === 'Growth'
            ? '记得经常陪伴它、给它喂食，它会慢慢长大的哦~'
            : '它已经准备好为你服务了，快去和它聊聊吧！'
          }}
        </p>

        <div class="step-actions">
          <button class="btn-secondary" @click="prevStep">← 返回修改</button>
          <button class="btn-primary btn-start" @click="completeSetup" :disabled="saving">
            {{ saving ? '创建中...' : '开始养宠！' }}
          </button>
        </div>
      </div>
    </Transition>
  </main>
</template>

<style scoped>
.onboarding-page {
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  padding: 24px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  display: flex;
  flex-direction: column;
}

.progress-indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 32px;
}

.progress-step {
  display: flex;
  flex-direction: column;
  align-items: center;
  color: rgba(255, 255, 255, 0.5);
  transition: color 0.3s;
}

.progress-step.active,
.progress-step.completed {
  color: white;
}

.step-number {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.2);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  margin-bottom: 6px;
  transition: background 0.3s;
}

.progress-step.active .step-number,
.progress-step.completed .step-number {
  background: white;
  color: #667eea;
}

.step-label {
  font-size: 12px;
}

.progress-line {
  width: 40px;
  height: 2px;
  background: rgba(255, 255, 255, 0.2);
  margin: 0 8px 18px;
  transition: background 0.3s;
}

.progress-line.active {
  background: white;
}

.step-content {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.step-title {
  color: white;
  font-size: 24px;
  font-weight: 700;
  margin: 0 0 8px;
  text-align: center;
}

.step-desc {
  color: rgba(255, 255, 255, 0.8);
  font-size: 14px;
  text-align: center;
  margin: 0 0 24px;
}

.mode-cards {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 24px;
}

.mode-card {
  background: white;
  border-radius: 16px;
  padding: 20px;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
  position: relative;
  border: 3px solid transparent;
}

.mode-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
}

.mode-card.selected {
  border-color: #667eea;
  box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.2);
}

.mode-icon {
  font-size: 32px;
  margin-bottom: 8px;
}

.mode-card h3 {
  margin: 0 0 8px;
  font-size: 18px;
  color: #333;
}

.mode-desc-text {
  margin: 0 0 12px;
  font-size: 13px;
  color: #666;
  line-height: 1.5;
}

.mode-features {
  margin: 0;
  padding-left: 20px;
}

.mode-features li {
  font-size: 12px;
  color: #888;
  margin-bottom: 4px;
}

.mode-check {
  position: absolute;
  top: 16px;
  right: 16px;
  width: 24px;
  height: 24px;
  background: #667eea;
  color: white;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  font-weight: bold;
}

.form-group {
  margin-bottom: 20px;
}

.form-label {
  display: block;
  color: white;
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 8px;
}

.form-input {
  width: 100%;
  padding: 12px 16px;
  border: none;
  border-radius: 12px;
  font-size: 15px;
  outline: none;
  background: white;
  box-sizing: border-box;
}

.form-input::placeholder {
  color: #aaa;
}

.form-textarea {
  width: 100%;
  padding: 12px 16px;
  border: none;
  border-radius: 12px;
  font-size: 14px;
  outline: none;
  background: white;
  resize: none;
  box-sizing: border-box;
  font-family: inherit;
}

.personality-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}

.personality-btn {
  padding: 10px 12px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  background: rgba(255, 255, 255, 0.15);
  color: white;
  border-radius: 10px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.personality-btn:hover {
  background: rgba(255, 255, 255, 0.25);
}

.personality-btn.active {
  background: white;
  color: #667eea;
  border-color: white;
}

.step-actions {
  margin-top: auto;
  padding-top: 16px;
}

.step-actions.two-buttons {
  display: flex;
  gap: 12px;
}

.step-actions.two-buttons button {
  flex: 1;
}

.btn-primary {
  width: 100%;
  padding: 14px 24px;
  background: white;
  color: #667eea;
  border: none;
  border-radius: 12px;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.btn-primary:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.btn-primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-secondary {
  padding: 14px 24px;
  background: rgba(255, 255, 255, 0.2);
  color: white;
  border: none;
  border-radius: 12px;
  font-size: 16px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-secondary:hover {
  background: rgba(255, 255, 255, 0.3);
}

.step-final {
  text-align: center;
}

.final-icon {
  font-size: 64px;
  margin-bottom: 16px;
}

.pet-name-highlight {
  color: #ffd700;
  font-size: 18px;
}

.summary-card {
  background: white;
  border-radius: 16px;
  padding: 20px;
  margin: 24px 0;
  text-align: left;
}

.summary-item {
  display: flex;
  justify-content: space-between;
  padding: 10px 0;
  border-bottom: 1px solid #f0f0f0;
}

.summary-item:last-child {
  border-bottom: none;
}

.summary-label {
  color: #666;
  font-size: 14px;
}

.summary-value {
  color: #333;
  font-size: 14px;
  font-weight: 600;
}

.final-hint {
  color: rgba(255, 255, 255, 0.9);
  font-size: 14px;
  margin: 0 0 24px;
}

.btn-start {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
  color: white;
}

/* 动画 */
.slide-fade-enter-active,
.slide-fade-leave-active {
  transition: opacity 0.3s, transform 0.3s;
}

.slide-fade-enter-from {
  opacity: 0;
  transform: translateX(20px);
}

.slide-fade-leave-to {
  opacity: 0;
  transform: translateX(-20px);
}
</style>
