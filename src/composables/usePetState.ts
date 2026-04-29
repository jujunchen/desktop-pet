import { invoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'
import { computed, onMounted, ref, watch } from 'vue'
import { onPetClicked as growthOnPetClicked, isPetAlive, growthState, loadGrowthState, EVT_GROWTH_CHANGED, type LifeStage } from './usePetGrowth'

export type BaseState = 'sitting' | 'sleeping'
export type TempAction =
  | 'talking'
  | 'happy'
  | 'tilt-head'
  | 'crazy'
  | 'crazy-plus'
  | 'running'
  | 'backing'
  | 'bored'
  | 'angry'
  | 'dance'
  | 'frisbee'
  | 'curious'
  | 'eat'
  | 'hungry'
  | 'dead'

const EVT_PET_PROCESSING = 'pet://processing'
const EVT_PET_SPEAKING = 'pet://speaking'

// 跨窗口通知宠物进入思考状态
export async function enterProcessingState(): Promise<void> {
  await emit(EVT_PET_PROCESSING, {})
}

// 跨窗口通知宠物进入说话状态
export async function enterSpeakingState(text: string): Promise<void> {
  await emit(EVT_PET_SPEAKING, { text })
}
export type PetName = 'dog'

type PetStateKey = BaseState | TempAction

type ActionPriority = 'instruction' | 'interaction' | 'idle-transition'

interface QueuedAction {
  name: TempAction
  duration: number
  priority: number
  interruptible: boolean
  createdAt: number
}

interface StateMeta {
  gifSrc: string
  loop: boolean
  singlePlayMs?: number
}

const CLICK_WINDOW_MS = 3_000
const SITTING_TO_SLEEPING_MS = 60_000 // 60秒无操作进入睡眠
const SITTING_TO_BORED_MS = 600_000 // 10分钟进入无聊状态
const TALKING_REMINDER_INTERVAL_MS = 3600_000 // 1小时连续使用提醒休息
const IDLE_CHECK_INTERVAL_MS = 1_000

type StageKey = 'baby' | 'adult' | 'elder'

function createStateMetaForStage(stage: StageKey, states: PetStateKey[]): Record<PetStateKey, StateMeta> {
  const meta: Record<string, StateMeta> = {}
  for (const state of states) {
    meta[state] = {
      gifSrc: new URL(`../assets/pets/dog/${stage}/${state}.gif`, import.meta.url).href,
      loop: state === 'sitting' || state === 'sleeping' || state === 'curious',
      singlePlayMs: state === 'sitting' || state === 'sleeping' || state === 'curious'
        ? undefined
        : (state === 'running' ? 180_000 : state === 'dance' || state === 'frisbee' ? 5_000 : 3_000)
    }
  }
  return meta as Record<PetStateKey, StateMeta>
}

const allStates: PetStateKey[] = ['sitting', 'sleeping', 'talking', 'happy', 'tilt-head', 'bored', 'backing', 'crazy', 'crazy-plus', 'angry', 'dance', 'frisbee', 'running', 'curious', 'eat', 'hungry', 'dead']
const babyStates: PetStateKey[] = ['sitting', 'sleeping', 'talking', 'happy', 'tilt-head', 'curious', 'running', 'eat', 'hungry', 'dead']
const elderStates: PetStateKey[] = ['sitting', 'sleeping', 'bored', 'talking', 'curious', 'eat', 'hungry', 'dead']

const PET_STATE_META: Record<PetName, Record<StageKey, Partial<Record<PetStateKey, StateMeta>>>> = {
  dog: {
    baby: createStateMetaForStage('baby', babyStates),
    adult: createStateMetaForStage('adult', allStates),
    elder: createStateMetaForStage('elder', elderStates),
  }
}

// 各阶段允许的动作配置
const STAGE_ALLOWED_ACTIONS: Record<StageKey, {
  allowed: PetStateKey[]
  replacements: Partial<Record<PetStateKey, PetStateKey>>
}> = {
  baby: {
    allowed: ['sitting', 'sleeping', 'happy', 'tilt-head', 'dance', 'eat', 'hungry', 'dead'],
    replacements: {
      'dance': 'happy',
      'frisbee': 'happy',
      'angry': 'tilt-head',
      'bored': 'tilt-head',
      'crazy': 'happy',
      'crazy-plus': 'happy',
      'backing': 'happy',
      'talking': 'happy',
    }
  },
  adult: {
    allowed: ['sitting', 'sleeping', 'happy', 'angry', 'bored', 'curious',
              'dance', 'frisbee', 'talking', 'tilt-head', 'running',
              'crazy', 'crazy-plus', 'backing', 'eat', 'hungry', 'dead'],
    replacements: {}
  },
  elder: {
    allowed: ['sitting', 'sleeping', 'happy', 'talking', 'eat', 'hungry', 'dead'],
    replacements: {
      'dance': 'happy',
      'frisbee': 'happy',
      'running': 'happy',
      'angry': 'talking',
      'tilt-head': 'talking',
      'crazy': 'happy',
      'crazy-plus': 'happy',
      'backing': 'sitting',
    }
  }
}

const priorityValue: Record<ActionPriority, number> = {
  instruction: 3,
  interaction: 2,
  'idle-transition': 1
}

const currentPet = ref<PetName>('dog')
const baseState = ref<BaseState>('sitting')
const currentAction = ref<TempAction | null>(null)
const actionQueue = ref<QueuedAction[]>([])
const currentActionPriority = ref(0)
const currentActionInterruptible = ref(true)
const clickCount = ref(0)
const lastTriggeredMilestone = ref(0)
const firstClickTime = ref(0)
const lastClickTime = ref(0)
const lastActivityTime = ref(Date.now())
const lastTalkingReminderTime = ref(Date.now())
const boredTriggeredInCurrentIdle = ref(false)
// 泡泡文字
const showSpeechBubble = ref(false)
const speechBubbleText = ref('')
let checkingSystemIdle = false

let actionTimer: number | null = null
let idleChecker: number | null = null
let speechTimer: number | null = null
let started = false

function getStageKey(stage: LifeStage | undefined): StageKey {
  if (stage === 'Baby') return 'baby'
  if (stage === 'Elder') return 'elder'
  return 'adult'
}

function getEffectiveAction(state: PetStateKey, stage: LifeStage | undefined): PetStateKey {
  const stageKey = getStageKey(stage)
  const config = STAGE_ALLOWED_ACTIONS[stageKey]

  if (config.allowed.includes(state)) {
    return state
  }

  return config.replacements[state] || 'sitting'
}

function getStateMetaByStage(state: PetStateKey, stage: LifeStage | undefined): StateMeta {
  const stageKey = getStageKey(stage)
  const pet = currentPet.value

  // 优先使用当前阶段的资源
  const stageMeta = PET_STATE_META[pet]?.[stageKey]?.[state]
  if (stageMeta) {
    return stageMeta
  }

  // 回退到成年阶段的资源
  return PET_STATE_META[pet].adult[state]!
}

function getStateMeta(state: PetStateKey): StateMeta {
  return getStateMetaByStage(state, growthState.value?.stage)
}

function getSinglePlayDuration(action: TempAction): number {
  const meta = getStateMeta(action)
  if (meta.loop) {
    throw new Error(`Action ${action} is configured as loop state`)
  }
  if (!meta.singlePlayMs) {
    throw new Error(`Action ${action} is missing singlePlayMs`)
  }
  return meta.singlePlayMs
}

// 根据生命阶段和属性状态调整显示的GIF动画
const effectiveBaseState = computed((): PetStateKey => {
  const stage = growthState.value?.stage
  const hunger = growthState.value?.hunger ?? 100

  // 死亡状态最高优先级：忽略动作队列与其他状态，强制播放 dead 动画
  if (stage === 'Dead') {
    return 'dead'
  }

  // 如果有临时动作，优先显示动作动画，并根据阶段进行动作过滤
  if (currentAction.value) {
    return getEffectiveAction(currentAction.value as PetStateKey, stage)
  }

  // 根据饥饿值调整（小于20显示饥饿状态）
  if (hunger < 20 && baseState.value === 'sitting') {
    return 'hungry' as PetStateKey
  }

  // // 根据生命阶段显示不同动画
  // if (stage === 'Baby') {
  //   // 幼体阶段使用更可爱的动画
  //   return baseState.value === 'sleeping' ? 'tilt-head' as PetStateKey : 'happy' as PetStateKey
  // }

  // if (stage === 'Elder') {
  //   // 老年阶段动作缓慢，使用坐姿或思考
  //   return baseState.value === 'sleeping' ? 'sleeping' : 'bored' as PetStateKey
  // }

  // 成年/助手模式：正常显示
  return baseState.value
})

export const currentGif = computed(() => {
  const stage = growthState.value?.stage
  return getStateMetaByStage(effectiveBaseState.value, stage).gifSrc
})

// 导出泡泡状态用于 UI 显示
const MAX_CHARS_PER_COLUMN = 20
const MAX_COLUMNS = 2
const MAX_CHARS = MAX_CHARS_PER_COLUMN * MAX_COLUMNS

export const truncatedSpeechText = computed(() => {
  if (!speechBubbleText.value) return ''
  if (speechBubbleText.value.length <= MAX_CHARS) return speechBubbleText.value
  return speechBubbleText.value.slice(0, MAX_CHARS) + '...'
})

export { showSpeechBubble, speechBubbleText }

const EVT_PET_ACTION = 'pet://action'

// 宠物动作事件类型
interface PetActionEvent {
  type: 'action' | 'say'
  action?: TempAction
  content?: string
}

// 在主窗口中监听宠物状态事件
export async function setupPetStateEventListeners(): Promise<() => void> {
  const unlistenProcessing = await listen(EVT_PET_PROCESSING, () => {
    clearActionAndQueue()
    playActionOnce('tilt-head', 'instruction')
  })

  // const unlistenSpeaking = await listen<{ text: string }>(EVT_PET_SPEAKING, (event) => {
  //   clearActionAndQueue()
  //   showSpeech(event.payload.text, 5000)
  //   playActionOnce('happy', 'instruction')
  // })

  // 监听来自工具的宠物控制指令
  const unlistenAction = await listen<PetActionEvent>(EVT_PET_ACTION, (event) => {
    clearActionAndQueue()

    if (event.payload.type === 'action' && event.payload.action) {
      playActionOnce(event.payload.action, 'instruction')
    } else if (event.payload.type === 'say' && event.payload.content) {
      showSpeech(event.payload.content, 5000)
      playActionOnce('happy', 'instruction')
    }
  })

  return () => {
    unlistenProcessing()
    // unlistenSpeaking()
    unlistenAction()
  }
}

// 显示泡泡文字
export function showSpeech(text: string, duration: number): void {
  if (speechTimer) {
    window.clearTimeout(speechTimer)
    speechTimer = null
  }
  speechBubbleText.value = text
  showSpeechBubble.value = true
  speechTimer = window.setTimeout(() => {
    showSpeechBubble.value = false
    speechTimer = null
  }, duration)
}

function clearCurrentAction(): void {
  if (actionTimer) {
    window.clearTimeout(actionTimer)
    actionTimer = null
  }
  if (currentAction.value === 'talking' && speechTimer) {
    window.clearTimeout(speechTimer)
    speechTimer = null
    showSpeechBubble.value = false
  }
  currentAction.value = null
  currentActionPriority.value = 0
  currentActionInterruptible.value = true
}

function enqueueQueuedAction(item: QueuedAction): void {
  if (
    currentAction.value &&
    item.priority > currentActionPriority.value &&
    currentActionInterruptible.value
  ) {
    clearCurrentAction()
  }

  actionQueue.value.push(item)
  actionQueue.value.sort((a, b) => {
    if (b.priority !== a.priority) {
      return b.priority - a.priority
    }
    return a.createdAt - b.createdAt
  })

  if (!currentAction.value) {
    processNextAction()
  }
}

function enqueueAction(
  action: TempAction,
  priority: ActionPriority,
  interruptible = priority !== 'instruction'
): void {
  const stage = growthState.value?.stage
  const effectiveAction = getEffectiveAction(action as PetStateKey, stage) as TempAction

  enqueueQueuedAction({
    name: effectiveAction,
    duration: getSinglePlayDuration(effectiveAction),
    priority: priorityValue[priority],
    interruptible,
    createdAt: Date.now()
  })
}

function playActionOnce(
  action: TempAction,
  priority: ActionPriority,
  interruptible = priority !== 'instruction'
): void {
  enqueueAction(action, priority, interruptible)
}

function processNextAction(): void {
  if (actionTimer) {
    window.clearTimeout(actionTimer)
    actionTimer = null
  }

  const next = actionQueue.value.shift()
  if (!next) {
    clearCurrentAction()
    return
  }

  currentAction.value = next.name
  currentActionPriority.value = next.priority
  currentActionInterruptible.value = next.interruptible
  if (next.name === 'talking') {
    showSpeech('该休息一下了', next.duration)
  }
  actionTimer = window.setTimeout(() => {
    clearCurrentAction()
    processNextAction()
  }, next.duration)
}

function wakeFromIdle(): void {
  console.log('[唤醒] sleeping -> sitting')
  baseState.value = 'sitting'
  boredTriggeredInCurrentIdle.value = false
  resetClickCounter()
}

function resetClickCounter(): void {
  clickCount.value = 0
  lastTriggeredMilestone.value = 0
  firstClickTime.value = 0
  lastClickTime.value = 0
}

function clearActionAndQueue(): void {
  clearCurrentAction()
  actionQueue.value = []
}

function tryTriggerCrazyMilestone(): boolean {
  if (clickCount.value >= 30) {
    console.log('[点击] 30次，触发 crazy 模式：running -> backing')
    clearActionAndQueue()
    playActionOnce('running', 'instruction')
    enqueueAction('backing', 'instruction')
    resetClickCounter()
    return true
  }

  if (clickCount.value >= 15 && lastTriggeredMilestone.value < 15) {
    console.log('[点击] 15次，触发 crazy-plus')
    clearActionAndQueue()
    playActionOnce('crazy-plus', 'instruction')
    lastTriggeredMilestone.value = 15
    return true
  }

  if (clickCount.value >= 10 && lastTriggeredMilestone.value < 10) {
    console.log('[点击] 10次，触发 crazy')
    clearActionAndQueue()
    playActionOnce('crazy', 'instruction')
    lastTriggeredMilestone.value = 10
    return true
  }

  return false
}

export function onPetClick(): void {
  const now = Date.now()
  const stateAtClick = baseState.value
  lastActivityTime.value = now
  boredTriggeredInCurrentIdle.value = false

  // 点击宠物增加亲密度
  if (isPetAlive()) {
    void growthOnPetClicked()
  }

  if (now - firstClickTime.value > CLICK_WINDOW_MS) {
    clickCount.value = 1
    lastTriggeredMilestone.value = 0
    firstClickTime.value = now
  } else {
    clickCount.value += 1
  }
  lastClickTime.value = now

  console.log(`[点击] 状态: ${stateAtClick}, 累计点击: ${clickCount.value}, 当前动作: ${currentAction.value ?? '无'}`)

  if (tryTriggerCrazyMilestone()) {
    return
  }

  if (stateAtClick === 'sitting') {
    // 第1次点击：保持 sitting，什么都不做
    if (clickCount.value === 1) {
      console.log('[点击] 第1次点击，保持 sitting 状态')
      return
    }

    if (clickCount.value === 5) {
      console.log('[点击] 触发 happy 动作')
      clearActionAndQueue()
      playActionOnce('happy', 'interaction')
      return
    }
    // 第2次及以后：tilt-head
    if (clickCount.value === 3) {
      console.log('[点击] 触发 tilt-head 动作')
      clearActionAndQueue()
      playActionOnce('tilt-head', 'interaction')
    }
  }
}

async function checkIdleState(): Promise<void> {
  if (checkingSystemIdle) {
    return
  }
  checkingSystemIdle = true

  const now = Date.now()
  let idleDuration = now - lastActivityTime.value

  // 尝试获取系统级别的空闲时间（Windows）
  try {
    const systemIdleMs = await invoke<number>('get_system_idle_ms')
    if (systemIdleMs >= 0) {
      idleDuration = systemIdleMs
    }
  } catch {
    // 系统空闲时间获取失败，使用前端的本地检测
  }

  if (currentAction.value === 'running') {
    checkingSystemIdle = false
    return
  }

  // 优先保证 sleeping/sitting 切换：
  // 有操作（<60s）就保持/恢复 sitting；无操作（>=60s）才进入 sleeping。
  if (baseState.value === 'sleeping' && idleDuration < SITTING_TO_SLEEPING_MS) {
    console.log(`[状态变更] 用户恢复活动，从 sleeping 唤醒 (空闲 ${Math.round(idleDuration / 1000)}s)`)
    wakeFromIdle()
    checkingSystemIdle = false
    return
  }

  if (baseState.value === 'sitting' && idleDuration >= SITTING_TO_SLEEPING_MS) {
    console.log(`[状态变更] 空闲 ${Math.round(idleDuration / 1000)}s，进入 sleeping`)
    const oldState = baseState.value
    baseState.value = 'sleeping'
    console.log(`[状态变更] ${oldState} -> sleeping`)
    checkingSystemIdle = false
    return
  }

  if (baseState.value === 'sitting' && idleDuration < SITTING_TO_BORED_MS) {
    boredTriggeredInCurrentIdle.value = false
  }

  // 连续使用提醒休息
  const timeSinceLastReminder = now - lastTalkingReminderTime.value
  if (timeSinceLastReminder >= TALKING_REMINDER_INTERVAL_MS) {
    console.log('[状态变更] 80秒提醒，触发 talking')
    enqueueAction('talking', 'interaction')
    lastTalkingReminderTime.value = now
    checkingSystemIdle = false
    return
  }

  // sitting 超过30秒触发 bored 状态
  if (baseState.value === 'sitting' && idleDuration >= SITTING_TO_BORED_MS) {
    if (boredTriggeredInCurrentIdle.value) {
      checkingSystemIdle = false
      return
    }
    console.log(`[状态变更] 空闲 ${Math.round(idleDuration / 1000)}s，触发 bored`)
    enqueueAction('bored', 'interaction')
    boredTriggeredInCurrentIdle.value = true
    checkingSystemIdle = false
    return
  }

  checkingSystemIdle = false
}

function startActivityListeners(): void {
  const activityEvents: Array<keyof WindowEventMap> = [
    'mousemove',
    'keydown',
    'wheel',
    'touchstart',
    'click'
  ]

  for (const event of activityEvents) {
    window.addEventListener(event, () => {
      lastActivityTime.value = Date.now()
      boredTriggeredInCurrentIdle.value = false
    }, { passive: true })
  }
}

function startIdleChecker(): void {
  idleChecker = window.setInterval(() => {
    void checkIdleState()
  }, IDLE_CHECK_INTERVAL_MS)
}

export function setupPetState(): () => void {
  if (started) {
    return teardownPetState
  }
  started = true

  console.log('[初始化] 宠物状态管理已启动，初始状态: sitting')

  // 加载成长状态（用于根据生命阶段显示不同动画）
  void loadGrowthState()

  startActivityListeners()
  startIdleChecker()

  return teardownPetState
}

function teardownPetState(): void {
  if (!started) {
    return
  }
  started = false

  if (idleChecker) {
    window.clearInterval(idleChecker)
    idleChecker = null
  }

  clearCurrentAction()
  if (speechTimer) {
    window.clearTimeout(speechTimer)
    speechTimer = null
  }
  showSpeechBubble.value = false

  actionQueue.value = []
  baseState.value = 'sitting'
  lastActivityTime.value = Date.now()
  lastTalkingReminderTime.value = Date.now()
  boredTriggeredInCurrentIdle.value = false
  checkingSystemIdle = false
  resetClickCounter()
}
