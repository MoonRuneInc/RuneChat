import { create } from 'zustand'
import { authApi, initApiClient, type User } from '../api/client'

interface AuthState {
  user: User | null
  accessToken: string | null
  setAuth: (user: User, token: string) => void
  clearAuth: () => void
  refresh: () => Promise<boolean>
}

export const useAuthStore = create<AuthState>((set, _get) => ({
  user: null,
  accessToken: null,

  setAuth: (user, accessToken) => set({ user, accessToken }),

  clearAuth: () => set({ user: null, accessToken: null }),

  refresh: async () => {
    try {
      const { access_token } = await authApi.refresh()
      // Update only the token, not user (user info stays from current state)
      set((state) => ({ accessToken: access_token, user: state.user }))
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
