import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'

export type BaseState = 'sitting' | 'sleeping'
export type TempAction =
  | 'talking'
  | 'happy'
  | 'tilt-head'
  | 'happy'
  | 'crazy'
  | 'crazy-plus'
  | 'running'
  | 'backing'
  | 'bored'
export type PetName = 'dog'

type ActionPriority = 'instruction' | 'interaction' | 'idle-transition'

interface QueuedAction {
  name: TempAction
  duration: number
  priority: number
  loopCount: number
  interruptible: boolean
  createdAt: number
}

const CLICK_WINDOW_MS = 3_000
const SITTING_TO_SLEEPING_MS = 60_000 // 60秒无操作进入睡眠
const SITTING_TO_BORED_MS = 30_000 // 30分钟进入无聊状态
const TALKING_REMINDER_INTERVAL_MS = 80_000 // 1小时连续使用提醒休息
const IDLE_CHECK_INTERVAL_MS = 1_000

const PETS: Record<PetName, Record<string, string>> = {
  dog: {
    // base states
    sitting: new URL('../assets/pets/dog/sitting.gif', import.meta.url).href,
    sleeping: new URL('../assets/pets/dog/sleeping.gif', import.meta.url).href,
    // interaction actions
    talking: new URL('../assets/pets/dog/talking.gif', import.meta.url).href,
    'happy': new URL('../assets/pets/dog/happy.gif', import.meta.url).href,
    'tilt-head': new URL('../assets/pets/dog/tilt-head.gif', import.meta.url).href,
    bored: new URL('../assets/pets/dog/bored.gif', import.meta.url).href,
    backing: new URL('../assets/pets/dog/backing.gif', import.meta.url).href,
    // excited states
    crazy: new URL('../assets/pets/dog/crazy.gif', import.meta.url).href,
    'crazy-plus': new URL('../assets/pets/dog/crazy-plus.gif', import.meta.url).href,
    running: new URL('../assets/pets/dog/running.gif', import.meta.url).href,
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
const firstClickTime = ref(0)
const lastClickTime = ref(0)
const lastActivityTime = ref(Date.now())
const lastTalkingReminderTime = ref(Date.now())
// 泡泡文字
const showSpeechBubble = ref(false)
const speechBubbleText = ref('')
let checkingSystemIdle = false

let actionTimer: number | null = null
let idleChecker: number | null = null
let started = false

export const currentGif = computed(() => {
  const key = currentAction.value ?? baseState.value
  return PETS[currentPet.value][key]
})

// 导出泡泡状态用于 UI 显示
export { showSpeechBubble, speechBubbleText }

// 显示泡泡文字
function showSpeech(text: string, duration: number): void {
  speechBubbleText.value = text
  showSpeechBubble.value = true
  window.setTimeout(() => {
    showSpeechBubble.value = false
  }, duration)
}

function enqueueQueuedAction(item: QueuedAction): void {
  if (
    currentAction.value &&
    item.priority > currentActionPriority.value &&
    currentActionInterruptible.value
  ) {
    if (actionTimer) {
      window.clearTimeout(actionTimer)
      actionTimer = null
    }
    currentAction.value = null
    currentActionPriority.value = 0
    currentActionInterruptible.value = true
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

function enqueueAction(action: TempAction, duration: number, priority: ActionPriority): void {
  enqueueQueuedAction({
    name: action,
    duration,
    priority: priorityValue[priority],
    loopCount: 1,
    interruptible: priority !== 'instruction',
    createdAt: Date.now()
  })
}

function enqueueLoopAction(
  action: TempAction,
  duration: number,
  loopCount: number,
  priority: ActionPriority,
  interruptible: boolean
): void {
  const item: QueuedAction = {
    name: action,
    duration: duration * loopCount,
    priority: priorityValue[priority],
    loopCount,
    interruptible,
    createdAt: Date.now()
  }
  enqueueQueuedAction(item)
}

function processNextAction(): void {
  if (actionTimer) {
    window.clearTimeout(actionTimer)
    actionTimer = null
  }

  const next = actionQueue.value.shift()
  if (!next) {
    currentAction.value = null
    currentActionPriority.value = 0
    currentActionInterruptible.value = true
    return
  }

  currentAction.value = next.name
  currentActionPriority.value = next.priority
  currentActionInterruptible.value = next.interruptible
  actionTimer = window.setTimeout(() => {
    currentAction.value = null
    currentActionPriority.value = 0
    currentActionInterruptible.value = true
    processNextAction()
  }, next.duration)
}

function wakeFromIdle(): void {
  console.log('[唤醒] sleeping -> sitting')
  baseState.value = 'sitting'
  resetClickCounter()
}

function resetClickCounter(): void {
  clickCount.value = 0
  firstClickTime.value = 0
  lastClickTime.value = 0
}

export function onPetClick(): void {
  const now = Date.now()
  const stateAtClick = baseState.value
  lastActivityTime.value = now

  if (now - firstClickTime.value > CLICK_WINDOW_MS) {
    clickCount.value = 1
    firstClickTime.value = now
  } else {
    clickCount.value += 1
  }
  lastClickTime.value = now

  console.log(`[点击] 状态: ${stateAtClick}, 累计点击: ${clickCount.value}, 当前动作: ${currentAction.value ?? '无'}`)

  if (stateAtClick === 'sleeping') {
    handleSleepingClick()
    return
  }

  if (stateAtClick === 'sitting') {
    // 第1次点击：保持 sitting，什么都不做
    if (clickCount.value === 1) {
      console.log('[点击] 第1次点击，保持 sitting 状态')
      return
    }

    // 立即清除所有正在播放的动作和队列，保证点击立即响应
    if (actionTimer) {
      window.clearTimeout(actionTimer)
      actionTimer = null
    }
    actionQueue.value = []
    currentAction.value = null
    currentActionPriority.value = 0
    currentActionInterruptible.value = true

    if (clickCount.value >= 5) {
      console.log('[点击] 触发 happy 动作')
      currentAction.value = 'happy'
      actionTimer = window.setTimeout(() => {
        console.log('[动作结束] happy')
        currentAction.value = null
        processNextAction()
      }, 3000)
      resetClickCounter()
      return
    }
    // 第2次及以后：tilt-head
    if (clickCount.value == 3) {
      console.log('[点击] 触发 tilt-head 动作')
      currentAction.value = 'tilt-head'
      actionTimer = window.setTimeout(() => {
        console.log('[动作结束] tilt-head')
        currentAction.value = null
        processNextAction()
      }, 3000)
    }
  }
}

function handleSleepingClick(): void {
  if (clickCount.value >= 50) {
    console.log('[睡眠中点击] 50次，触发 crazy 模式：backing -> running')
    baseState.value = 'sitting'
    resetClickCounter()
    currentAction.value = 'backing'
    actionTimer = window.setTimeout(() => {
      currentAction.value = 'running'
      actionTimer = window.setTimeout(() => {
        console.log('[动作结束] running')
        currentAction.value = null
        processNextAction()
      }, 180000)
    }, 1600)
    return
  }
  if (clickCount.value >= 15) {
    console.log('[睡眠中点击] 15次，触发 crazy-plus')
    currentAction.value = 'crazy-plus'
    actionTimer = window.setTimeout(() => {
      console.log('[动作结束] crazy-plus')
      currentAction.value = null
      processNextAction()
    }, 2600)
    return
  }
  if (clickCount.value >= 5) {
    console.log('[睡眠中点击] 5次，触发 crazy')
    currentAction.value = 'crazy'
    actionTimer = window.setTimeout(() => {
      console.log('[动作结束] crazy')
      currentAction.value = null
      processNextAction()
    }, 3600)
    return
  }
  wakeFromIdle()
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

  // 1小时连续使用提醒休息
  const timeSinceLastReminder = now - lastTalkingReminderTime.value
  if (timeSinceLastReminder >= TALKING_REMINDER_INTERVAL_MS) {
    console.log('[状态变更] 1小时提醒，触发 talking')
    currentAction.value = 'talking'
    showSpeech('该休息一下了', 10000)  // 泡泡显示10秒
    actionTimer = window.setTimeout(() => {
      console.log('[动作结束] talking')
      currentAction.value = null
      processNextAction()
    }, 10000)
    lastTalkingReminderTime.value = now
    checkingSystemIdle = false
    return
  }

  // sitting 超过30分钟触发 bored 状态
  if (baseState.value === 'sitting' && idleDuration >= SITTING_TO_BORED_MS) {
    console.log(`[状态变更] 空闲 ${Math.round(idleDuration / 1000)}s，触发 bored`)
    currentAction.value = 'bored'
    actionTimer = window.setTimeout(() => {
      console.log('[动作结束] bored')
      currentAction.value = null
      processNextAction()
    }, 3000)
    checkingSystemIdle = false
    return
  }

  // sitting -> sleeping（60秒无操作）
  if (baseState.value === 'sitting' && idleDuration >= SITTING_TO_SLEEPING_MS) {
    console.log(`[状态变更] 空闲 ${Math.round(idleDuration / 1000)}s，进入 sleeping`)
    const oldState = baseState.value
    baseState.value = 'sleeping'
    console.log(`[状态变更] ${oldState} -> sleeping`)
    checkingSystemIdle = false
    return
  }

  // sleeping 状态下，用户恢复活动
  if (baseState.value === 'sleeping' && idleDuration < SITTING_TO_SLEEPING_MS) {
    console.log(`[状态变更] 用户恢复活动，从 sleeping 唤醒 (空闲 ${Math.round(idleDuration / 1000)}s)`)
    wakeFromIdle()
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
  console.log('[配置] 进入睡眠: 60s, bored: 30分钟, 休息提醒: 1小时')

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
  if (actionTimer) {
    window.clearTimeout(actionTimer)
    actionTimer = null
  }

  actionQueue.value = []
  currentAction.value = null
  currentActionPriority.value = 0
  currentActionInterruptible.value = true
  baseState.value = 'sitting'
  lastActivityTime.value = Date.now()
  lastTalkingReminderTime.value = Date.now()
  checkingSystemIdle = false
  resetClickCounter()
}
