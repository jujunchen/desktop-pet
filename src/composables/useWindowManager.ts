import { invoke } from '@tauri-apps/api/core'

export const SCALE_MIN = 0.1
export const SCALE_MAX = 1.0
export const SCALE_CHANGED_EVENT = 'm1://scale-changed'
export const CONFIG_CHANGED_EVENT = 'm6://config-changed'

export type AsrProvider = 'whisper-local' | 'dashscope' | 'volcengine' | 'funasr'

export interface AppConfig {
  llm: {
    api_key: string
    model: string
    base_url: string
  }
  asr: {
    provider: AsrProvider
    whisper_local: {
      model_size: 'tiny' | 'base' | 'small' | 'medium'
    }
    dashscope: OnlineAsrConfig
    volcengine: OnlineAsrConfig
    funasr: OnlineAsrConfig
  }
  pet: {
    current: string
    scale: number
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
      provider: 'whisper-local',
      whisper_local: {
        model_size: 'tiny'
      },
      dashscope: {
        api_key: '',
        model: 'paraformer-v2',
        base_url: 'https://dashscope.aliyuncs.com/compatible-mode/v1/audio/transcriptions'
      },
      volcengine: {
        api_key: '',
        model: 'speech-paraformer',
        base_url: 'https://ark.cn-beijing.volces.com/api/v3/audio/transcriptions'
      },
      funasr: {
        api_key: '',
        model: 'paraformer',
        base_url: 'http://127.0.0.1:10095/transcriptions'
      }
    },
    pet: {
      current: 'dog',
      scale: 1
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
  const normalized = {
    ...config,
    pet: {
      ...config.pet,
      scale: clampScale(config.pet.scale)
    }
  }
  return invoke<AppConfig>('save_config', { config: normalized })
}

export async function showPetContextMenu(x: number, y: number): Promise<void> {
  await invoke('show_pet_context_menu', { x, y })
}
