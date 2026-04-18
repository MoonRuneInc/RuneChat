import { useEffect, useRef, useCallback, useState } from 'react'
import { useAuthStore } from '../stores/authStore'

export type WsStatus = 'connecting' | 'open' | 'closed'

export function useWebSocket(onMessage: (data: unknown) => void) {
  const wsRef = useRef<WebSocket | null>(null)
  const attemptRef = useRef(0)
  const [status, setStatus] = useState<WsStatus>('closed')
  const onMessageRef = useRef(onMessage)
  onMessageRef.current = onMessage

  const connect = useCallback(() => {
    const token = useAuthStore.getState().accessToken
    if (!token) return

    setStatus('connecting')
    const ws = new WebSocket(`/ws?token=${encodeURIComponent(token)}`)
    wsRef.current = ws

    ws.onopen = () => {
      attemptRef.current = 0
      setStatus('open')
    }

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)
        onMessageRef.current(data)
      } catch {
        // ignore malformed messages
      }
    }

    ws.onclose = () => {
      setStatus('closed')
      const attempt = attemptRef.current
      // Exponential backoff: 1s, 2s, 4s, 8s, ... up to 30s
      const delay = Math.min(1000 * Math.pow(2, attempt), 30_000) + Math.random() * 500
      attemptRef.current++
      if (attempt < 10) {
        setTimeout(connect, delay)
      }
    }

    ws.onerror = () => {
      ws.close()
    }
  }, [])

  useEffect(() => {
    connect()
    return () => {
      attemptRef.current = 100 // prevent reconnect on unmount
      wsRef.current?.close()
    }
  }, [connect])

  return { status }
}
