import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { messagesApi, type Message } from '../api/client'
import { useWebSocket } from './useWebSocket'

export function useMessages(channelId: string | null) {
  const queryClient = useQueryClient()

  const { data: messages = [], isLoading } = useQuery({
    queryKey: ['messages', channelId],
    queryFn: () => messagesApi.list(channelId!),
    enabled: !!channelId,
    staleTime: 0,
  })

  const handleWsMessage = useCallback(
    (data: unknown) => {
      const event = data as { type?: string; channel_id?: string; message?: Message }
      if (
        event.type === 'message.created' &&
        event.channel_id === channelId &&
        event.message
      ) {
        queryClient.setQueryData<Message[]>(['messages', channelId], (prev = []) => [
          ...prev,
          event.message!,
        ])
      }
    },
    [channelId, queryClient]
  )

  const { status } = useWebSocket(handleWsMessage)

  return { messages, isLoading, wsStatus: status }
}
