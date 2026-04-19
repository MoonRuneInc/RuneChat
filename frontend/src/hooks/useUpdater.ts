import { useEffect } from 'react'

function isTauri(): boolean {
  // @ts-expect-error tauri global is injected by the Tauri runtime
  return typeof window !== 'undefined' && window.__TAURI__ !== undefined
}

export function useUpdater() {
  useEffect(() => {
    if (!isTauri()) return
    let cancelled = false

    async function runUpdateCheck() {
      try {
        const { check } = await import('@tauri-apps/plugin-updater')
        const update = await check()
        if (cancelled || !update) return

        console.log(`Update available: ${update.version} — downloading...`)
        await update.downloadAndInstall((event) => {
          switch (event.event) {
            case 'Started':
              console.log(`Downloading update (${event.data.contentLength} bytes)`)
              break
            case 'Progress':
              console.log(`Downloaded ${event.data.chunkLength} bytes`)
              break
            case 'Finished':
              console.log('Download finished — restart to apply')
              break
          }
        })
      } catch (err) {
        // Silently ignore updater errors (e.g., offline, no endpoint, not in Tauri)
        console.debug('Updater check failed:', err)
      }
    }

    runUpdateCheck()
    return () => { cancelled = true }
  }, [])
}
