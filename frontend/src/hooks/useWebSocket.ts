import { useEffect, useRef, useState } from 'react'
import { useAuthStore } from '../stores/authStore'
import { makeWsUrl } from '../config'

export type WsStatus = 'connecting' | 'open' | 'closed'

export function useWebSocket(onMessage: (data: unknown) => void) {
  const wsRef = useRef<WebSocket | null>(null)
  const attemptRef = useRef(0)
  const [status, setStatus] = useState<WsStatus>('closed')
  const onMessageRef = useRef(onMessage)

  useEffect(() => {
    onMessageRef.current = onMessage
  })

  useEffect(() => {
    const token = useAuthStore.getState().accessToken
    if (!token) return

    let cancelled = false

    const connect = () => {
      if (cancelled) return
      setStatus('connecting')
      const ws = new WebSocket(makeWsUrl(token))
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
        if (cancelled) return
        setStatus('closed')
        const attempt = attemptRef.current
        const delay = Math.min(1000 * Math.pow(2, attempt), 30_000) + Math.random() * 500
        attemptRef.current++
        if (attempt < 10) {
          setTimeout(connect, delay)
        }
      }

      ws.onerror = () => {
        ws.close()
      }
    }

    connect()

    return () => {
      cancelled = true
      attemptRef.current = 100
      wsRef.current?.close()
    }
  }, [])

  return { status }
}
