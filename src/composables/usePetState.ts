import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'

export type BaseState = 'resting' | 'sitting'
export type TempAction = 'talking' | 'happy' | 'wag-tail' | 'tilt-head' | 'lying-down'
export type PetName = 'dog'

type ActionPriority = 'instruction' | 'interaction'

interface QueuedAction {
  name: TempAction
  duration: number
  priority: number
  createdAt: number
}

interface ActivityPayload {
  active?: boolean
}

const ACTIVITY_EVENT = 'activity-state-changed'
const IDLE_TIMEOUT_MS = 30_000
const IDLE_CHECK_INTERVAL_MS = 1_000

const PETS: Record<PetName, Record<BaseState | TempAction, string>> = {
  dog: {
    resting: new URL('../assets/pets/dog/resting.gif', import.meta.url).href,
    sitting: new URL('../assets/pets/dog/sitting.gif', import.meta.url).href,
    talking: new URL('../assets/pets/dog/talking.gif', import.meta.url).href,
    happy: new URL('../assets/pets/dog/happy.gif', import.meta.url).href,
    'wag-tail': new URL('../assets/pets/dog/wag-tail.gif', import.meta.url).href,
    'tilt-head': new URL('../assets/pets/dog/tilt-head.gif', import.meta.url).href,
    'lying-down': new URL('../assets/pets/dog/lying-down.gif', import.meta.url).href
  }
}

const priorityValue: Record<ActionPriority, number> = {
  instruction: 2,
  interaction: 1
}

const interactiveActions: TempAction[] = ['happy', 'wag-tail', 'tilt-head']

const currentPet = ref<PetName>('dog')
const baseState = ref<BaseState>('sitting')
const currentAction = ref<TempAction | null>(null)
const actionQueue = ref<QueuedAction[]>([])

let actionTimer: number | null = null
let idleChecker: number | null = null
let lastActivityTime = Date.now()
let unlistenActivity: null | (() => void) = null
let started = false

export const currentGif = computed(() => {
  const key = currentAction.value ?? baseState.value
  return PETS[currentPet.value][key]
})

export function enqueueInstructionAction(action: TempAction, duration = 2000): void {
  enqueueAction(action, duration, 'instruction')
}

export function onPetClick(): void {
  const index = Math.floor(Math.random() * interactiveActions.length)
  const selected = interactiveActions[index]
  enqueueAction(selected, 2000, 'interaction')
}

export function setupPetState(): () => void {
  if (started) {
    return teardownPetState
  }
  started = true

  startActivityListeners()
  startIdleChecker()
  void startBackendActivityMonitor()

  return teardownPetState
}

function enqueueAction(action: TempAction, duration: number, priority: ActionPriority): void {
  const item: QueuedAction = {
    name: action,
    duration,
    priority: priorityValue[priority],
    createdAt: Date.now()
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

function processNextAction(): void {
  if (actionTimer) {
    window.clearTimeout(actionTimer)
    actionTimer = null
  }

  const next = actionQueue.value.shift()
  if (!next) {
    currentAction.value = null
    return
  }

  currentAction.value = next.name
  actionTimer = window.setTimeout(() => {
    currentAction.value = null
    processNextAction()
  }, next.duration)
}

function setBaseStateFromActivity(active: boolean): void {
  baseState.value = active ? 'sitting' : 'resting'
  if (active) {
    lastActivityTime = Date.now()
  }
}

function reportUserActivity(): void {
  setBaseStateFromActivity(true)
}

function startActivityListeners(): void {
  const activityEvents: Array<keyof WindowEventMap> = [
    'mousemove',
    'mousedown',
    'keydown',
    'wheel',
    'touchstart'
  ]

  for (const event of activityEvents) {
    window.addEventListener(event, reportUserActivity, { passive: true })
  }

  void listen<ActivityPayload>(ACTIVITY_EVENT, (event) => {
    const payload = event.payload
    if (typeof payload?.active === 'boolean') {
      setBaseStateFromActivity(payload.active)
    }
  }).then((unlisten) => {
    unlistenActivity = unlisten
  })
}

function startIdleChecker(): void {
  idleChecker = window.setInterval(() => {
    if (Date.now() - lastActivityTime >= IDLE_TIMEOUT_MS) {
      baseState.value = 'resting'
    }
  }, IDLE_CHECK_INTERVAL_MS)
}

async function startBackendActivityMonitor(): Promise<void> {
  try {
    await invoke('start_activity_monitor')
  } catch {
    // M3 not ready: front-end idle checker keeps M2 behavior working.
  }
}

function teardownPetState(): void {
  if (!started) {
    return
  }
  started = false

  const activityEvents: Array<keyof WindowEventMap> = [
    'mousemove',
    'mousedown',
    'keydown',
    'wheel',
    'touchstart'
  ]
  for (const event of activityEvents) {
    window.removeEventListener(event, reportUserActivity)
  }

  if (unlistenActivity) {
    unlistenActivity()
    unlistenActivity = null
  }

  if (idleChecker) {
    window.clearInterval(idleChecker)
    idleChecker = null
  }
  if (actionTimer) {
    window.clearTimeout(actionTimer)
    actionTimer = null
  }
}
