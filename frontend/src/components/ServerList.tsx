import { useQuery } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'
import { serversApi, type Server } from '../api/client'
import { useUiStore } from '../stores/uiStore'

interface Props {
  onCreateServer: () => void
}

export default function ServerList({ onCreateServer }: Props) {
  const { data: servers = [] } = useQuery({
    queryKey: ['servers'],
    queryFn: serversApi.list,
  })
  const { selectedServerId, selectServer } = useUiStore()
  const navigate = useNavigate()

  const handleSelect = (server: Server) => {
    selectServer(server.id)
    navigate(`/servers/${server.id}`)
  }

  return (
    <div className="w-16 flex flex-col items-center py-3 gap-2 bg-surface-900 border-r border-surface-700 overflow-y-auto scrollbar-thin">
      {servers.map((s) => (
        <button
          key={s.id}
          onClick={() => handleSelect(s)}
          title={s.name}
          className={`w-10 h-10 rounded-xl flex items-center justify-center text-sm font-bold transition-all
            ${selectedServerId === s.id
              ? 'bg-accent-500 text-ivory rounded-2xl'
              : 'bg-surface-700 hover:bg-surface-600 text-ivory/80 hover:rounded-2xl'
            }`}
        >
          {s.name[0]?.toUpperCase()}
        </button>
      ))}

      <button
        onClick={onCreateServer}
        title="Create server"
        className="w-10 h-10 rounded-xl bg-surface-700 hover:bg-gold-600 text-ivory/60 hover:text-ivory text-xl font-light transition-all hover:rounded-2xl"
      >
        +
      </button>
    </div>
  )
}
