import { useState } from 'react'
import { useNavigate, Link, useSearchParams } from 'react-router-dom'
import { authApi } from '../api/client'
import { useAuthStore } from '../stores/authStore'

export default function RegisterPage() {
  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const { setAuth } = useAuthStore()
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setLoading(true)
    try {
      const { access_token, user } = await authApi.register(username, email, password)
      setAuth(user, access_token)
      const redirect = searchParams.get('redirect')
      navigate(redirect ?? '/')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Registration failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen flex flex-col items-center justify-center bg-surface-900 gap-6">
      <img src="/placeholder.svg" alt="Cauldron" className="w-48 h-36" />
      <div className="w-full max-w-md p-8 bg-surface-800 rounded-xl shadow-2xl">
        <h1 className="text-2xl font-bold text-ivory mb-2">Create account</h1>
        <p className="text-ivory/60 text-sm mb-6">Join Cauldron</p>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm text-ivory/80 mb-1">Username</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
              minLength={2}
              maxLength={32}
              pattern="[a-zA-Z0-9_\-]+"
              title="2-32 characters: letters, numbers, underscores, hyphens"
              className="w-full px-4 py-2.5 bg-surface-700 border border-surface-600 rounded-lg text-ivory placeholder-ivory/50 focus:outline-none focus:border-accent-500 transition-colors"
              placeholder="your_username"
            />
            <p className="text-xs text-ivory/50 mt-1">2-32 chars: letters, numbers, _ and -</p>
          </div>
          <div>
            <label className="block text-sm text-ivory/80 mb-1">Email</label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              className="w-full px-4 py-2.5 bg-surface-700 border border-surface-600 rounded-lg text-ivory placeholder-ivory/50 focus:outline-none focus:border-accent-500 transition-colors"
              placeholder="alice@example.com"
            />
          </div>
          <div>
            <label className="block text-sm text-ivory/80 mb-1">Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              minLength={8}
              className="w-full px-4 py-2.5 bg-surface-700 border border-surface-600 rounded-lg text-ivory placeholder-ivory/50 focus:outline-none focus:border-accent-500 transition-colors"
            />
            <p className="text-xs text-ivory/50 mt-1">At least 8 characters</p>
          </div>

          {error && <p className="text-red-400 text-sm">{error}</p>}

          <button
            type="submit"
            disabled={loading}
            className="w-full py-2.5 bg-accent-500 hover:bg-accent-400 disabled:opacity-50 text-ivory font-medium rounded-lg transition-colors"
          >
            {loading ? 'Creating account…' : 'Create account'}
          </button>
        </form>

        <p className="text-center text-sm text-ivory/60 mt-6">
          Have an account?{' '}
          <Link to="/login" className="text-accent-400 hover:underline">
            Sign in
          </Link>
        </p>
      </div>
    </div>
  )
}
