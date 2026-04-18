import { useRef, useEffect } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { type Message } from '../api/client'
import { CompromisedBadge } from './CompromisedBanner'

interface Props {
  messages: Message[]
  isLoading: boolean
}

function MessageItem({ message }: { message: Message }) {
  const ts = new Date(message.created_at).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
  })

  return (
    <div className="flex gap-3 px-4 py-1.5 hover:bg-surface-700/30 rounded group">
      <div className="w-8 h-8 rounded-full bg-accent-500/20 flex items-center justify-center text-xs font-bold text-accent-400 shrink-0 mt-0.5">
        {message.author_username[0]?.toUpperCase()}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-baseline gap-2 flex-wrap">
          <span className="font-medium text-white text-sm">
            {message.author_username}
          </span>
          {message.author_status === 'compromised' && <CompromisedBadge />}
          <span className="text-xs text-gray-500">{ts}</span>
        </div>
        <p className={`text-sm leading-relaxed text-gray-200 break-words ${
          message.compromised_at_send ? 'opacity-60 italic' : ''
        }`}>
          {message.compromised_at_send && (
            <span className="text-amber-500 text-xs mr-1">[sent while compromised]</span>
          )}
          {message.content}
        </p>
      </div>
    </div>
  )
}

export default function MessageList({ messages, isLoading }: Props) {
  const parentRef = useRef<HTMLDivElement>(null)
  const atBottomRef = useRef(true)

  const virtualizer = useVirtualizer({
    count: messages.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 60,
    overscan: 10,
  })

  // Auto-scroll to bottom on new messages if user was at bottom
  useEffect(() => {
    if (!atBottomRef.current) return
    if (messages.length === 0) return
    virtualizer.scrollToIndex(messages.length - 1, { align: 'end' })
  }, [messages.length, virtualizer])

  const handleScroll = () => {
    const el = parentRef.current
    if (!el) return
    atBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 100
  }

  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center text-gray-500 text-sm">
        Loading messages…
      </div>
    )
  }

  if (messages.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center text-gray-500 text-sm">
        No messages yet. Say hello!
      </div>
    )
  }

  const items = virtualizer.getVirtualItems()

  return (
    <div
      ref={parentRef}
      onScroll={handleScroll}
      className="flex-1 overflow-y-auto scrollbar-thin py-2"
    >
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          position: 'relative',
        }}
      >
        <div
          style={{
            position: 'absolute',
            top: 0,
            left: 0,
            right: 0,
            transform: `translateY(${items[0]?.start ?? 0}px)`,
          }}
        >
          {items.map((item) => (
            <div key={item.key} data-index={item.index} ref={virtualizer.measureElement}>
              <MessageItem message={messages[item.index]} />
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
