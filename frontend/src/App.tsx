import { useState, useEffect, useRef } from 'react'
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { useAuthStore } from './stores/authStore'
import LoginPage from './pages/LoginPage'
import RegisterPage from './pages/RegisterPage'
import InvitePage from './pages/InvitePage'
import ChatPage from './pages/ChatPage'

function RequireAuth({ children }: { children: React.ReactNode }) {
  const { user } = useAuthStore()
  if (!user) return <Navigate to="/login" replace />
  return <>{children}</>
}

function AppRouter() {
  const [bootstrapped, setBootstrapped] = useState(false)
  const bootstrap = useAuthStore((s) => s.bootstrap)
  const user = useAuthStore((s) => s.user)
  const ranRef = useRef(false)

  useEffect(() => {
    if (ranRef.current) return
    ranRef.current = true

    const run = async () => {
      if (!user) {
        await bootstrap()
      }
      setBootstrapped(true)
    }

    run()
  }, [bootstrap, user])

  if (!bootstrapped) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-surface-900">
        <div className="text-ivory/60 text-sm">Loading…</div>
      </div>
    )
  }

  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route path="/register" element={<RegisterPage />} />
      <Route path="/invite/:code" element={<InvitePage />} />
      <Route
        path="/*"
        element={
          <RequireAuth>
            <Routes>
              <Route index element={<ChatPage />} />
              <Route path="servers/:serverId" element={<ChatPage />} />
              <Route path="servers/:serverId/channels/:channelId" element={<ChatPage />} />
            </Routes>
          </RequireAuth>
        }
      />
    </Routes>
  )
}

export default function App() {
  return (
    <BrowserRouter>
      <AppRouter />
    </BrowserRouter>
  )
}
