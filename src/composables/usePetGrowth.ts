import { invoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'
import { ref, onUnmounted } from 'vue'

export const EVT_GROWTH_CHANGED = 'pet://growth-changed'
export const EVT_STAGE_CHANGED = 'pet://stage-changed'
export const EVT_PET_DIED = 'pet://died'

export type PetMode = 'Growth' | 'Assistant'
export type LifeStage = 'Baby' | 'Adult' | 'Elder' | 'Dead'

export interface GrowthState {
  stage: LifeStage
  affection: number
  growth: number
  hunger: number
  happiness: number
  health: number
  created_at: number
  last_fed_at: number
  last_interacted_at: number
  reincarnation_count: number
  inherited_bonus: number
  last_updated_at: number
}

export interface PetGrowthConfig {
  mode: PetMode
  growth: GrowthState
}

// 全局状态
const growthState = ref<GrowthState | null>(null)
const petMode = ref<PetMode>('Assistant')
let updateTimer: number | null = null
let initialized = false

const DAY_SECONDS = 86400
const BABY_TO_ADULT_DAYS = 7
const ADULT_TO_ELDER_DAYS = 300
const MAX_LIFESPAN_DAYS = 365

// 随机数工具
function randomInRange(min: number, max: number): number {
  return Math.random() * (max - min) + min
}

// 计算存活天数
function getAliveDays(state: GrowthState): number {
  const now = Math.floor(Date.now() / 1000)
  return (now - state.created_at) / DAY_SECONDS
}

// 判断是否可以执行指令
export function canExecuteCommands(): boolean {
  if (petMode.value === 'Assistant') return true
  if (!growthState.value) return true
  return growthState.value.stage !== 'Baby' && growthState.value.stage !== 'Dead'
}

// 判断宠物是否活着
export function isPetAlive(): boolean {
  if (!growthState.value) return true
  return growthState.value.stage !== 'Dead'
}

// 获取当前生命阶段
export function getCurrentStage(): LifeStage | null {
  return growthState.value?.stage || null
}

// 获取状态数值（带标签）
export function getGrowthValues() {
  if (!growthState.value) return null
  return {
    affection: growthState.value.affection,
    growth: growthState.value.growth,
    hunger: growthState.value.hunger,
    happiness: growthState.value.happiness,
    health: growthState.value.health,
    aliveDays: getAliveDays(growthState.value),
    reincarnationCount: growthState.value.reincarnation_count,
    inheritedBonus: growthState.value.inherited_bonus
  }
}

// 喂食：亲密度 +3~6，饥饿值 +30，快乐值 +10~15
export async function feedPet(): Promise<void> {
  if (!isPetAlive()) return
  await invoke('feed_pet')
}

// 玩耍：随机播放动作，亲密度 +1~2，快乐值 +3~5
export async function playWithPet(): Promise<void> {
  if (!isPetAlive()) return
  await invoke('play_with_pet')
}

// 点击宠物增加属性
export async function onPetClicked(): Promise<void> {
  if (!isPetAlive()) return
  await invoke('on_pet_clicked')
}

// 聊天增加亲密度
export async function onChatCompleted(): Promise<void> {
  if (!isPetAlive()) return
  await invoke('on_chat_completed')
}

// 转世
export async function reincarnatePet(keepName: boolean): Promise<void> {
  await invoke('reincarnate_pet', { keepName })
}

// 手动触发状态更新（一般不需要，会自动定时更新）
export async function updateGrowthState(): Promise<void> {
  await invoke('update_growth_state')
}

// 加载养成状态
export async function loadGrowthState(): Promise<PetGrowthConfig | null> {
  try {
    const result = await invoke<PetGrowthConfig>('get_growth_state')
    petMode.value = result.mode
    growthState.value = result.growth
    return result
  } catch (e) {
    console.error('Failed to load growth state:', e)
    return null
  }
}

// 启动养成系统
export async function setupPetGrowth(): Promise<() => void> {
  if (initialized) {
    return cleanupPetGrowth
  }
  initialized = true

  // 加载初始状态
  await loadGrowthState()

  // 监听状态变化事件
  const unlistenGrowth = await listen<GrowthState>(EVT_GROWTH_CHANGED, (event) => {
    growthState.value = event.payload
  })

  const unlistenStage = await listen<LifeStage>(EVT_STAGE_CHANGED, (event) => {
    console.log('Pet stage changed to:', event.payload)
    if (growthState.value) {
      growthState.value.stage = event.payload
    }
  })

  const unlistenDied = await listen(EVT_PET_DIED, () => {
    console.log('Pet died!')
    if (growthState.value) {
      growthState.value.stage = 'Dead'
    }
  })

  // 启动定时更新（每分钟更新一次属性衰减）
  updateTimer = window.setInterval(() => {
    void updateGrowthState()
  }, 60000)

  // 启动后立即更新一次（计算离线期间的变化）
  await updateGrowthState()

  return function cleanup() {
    unlistenGrowth()
    unlistenStage()
    unlistenDied()
    if (updateTimer) {
      window.clearInterval(updateTimer)
      updateTimer = null
    }
    initialized = false
  }
}

export function cleanupPetGrowth(): void {
  if (updateTimer) {
    window.clearInterval(updateTimer)
    updateTimer = null
  }
  initialized = false
}

// 导出状态供组件使用
export { growthState, petMode }
