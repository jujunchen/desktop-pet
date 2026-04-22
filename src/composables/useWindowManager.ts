import { invoke } from '@tauri-apps/api/core'

const SCALE_MIN = 0.5
const SCALE_MAX = 3.0

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
