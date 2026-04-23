import { invoke } from '@tauri-apps/api/core'

export const SCALE_MIN = 0.1
export const SCALE_MAX = 1.0
export const SCALE_CHANGED_EVENT = 'm1://scale-changed'

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

export async function showPetContextMenu(x: number, y: number): Promise<void> {
  await invoke('show_pet_context_menu', { x, y })
}
