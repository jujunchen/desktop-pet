import { invoke } from '@tauri-apps/api/core'

export interface MemoryStats {
  short_term_count: number
  medium_term_count: number
  long_term_count: number
  total_count: number
}

/**
 * 保存单条记忆
 */
export async function addMemory(content: string, memoryType: 'chat' | 'fact' | 'interaction' = 'chat'): Promise<string> {
  try {
    return await invoke<string>('add_memory', { content, memoryType })
  } catch (err) {
    console.error('保存记忆失败:', err)
    throw err
  }
}

/**
 * 保存对话对（用户消息 + 宠物回复）
 */
export async function addChatMemory(userMessage: string, petResponse: string): Promise<[string, string]> {
  try {
    return await invoke<[string, string]>('add_chat_memory', { userMessage, petResponse })
  } catch (err) {
    console.error('保存对话记忆失败:', err)
    throw err
  }
}

/**
 * 批量保存聊天记录
 */
export async function saveChatHistory(messages: Array<{ role: 'user' | 'pet', text: string }>): Promise<void> {
  try {
    // 跳过空消息和欢迎消息
    const filtered = messages.filter(m => m.text.trim())
    if (filtered.length === 0) return

    console.log(`[Memory] 保存 ${filtered.length} 条聊天记录`)

    // 批量保存
    for (const msg of filtered) {
      const content = msg.role === 'user'
        ? `主人说：${msg.text}`
        : `我回答：${msg.text}`
      await addMemory(content, 'chat')
    }

    console.log('[Memory] 聊天记录保存完成')
  } catch (err) {
    console.error('批量保存聊天记录失败:', err)
  }
}

/**
 * 检索相关记忆
 */
export async function searchMemories(query: string, limit: number = 5): Promise<string[]> {
  try {
    return await invoke<string[]>('search_memories', { query, limit })
  } catch (err) {
    console.error('检索记忆失败:', err)
    return []
  }
}

/**
 * 获取记忆统计信息
 */
export async function getMemoryStats(): Promise<MemoryStats | null> {
  try {
    return await invoke<MemoryStats>('get_memory_stats')
  } catch (err) {
    console.error('获取记忆统计失败:', err)
    return null
  }
}

/**
 * 构建给 LLM 的记忆提示词
 */
export async function buildMemoryPrompt(query: string, limit: number = 5): Promise<string> {
  try {
    return await invoke<string>('build_memory_prompt', { query, limit })
  } catch (err) {
    console.error('构建记忆提示词失败:', err)
    return ''
  }
}
