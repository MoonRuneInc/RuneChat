import { create } from 'zustand'

interface UiState {
  selectedServerId: string | null
  selectedChannelId: string | null
  selectServer: (id: string | null) => void
  selectChannel: (id: string | null) => void
}

export const useUiStore = create<UiState>((set) => ({
  selectedServerId: null,
  selectedChannelId: null,
  selectServer: (id) => set({ selectedServerId: id, selectedChannelId: null }),
  selectChannel: (id) => set({ selectedChannelId: id }),
}))
