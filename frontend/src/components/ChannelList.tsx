import { useQuery } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'
import { channelsApi, type Channel } from '../api/client'
import { useUiStore } from '../stores/uiStore'

interface Props {
  serverId: string
  serverName: string
  onCreateChannel: () => void
}

export default function ChannelList({ serverId, serverName, onCreateChannel }: Props) {
  const { data: channels = [] } = useQuery({
    queryKey: ['channels', serverId],
    queryFn: () => channelsApi.list(serverId),
    enabled: !!serverId,
  })
  const { selectedChannelId, selectChannel } = useUiStore()
  const navigate = useNavigate()

  const handleSelect = (channel: Channel) => {
    selectChannel(channel.id)
    navigate(`/servers/${serverId}/channels/${channel.id}`)
  }

  return (
    <div className="w-56 flex flex-col bg-surface-800 border-r border-surface-700">
      <div className="px-4 py-3 border-b border-surface-700 flex items-center justify-between">
        <span className="font-semibold text-white text-sm truncate">{serverName}</span>
      </div>

      <div className="flex-1 overflow-y-auto scrollbar-thin py-2">
        <div className="px-3 mb-1 flex items-center justify-between">
          <span className="text-xs font-semibold uppercase text-gray-500 tracking-wide">
            Channels
          </span>
          <button
            onClick={onCreateChannel}
            title="Create channel"
            className="text-gray-500 hover:text-white text-lg leading-none"
          >
            +
          </button>
        </div>
        {channels.map((c) => (
          <button
            key={c.id}
            onClick={() => handleSelect(c)}
            className={`w-full text-left px-3 py-1.5 rounded-md mx-1 flex items-center gap-1.5 text-sm transition-colors
              ${selectedChannelId === c.id
                ? 'bg-surface-600 text-white'
                : 'text-gray-400 hover:text-gray-200 hover:bg-surface-700'
              }`}
          >
            <span className="text-gray-500">#</span>
            <span className="truncate">{c.display_name}</span>
          </button>
        ))}
      </div>
    </div>
  )
}
