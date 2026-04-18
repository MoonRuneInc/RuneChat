import { create } from 'zustand'
import { authApi, initApiClient, type User } from '../api/client'

interface AuthState {
  user: User | null
  accessToken: string | null
  setAuth: (user: User, token: string) => void
  clearAuth: () => void
  refresh: () => Promise<boolean>
  bootstrap: () => Promise<boolean>
}

function decodeJwtPayload(token: string): Partial<User> & { sub?: string } | null {
  try {
    const payload = token.split('.')[1]
    const json = atob(payload.replace(/-/g, '+').replace(/_/g, '/'))
    return JSON.parse(json)
  } catch {
    return null
  }
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  accessToken: null,

  setAuth: (user, accessToken) => set({ user, accessToken }),

  clearAuth: () => set({ user: null, accessToken: null }),

  refresh: async () => {
    try {
      const { access_token } = await authApi.refresh()
      set((state) => ({ accessToken: access_token, user: state.user }))
      return true
    } catch {
      set({ user: null, accessToken: null })
      return false
    }
  },

  bootstrap: async () => {
    try {
      const { access_token } = await authApi.refresh()
      const payload = decodeJwtPayload(access_token)
      if (!payload?.sub || !payload.username) {
        set({ user: null, accessToken: null })
        return false
      }
      const user: User = {
        id: payload.sub,
        username: payload.username,
        account_status: payload.account_status ?? 'active',
      }
      set({ user, accessToken: access_token })
      return true
    } catch {
      set({ user: null, accessToken: null })
      return false
    }
  },
}))

// Wire API client to auth store
initApiClient({
  getToken: () => useAuthStore.getState().accessToken,
  refresh: () => useAuthStore.getState().refresh(),
  clearAuth: () => useAuthStore.getState().clearAuth(),
})
