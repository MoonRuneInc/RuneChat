import { useState } from 'react'
import { useNavigate, Link, useSearchParams } from 'react-router-dom'
import { authApi } from '../api/client'
import { useAuthStore } from '../stores/authStore'

export default function LoginPage() {
  const [identifier, setIdentifier] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const { setAuth } = useAuthStore()
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()

  const redirect = searchParams.get('redirect')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setLoading(true)
    try {
      const { access_token, user } = await authApi.login(identifier, password)
      setAuth(user, access_token)
      navigate(redirect ?? '/')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen flex flex-col items-center justify-center bg-surface-900 gap-6">
      <img src="/placeholder.svg" alt="Cauldron" className="w-48 h-36" />
      <div className="w-full max-w-md p-8 bg-surface-800 rounded-xl shadow-2xl">
        <h1 className="text-2xl font-bold text-ivory mb-2">Welcome back</h1>
        <p className="text-ivory/60 text-sm mb-6">Sign in to Cauldron</p>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm text-ivory/80 mb-1">Username or email</label>
            <input
              type="text"
              value={identifier}
              onChange={(e) => setIdentifier(e.target.value)}
              required
              className="w-full px-4 py-2.5 bg-surface-700 border border-surface-600 rounded-lg text-ivory placeholder-ivory/50 focus:outline-none focus:border-accent-500 transition-colors"
              placeholder="alice or alice@example.com"
            />
          </div>
          <div>
            <label className="block text-sm text-ivory/80 mb-1">Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              className="w-full px-4 py-2.5 bg-surface-700 border border-surface-600 rounded-lg text-ivory placeholder-ivory/50 focus:outline-none focus:border-accent-500 transition-colors"
            />
          </div>

          {error && (
            <p className="text-red-400 text-sm">{error}</p>
          )}

          <button
            type="submit"
            disabled={loading}
            className="w-full py-2.5 bg-accent-500 hover:bg-accent-400 disabled:opacity-50 text-ivory font-medium rounded-lg transition-colors"
          >
            {loading ? 'Signing in…' : 'Sign in'}
          </button>
        </form>

        <p className="text-center text-sm text-ivory/60 mt-6">
          No account?{' '}
          <Link
            to={redirect ? `/register?redirect=${encodeURIComponent(redirect)}` : '/register'}
            className="text-accent-400 hover:underline"
          >
            Create one
          </Link>
        </p>
      </div>
    </div>
  )
}
