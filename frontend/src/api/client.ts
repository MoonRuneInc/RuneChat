import { API_BASE } from '../config'

const BASE = API_BASE

export interface User {
  id: string
  username: string
  account_status: string
}

// Auth store reference — injected at runtime to avoid circular imports
let getToken: () => string | null = () => null
let refreshFn: () => Promise<boolean> = async () => false
let clearAuth: () => void = () => {}

export function initApiClient(deps: {
  getToken: () => string | null
  refresh: () => Promise<boolean>
  clearAuth: () => void
}) {
  getToken = deps.getToken
  refreshFn = deps.refresh
  clearAuth = deps.clearAuth
}

async function request<T>(
  path: string,
  options: RequestInit = {},
  retry = true
): Promise<T> {
  const token = getToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string> ?? {}),
  }
  if (token) headers['Authorization'] = `Bearer ${token}`

  const res = await fetch(`${BASE}${path}`, { ...options, headers, credentials: 'include' })

  if (res.status === 401 && retry) {
    const refreshed = await refreshFn()
    if (refreshed) {
      return request<T>(path, options, false)
    }
    clearAuth()
    throw new Error('Unauthorized')
  }

  if (!res.ok) {
    const body = await res.json().catch(() => ({}))
    throw new Error(body.error ?? `HTTP ${res.status}`)
  }

  if (res.status === 204) return undefined as T
  return res.json()
}

// --- Auth ---
export const authApi = {
  register: (username: string, email: string, password: string) =>
    request<{ access_token: string; user: User }>('/auth/register', {
      method: 'POST',
      body: JSON.stringify({ username, email, password }),
    }),
  login: (identifier: string, password: string) =>
    request<{ access_token: string; user: User }>('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ identifier, password }),
    }),
  refresh: () =>
    request<{ access_token: string }>('/auth/refresh', { method: 'POST' }, false),
  logout: () => request<void>('/auth/logout', { method: 'POST' }),
}

// --- Servers ---
export interface Server {
  id: string
  name: string
  owner_id: string
  member_count: number
  my_role: string
}

export const serversApi = {
  list: () => request<Server[]>('/servers'),
  create: (name: string) =>
    request<Server>('/servers', { method: 'POST', body: JSON.stringify({ name }) }),
}

// --- Channels ---
export interface Channel {
  id: string
  server_id: string
  display_name: string
  slug: string
  created_at: string
}

export const channelsApi = {
  list: (serverId: string) => request<Channel[]>(`/servers/${serverId}/channels`),
  create: (serverId: string, displayName: string) =>
    request<Channel>(`/servers/${serverId}/channels`, {
      method: 'POST',
      body: JSON.stringify({ display_name: displayName }),
    }),
}

// --- Messages ---
export interface Message {
  id: string
  channel_id: string
  author_id: string
  author_username: string
  author_status: string
  content: string
  compromised_at_send: boolean
  created_at: string
}

export const messagesApi = {
  list: (channelId: string, before?: string) =>
    request<Message[]>(
      `/channels/${channelId}/messages${before ? `?before=${before}` : ''}`
    ),
  send: (channelId: string, content: string) =>
    request<Message>(`/channels/${channelId}/messages`, {
      method: 'POST',
      body: JSON.stringify({ content }),
    }),
}

// --- Invites ---
export interface InvitePreview {
  server_name: string
  member_count: number
  valid: boolean
}

export const invitesApi = {
  preview: (code: string) => request<InvitePreview>(`/invite/${code}`),
  join: (code: string) =>
    request<{ server_id: string; server_name: string }>(`/invite/${code}/join`, {
      method: 'POST',
    }),
  create: (serverId: string, maxUses?: number, expiresInHours?: number) =>
    request<{ id: string; code: string }>('/invite', {
      method: 'POST',
      body: JSON.stringify({ server_id: serverId, max_uses: maxUses, expires_in_hours: expiresInHours }),
    }),
}
