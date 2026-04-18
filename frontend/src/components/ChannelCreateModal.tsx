import { useState } from 'react'
import { channelsApi } from '../api/client'
import { useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'

interface Props {
  serverId: string
  onClose: () => void
}

export default function ChannelCreateModal({ serverId, onClose }: Props) {
  const [displayName, setDisplayName] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const queryClient = useQueryClient()
  const navigate = useNavigate()

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setLoading(true)
    try {
      const channel = await channelsApi.create(serverId, displayName.trim())
      queryClient.invalidateQueries({ queryKey: ['channels', serverId] })
      onClose()
      navigate(`/servers/${serverId}/channels/${channel.id}`)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create channel')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-surface-800 rounded-xl p-6 w-full max-w-sm shadow-2xl" onClick={(e) => e.stopPropagation()}>
        <h2 className="text-lg font-semibold text-white mb-4">Create a channel</h2>
        <form onSubmit={handleCreate} className="space-y-4">
          <div>
            <label className="block text-sm text-gray-300 mb-1">Channel name</label>
            <input
              autoFocus
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              required
              maxLength={80}
              className="w-full px-4 py-2.5 bg-surface-700 border border-surface-600 rounded-lg text-white focus:outline-none focus:border-accent-500"
              placeholder="general"
            />
          </div>
          {error && <p className="text-red-400 text-sm">{error}</p>}
          <div className="flex gap-3 justify-end">
            <button type="button" onClick={onClose} className="px-4 py-2 text-gray-400 hover:text-white text-sm">
              Cancel
            </button>
            <button
              type="submit"
              disabled={!displayName.trim() || loading}
              className="px-4 py-2 bg-accent-500 hover:bg-accent-400 disabled:opacity-50 text-white rounded-lg text-sm"
            >
              {loading ? 'Creating…' : 'Create'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
