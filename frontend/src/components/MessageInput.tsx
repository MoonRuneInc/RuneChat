import { useState, useRef } from 'react'
import { messagesApi } from '../api/client'

interface Props {
  channelId: string
  channelName: string
  disabled?: boolean
  disabledReason?: string
}

export default function MessageInput({ channelId, channelName, disabled, disabledReason }: Props) {
  const [content, setContent] = useState('')
  const [sending, setSending] = useState(false)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const handleSend = async () => {
    const trimmed = content.trim()
    if (!trimmed || sending || disabled) return

    setSending(true)
    try {
      await messagesApi.send(channelId, trimmed)
      setContent('')
      // Message will arrive via WebSocket and be appended by useMessages hook
    } catch (err) {
      // Show error inline if needed — for MVP just log
      console.error('send failed:', err)
    } finally {
      setSending(false)
      textareaRef.current?.focus()
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  if (disabled) {
    return (
      <div className="px-4 py-3 border-t border-surface-700">
        <div className="px-4 py-3 bg-surface-700/50 rounded-lg text-sm text-gray-500 text-center">
          {disabledReason ?? 'You cannot send messages here'}
        </div>
      </div>
    )
  }

  return (
    <div className="px-4 py-3 border-t border-surface-700">
      <div className="flex items-end gap-2 bg-surface-700 rounded-lg px-4 py-2">
        <textarea
          ref={textareaRef}
          value={content}
          onChange={(e) => setContent(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={`Message #${channelName}`}
          rows={1}
          maxLength={4000}
          className="flex-1 bg-transparent text-white placeholder-gray-500 resize-none focus:outline-none text-sm leading-6 max-h-32 overflow-y-auto"
          style={{ height: 'auto' }}
          onInput={(e) => {
            const el = e.currentTarget
            el.style.height = 'auto'
            el.style.height = `${Math.min(el.scrollHeight, 128)}px`
          }}
        />
        <button
          onClick={handleSend}
          disabled={!content.trim() || sending}
          className="shrink-0 p-1.5 text-accent-400 hover:text-accent-300 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
          title="Send message (Enter)"
        >
          <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
            <path d="M10.894 2.553a1 1 0 00-1.788 0l-7 14a1 1 0 001.169 1.409l5-1.429A1 1 0 009 15.571V11a1 1 0 112 0v4.571a1 1 0 00.725.962l5 1.428a1 1 0 001.17-1.408l-7-14z" />
          </svg>
        </button>
      </div>
      <p className="text-xs text-gray-600 mt-1 ml-1">Enter to send · Shift+Enter for new line</p>
    </div>
  )
}
