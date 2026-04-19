// Runtime API configuration. For Tauri builds, set VITE_API_BASE_URL and
// VITE_WS_BASE_URL at build time. For web builds, relative paths are used.

const envApiBase = import.meta.env.VITE_API_BASE_URL as string | undefined
const envWsBase = import.meta.env.VITE_WS_BASE_URL as string | undefined

function isTauri(): boolean {
  // @ts-expect-error tauri global is injected by the Tauri runtime
  return typeof window !== 'undefined' && window.__TAURI__ !== undefined
}

export const API_BASE = envApiBase ?? (isTauri() ? 'http://localhost:3000/api' : '/api')

export function makeWsUrl(token: string): string {
  if (envWsBase) {
    return `${envWsBase}?token=${encodeURIComponent(token)}`
  }
  if (isTauri()) {
    return `ws://localhost:3000/ws?token=${encodeURIComponent(token)}`
  }
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${protocol}//${window.location.host}/ws?token=${encodeURIComponent(token)}`
}
