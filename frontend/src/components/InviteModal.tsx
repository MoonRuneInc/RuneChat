import { useState } from 'react'
import { invitesApi } from '../api/client'

interface Props {
  serverId: string
  onClose: () => void
}

export default function InviteModal({ serverId, onClose }: Props) {
  const [code, setCode] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [copied, setCopied] = useState(false)

  const handleGenerate = async () => {
    setError(null)
    setLoading(true)
    try {
      const invite = await invitesApi.create(serverId)
      setCode(invite.code)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create invite')
    } finally {
      setLoading(false)
    }
  }

  const inviteLink = code ? `${window.location.origin}/invite/${code}` : null

  const handleCopy = () => {
    if (!inviteLink) return
    navigator.clipboard.writeText(inviteLink)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-surface-800 rounded-xl p-6 w-full max-w-sm shadow-2xl" onClick={(e) => e.stopPropagation()}>
        <h2 className="text-lg font-semibold text-white mb-2">Invite people</h2>
        <p className="text-sm text-gray-400 mb-4">Share a link to let others join this server.</p>

        {!code ? (
          <>
            {error && <p className="text-red-400 text-sm mb-3">{error}</p>}
            <button
              onClick={handleGenerate}
              disabled={loading}
              className="w-full py-2.5 bg-accent-500 hover:bg-accent-400 disabled:opacity-50 text-white rounded-lg text-sm"
            >
              {loading ? 'Generating…' : 'Generate invite link'}
            </button>
          </>
        ) : (
          <div className="space-y-3">
            <div className="flex gap-2">
              <input
                readOnly
                value={inviteLink!}
                className="flex-1 px-3 py-2 bg-surface-700 text-gray-200 rounded-lg text-sm focus:outline-none"
              />
              <button
                onClick={handleCopy}
                className="px-3 py-2 bg-accent-500 hover:bg-accent-400 text-white rounded-lg text-sm shrink-0"
              >
                {copied ? 'Copied!' : 'Copy'}
              </button>
            </div>
            <p className="text-xs text-gray-500">Link expires never · Unlimited uses by default</p>
          </div>
        )}

        <button type="button" onClick={onClose} className="w-full mt-3 py-2 text-gray-400 hover:text-white text-sm">
          Close
        </button>
      </div>
    </div>
  )
}
