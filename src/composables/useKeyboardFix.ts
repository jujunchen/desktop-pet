/**
 * 修复 macOS 上 Tauri 窗口的复制粘贴快捷键问题
 * 在 macOS 上，当应用不在 Dock 中时，系统快捷键可能失效
 * 此模块通过前端手动监听键盘事件来实现这些功能
 */

export function useKeyboardFix() {
  const handleKeydown = (e: KeyboardEvent) => {
    // 只在 macOS 上处理
    if (!navigator.userAgent.includes('Mac')) {
      return
    }

    const isCmd = e.metaKey || e.key === 'Meta'
    if (!isCmd) return

    const target = e.target as HTMLElement
    const isInput =
      target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.isContentEditable

    // 只在输入框中处理
    if (!isInput) return

    switch (e.key.toLowerCase()) {
      case 'c':
        e.preventDefault()
        document.execCommand('copy')
        break
      case 'v':
        e.preventDefault()
        document.execCommand('paste')
        break
      case 'x':
        e.preventDefault()
        document.execCommand('cut')
        break
      case 'a':
        e.preventDefault()
        document.execCommand('selectAll')
        break
      case 'z':
        e.preventDefault()
        if (e.shiftKey) {
          document.execCommand('redo')
        } else {
          document.execCommand('undo')
        }
        break
    }
  }

  const enable = () => {
    document.addEventListener('keydown', handleKeydown, true)
  }

  const disable = () => {
    document.removeEventListener('keydown', handleKeydown, true)
  }

  return { enable, disable }
}

// 自动启用（在设置和聊天窗口中）
const params = new URLSearchParams(window.location.search)
const entry = params.get('window')
if (entry === 'settings' || entry === 'chat') {
  const { enable } = useKeyboardFix()
  enable()
}
