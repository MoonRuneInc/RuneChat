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

export default function App() {
  return (
    <BrowserRouter>
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
    </BrowserRouter>
  )
}
