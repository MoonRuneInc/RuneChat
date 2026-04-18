import { useState } from 'react'
import { useNavigate, Link } from 'react-router-dom'
import { authApi } from '../api/client'
import { useAuthStore } from '../stores/authStore'

export default function LoginPage() {
  const [identifier, setIdentifier] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const { setAuth } = useAuthStore()
  const navigate = useNavigate()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setLoading(true)
    try {
      const { access_token, user } = await authApi.login(identifier, password)
      setAuth(user, access_token)
      navigate('/')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-surface-900">
      <div className="w-full max-w-md p-8 bg-surface-800 rounded-xl shadow-2xl">
        <h1 className="text-2xl font-bold text-white mb-2">Welcome back</h1>
        <p className="text-gray-400 text-sm mb-6">Sign in to RuneChat</p>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm text-gray-300 mb-1">Username or email</label>
            <input
              type="text"
              value={identifier}
              onChange={(e) => setIdentifier(e.target.value)}
              required
              className="w-full px-4 py-2.5 bg-surface-700 border border-surface-600 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-accent-500 transition-colors"
              placeholder="alice or alice@example.com"
            />
          </div>
          <div>
            <label className="block text-sm text-gray-300 mb-1">Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              className="w-full px-4 py-2.5 bg-surface-700 border border-surface-600 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-accent-500 transition-colors"
            />
          </div>

          {error && (
            <p className="text-red-400 text-sm">{error}</p>
          )}

          <button
            type="submit"
            disabled={loading}
            className="w-full py-2.5 bg-accent-500 hover:bg-accent-400 disabled:opacity-50 text-white font-medium rounded-lg transition-colors"
          >
            {loading ? 'Signing in…' : 'Sign in'}
          </button>
        </form>

        <p className="text-center text-sm text-gray-400 mt-6">
          No account?{' '}
          <Link to="/register" className="text-accent-400 hover:underline">
            Create one
          </Link>
        </p>
      </div>
    </div>
  )
}
