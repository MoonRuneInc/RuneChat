import { useEffect, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { invitesApi, type InvitePreview } from '../api/client'
import { useAuthStore } from '../stores/authStore'

export default function InvitePage() {
  const { code } = useParams<{ code: string }>()
  const navigate = useNavigate()
  const { user } = useAuthStore()
  const [preview, setPreview] = useState<InvitePreview | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [joining, setJoining] = useState(false)

  useEffect(() => {
    if (!code) return
    invitesApi
      .preview(code)
      .then(setPreview)
      .catch(() => setError('Invite not found or has expired.'))
  }, [code])

  const handleJoin = async () => {
    if (!user) {
      navigate(`/login?redirect=/invite/${code}`)
      return
    }
    if (!code) return
    setJoining(true)
    try {
      const { server_id } = await invitesApi.join(code)
      navigate(`/servers/${server_id}`)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to join')
    } finally {
      setJoining(false)
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-surface-900">
      <div className="w-full max-w-sm p-8 bg-surface-800 rounded-xl shadow-2xl text-center">
        {error ? (
          <>
            <p className="text-red-400 text-lg mb-4">{error}</p>
            <button
              onClick={() => navigate('/')}
              className="px-6 py-2 bg-surface-700 hover:bg-surface-600 text-white rounded-lg"
            >
              Go home
            </button>
          </>
        ) : preview ? (
          <>
            <div className="w-16 h-16 bg-accent-500 rounded-2xl flex items-center justify-center text-2xl font-bold text-white mx-auto mb-4">
              {preview.server_name[0]?.toUpperCase()}
            </div>
            <h1 className="text-xl font-bold text-white mb-1">{preview.server_name}</h1>
            <p className="text-gray-400 text-sm mb-6">
              {preview.member_count} {preview.member_count === 1 ? 'member' : 'members'}
            </p>
            <button
              onClick={handleJoin}
              disabled={joining}
              className="w-full py-2.5 bg-accent-500 hover:bg-accent-400 disabled:opacity-50 text-white font-medium rounded-lg transition-colors"
            >
              {joining ? 'Joining…' : user ? 'Join server' : 'Sign in to join'}
            </button>
          </>
        ) : (
          <p className="text-gray-400">Loading invite…</p>
        )}
      </div>
    </div>
  )
}
