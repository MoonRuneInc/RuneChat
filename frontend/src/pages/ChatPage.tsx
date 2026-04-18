import { useState, useEffect } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { serversApi, channelsApi } from '../api/client'
import { useAuthStore } from '../stores/authStore'
import { useMessages } from '../hooks/useMessages'
import ServerList from '../components/ServerList'
import ChannelList from '../components/ChannelList'
import MessageList from '../components/MessageList'
import MessageInput from '../components/MessageInput'
import CompromisedBanner from '../components/CompromisedBanner'
import ServerCreateModal from '../components/ServerCreateModal'
import InviteModal from '../components/InviteModal'
import ChannelCreateModal from '../components/ChannelCreateModal'

export default function ChatPage() {
  const { serverId, channelId } = useParams<{ serverId?: string; channelId?: string }>()
  const navigate = useNavigate()
  const { user } = useAuthStore()
  const [showCreateServer, setShowCreateServer] = useState(false)
  const [showInvite, setShowInvite] = useState(false)
  const [showCreateChannel, setShowCreateChannel] = useState(false)

  const { data: servers = [] } = useQuery({
    queryKey: ['servers'],
    queryFn: serversApi.list,
  })

  // If no server selected, redirect to first server
  useEffect(() => {
    if (!serverId && servers.length > 0) {
      navigate(`/servers/${servers[0].id}`, { replace: true })
    }
  }, [serverId, servers, navigate])

  // If server selected but no channel, redirect to first channel
  const { data: channels = [] } = useQuery({
    queryKey: ['channels', serverId],
    queryFn: () => channelsApi.list(serverId!),
    enabled: !!serverId,
  })

  useEffect(() => {
    if (serverId && !channelId && channels.length > 0) {
      navigate(`/servers/${serverId}/channels/${channels[0].id}`, { replace: true })
    }
  }, [serverId, channelId, channels, navigate])

  const currentServer = servers.find((s) => s.id === serverId)
  const currentChannel = channels.find((c) => c.id === channelId)

  const { messages, isLoading } = useMessages(channelId ?? null)

  const isCompromised = user?.account_status === 'compromised'

  if (servers.length === 0 && !showCreateServer) {
    return (
      <div className="h-screen flex bg-surface-900">
        <ServerList onCreateServer={() => setShowCreateServer(true)} />
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <p className="text-gray-400 mb-4">No servers yet. Create one to get started.</p>
            <button
              onClick={() => setShowCreateServer(true)}
              className="px-6 py-2.5 bg-accent-500 hover:bg-accent-400 text-white rounded-lg"
            >
              Create server
            </button>
          </div>
        </div>
        {showCreateServer && <ServerCreateModal onClose={() => setShowCreateServer(false)} />}
      </div>
    )
  }

  return (
    <div className="h-screen flex bg-surface-900 overflow-hidden">
      <ServerList onCreateServer={() => setShowCreateServer(true)} />

      {serverId && currentServer && (
        <ChannelList
          serverId={serverId}
          serverName={currentServer.name}
          onCreateChannel={() => setShowCreateChannel(true)}
        />
      )}

      {channelId && currentChannel ? (
        <div className="flex-1 flex flex-col min-w-0">
          {/* Channel header */}
          <div className="h-12 border-b border-surface-700 flex items-center px-4 gap-2 shrink-0">
            <span className="text-gray-500">#</span>
            <span className="font-semibold text-white">{currentChannel.display_name}</span>
            <div className="flex-1" />
            {serverId && (
              <button
                onClick={() => setShowInvite(true)}
                className="text-xs text-gray-400 hover:text-white px-2 py-1 rounded hover:bg-surface-700"
              >
                Invite
              </button>
            )}
          </div>

          {/* Compromised banner */}
          {isCompromised && (
            <div className="px-4 pt-3">
              <CompromisedBanner username={user!.username} />
            </div>
          )}

          <MessageList messages={messages} isLoading={isLoading} />

          <MessageInput
            channelId={channelId}
            channelName={currentChannel.display_name}
            disabled={isCompromised}
            disabledReason="Your account is locked. Unlock it to send messages."
          />
        </div>
      ) : (
        <div className="flex-1 flex items-center justify-center text-gray-500 text-sm">
          {channels.length === 0 ? 'No channels yet — create one!' : 'Select a channel'}
        </div>
      )}

      {showCreateServer && <ServerCreateModal onClose={() => setShowCreateServer(false)} />}
      {showCreateChannel && serverId && (
        <ChannelCreateModal serverId={serverId} onClose={() => setShowCreateChannel(false)} />
      )}
      {showInvite && serverId && (
        <InviteModal serverId={serverId} onClose={() => setShowInvite(false)} />
      )}
    </div>
  )
}
