import { invoke } from '@tauri-apps/api/core'

export const SCALE_MIN = 0.1
export const SCALE_MAX = 1.0
export const SCALE_CHANGED_EVENT = 'm1://scale-changed'
export const CONFIG_CHANGED_EVENT = 'm6://config-changed'

export type AsrProvider = 'system'

export interface AppConfig {
  llm: {
    api_key: string
    model: string
    base_url: string
  }
  asr: {
    provider: AsrProvider
  }
  pet: {
    current: string
    scale: number
    name: string
    prompt: string
  }
  shortcuts: {
    push_to_talk: string
    open_chat: string
    feed_pet: string
  }
}

export interface OnlineAsrConfig {
  api_key: string
  model: string
  base_url: string
}

export function getDefaultConfig(): AppConfig {
  return {
    llm: {
      api_key: '',
      model: 'gpt-4o-mini',
      base_url: 'https://api.openai.com/v1'
    },
    asr: {
      provider: 'system'
    },
    pet: {
      current: 'dog',
      scale: 1,
      name: '小白',
      prompt: '你是一只可爱的桌面宠物，名字叫{name}。你的性格活泼、友好、有点调皮。请用简短、口语化的方式回复，不要太长。回复时要像宠物一样可爱，可以用一些语气词如"汪"、"呀"、"呢"等。'
    },
    shortcuts: {
      push_to_talk: 'Ctrl+Shift+Space',
      open_chat: 'Ctrl+Shift+C',
      feed_pet: 'Ctrl+Shift+F'
    }
  }
}

export function clampScale(scale: number): number {
  return Math.min(SCALE_MAX, Math.max(SCALE_MIN, scale))
}

export async function loadScale(): Promise<number> {
  const value = await invoke<number>('load_window_scale')
  return clampScale(value)
}

export async function saveScale(scale: number): Promise<void> {
  await invoke('save_window_scale', { scale: clampScale(scale) })
}

export async function hideWindow(): Promise<void> {
  await invoke('hide_main_window')
}

export async function showWindow(): Promise<void> {
  await invoke('show_main_window')
}

export async function openSettings(): Promise<void> {
  await invoke('open_settings')
}

export async function setMainWindowScale(scale: number): Promise<void> {
  await invoke('set_main_window_scale', { scale: clampScale(scale) })
}

export async function loadConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('load_config')
}

export async function saveConfig(config: AppConfig): Promise<AppConfig> {
  const defaultConfig = getDefaultConfig()
  const normalized = {
    ...config,
    pet: {
      ...config.pet,
      scale: clampScale(config.pet.scale),
      name: config.pet.name.trim() || defaultConfig.pet.name,
      prompt: config.pet.prompt.trim() || defaultConfig.pet.prompt
    }
  }
  return invoke<AppConfig>('save_config', { config: normalized })
}

export async function showPetContextMenu(x: number, y: number): Promise<void> {
  await invoke('show_pet_context_menu', { x, y })
}
