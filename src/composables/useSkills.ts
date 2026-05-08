import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

export interface Skill {
  name: string
  description: string
  author?: string
  version?: string
  installed_at: number
  enabled: boolean
  skill_path: string
  content: string
  has_resources: boolean
}

export interface SkillSearchResult {
  package: string
  name: string
  description: string
  author?: string
  version?: string
}

const skills = ref<Skill[]>([])
const loading = ref(false)
const searchResults = ref<SkillSearchResult[]>([])
const searchLoading = ref(false)
const searchError = ref('')

/**
 * 获取已安装的技能列表
 */
export async function listSkills(): Promise<Skill[]> {
  try {
    loading.value = true
    skills.value = await invoke<Skill[]>('skill_list')
    return skills.value
  } catch (err) {
    console.error('获取技能列表失败:', err)
    return []
  } finally {
    loading.value = false
  }
}

/**
 * 安装技能
 */
export async function installSkill(packageName: string): Promise<Skill> {
  try {
    loading.value = true
    const skill = await invoke<Skill>('skill_install', { package: packageName })
    await listSkills()
    return skill
  } catch (err) {
    console.error('安装技能失败:', err)
    throw err
  } finally {
    loading.value = false
  }
}

/**
 * 卸载技能
 */
export async function uninstallSkill(name: string): Promise<void> {
  try {
    loading.value = true
    await invoke('skill_uninstall', { name })
    await listSkills()
  } catch (err) {
    console.error('卸载技能失败:', err)
    throw err
  } finally {
    loading.value = false
  }
}

/**
 * 启用技能
 */
export async function enableSkill(name: string): Promise<void> {
  try {
    await invoke('skill_enable', { name })
    await listSkills()
  } catch (err) {
    console.error('启用技能失败:', err)
    throw err
  }
}

/**
 * 禁用技能
 */
export async function disableSkill(name: string): Promise<void> {
  try {
    await invoke('skill_disable', { name })
    await listSkills()
  } catch (err) {
    console.error('禁用技能失败:', err)
    throw err
  }
}

/**
 * 搜索可用技能
 */
export async function searchSkills(query: string): Promise<SkillSearchResult[]> {
  searchError.value = ''

  if (!query.trim()) {
    searchResults.value = []
    return []
  }

  try {
    searchLoading.value = true
    searchResults.value = await invoke<SkillSearchResult[]>('skill_search', { query })
    return searchResults.value
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err)
    searchError.value = msg
    console.error('搜索技能失败:', msg)
    return []
  } finally {
    searchLoading.value = false
  }
}

export function useSkills() {
  return {
    skills,
    loading,
    searchResults,
    searchLoading,
    searchError,
    listSkills,
    installSkill,
    uninstallSkill,
    enableSkill,
    disableSkill,
    searchSkills,
  }
}
