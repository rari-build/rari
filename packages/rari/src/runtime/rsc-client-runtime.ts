/* eslint-disable node/prefer-global/process */
import type { GlobalWithRari, WindowWithRari } from './shared/types'
import { installRscChunkLoader, requireClientComponent } from './shared/get-client-component'

if (typeof (globalThis as unknown as GlobalWithRari)['~rari'] === 'undefined')
  (globalThis as unknown as GlobalWithRari)['~rari'] = {}

;(globalThis as unknown as GlobalWithRari)['~rari'].isDevelopment = process.env.NODE_ENV !== 'production'

if (typeof (globalThis as unknown as GlobalWithRari)['~clientComponents'] === 'undefined')
  (globalThis as unknown as GlobalWithRari)['~clientComponents'] = {}
if (typeof (globalThis as unknown as GlobalWithRari)['~clientComponentNames'] === 'undefined')
  (globalThis as unknown as GlobalWithRari)['~clientComponentNames'] = {}
if (typeof (globalThis as unknown as GlobalWithRari)['~clientComponentPaths'] === 'undefined')
  (globalThis as unknown as GlobalWithRari)['~clientComponentPaths'] = {}

if (typeof window !== 'undefined') {
  installRscChunkLoader()
  ;(globalThis as any).__rari_rsc_require__ = requireClientComponent
}

if (typeof window !== 'undefined') {
  if (!(window as unknown as WindowWithRari)['~rari'])
    (window as unknown as WindowWithRari)['~rari'] = (globalThis as unknown as GlobalWithRari)['~rari']

  if (!(window as unknown as WindowWithRari)['~rari'].streaming)
    (window as unknown as WindowWithRari)['~rari'].streaming = { bufferedRows: [] }
  else if (!(window as unknown as WindowWithRari)['~rari'].streaming!.bufferedRows)
    (window as unknown as WindowWithRari)['~rari'].streaming!.bufferedRows = []
}

if (import.meta.hot) {
  function resolveRariServerUrl(): string {
    if (typeof import.meta !== 'undefined' && import.meta.env?.RARI_SERVER_URL)
      return import.meta.env.RARI_SERVER_URL
    if (typeof window !== 'undefined')
      return window.location.origin

    return 'http://localhost:3000'
  }

  function isServerComponent(filePath: string): boolean {
    if (!filePath)
      return false
    try {
      return !!(globalThis as unknown as GlobalWithRari)['~rari'].serverComponents?.has(filePath)
    }
    catch {
      return false
    }
  }

  let hmrListenersReady = false
  const bufferedEvents: Array<{ event: string, data: any }> = []
  const handlers = new Map<string, (data: any) => void | Promise<void>>()

  function registerHandler(event: string, handler: (data: any) => void | Promise<void>) {
    handlers.set(event, handler)
    import.meta.hot!.on(event, async (data) => {
      if (!hmrListenersReady) {
        bufferedEvents.push({ event, data })
        return
      }
      try {
        await handler(data)
      }
      catch (error) {
        console.error(`[rari] HMR: Error in handler for '${event}':`, error)
      }
    })
  }

  registerHandler('rari:register-server-component', (data) => {
    if (data?.filePath) {
      ;(globalThis as unknown as GlobalWithRari)['~rari'].serverComponents = (globalThis as unknown as GlobalWithRari)['~rari'].serverComponents || new Set()
      ;(globalThis as unknown as GlobalWithRari)['~rari'].serverComponents!.add(data.filePath)
    }
  })

  registerHandler('rari:server-components-registry', (data) => {
    if (data?.serverComponents && Array.isArray(data.serverComponents)) {
      ;(globalThis as unknown as GlobalWithRari)['~rari'].serverComponents = (globalThis as unknown as GlobalWithRari)['~rari'].serverComponents || new Set()
      data.serverComponents.forEach((path: string) => {
        ;(globalThis as unknown as GlobalWithRari)['~rari'].serverComponents?.add(path)
      })
    }
  })

  registerHandler('vite:beforeFullReload', async (data) => {
    if (data?.path && isServerComponent(data.path)) {
      window.dispatchEvent(new CustomEvent('rari:rsc-invalidate', {
        detail: { filePath: data.path },
      }))
    }
  })

  registerHandler('rari:server-component-updated', async (data) => {
    const componentId = data?.id || data?.componentId
    const timestamp = data?.t || data?.timestamp

    if (componentId) {
      window.dispatchEvent(new CustomEvent('rari:rsc-invalidate', {
        detail: { componentId, filePath: data.filePath || data.file, type: 'server-component', timestamp },
      }))
    }
    else if (data?.path && isServerComponent(data.path)) {
      window.dispatchEvent(new CustomEvent('rari:rsc-invalidate', {
        detail: { filePath: data.path },
      }))
    }
  })

  registerHandler('rari:app-router-updated', async (data) => {
    if (!data || (!data.routePath && (!data.affectedRoutes || data.affectedRoutes.length === 0)))
      return

    try {
      const rariServerUrl = resolveRariServerUrl()

      const reloadResponse = await fetch(`${rariServerUrl}/_rari/hmr`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action: 'register', file_path: data.filePath }),
      })

      if (!reloadResponse.ok)
        throw new Error(`Component reload failed: ${reloadResponse.status}`)

      const result = await reloadResponse.json()
      if (result?.success !== true && result?.reloaded !== true)
        throw new Error(result?.error || 'Component reload unsuccessful')

      await fetch(`${rariServerUrl}/_rari/hmr`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action: 'invalidate', componentId: data.routePath || data.filePath, filePath: data.filePath }),
      })

      if (data.metadataChanged && data.metadata) {
        if (data.metadata.title)
          document.title = data.metadata.title
        if (data.metadata.description) {
          let metaDesc = document.querySelector('meta[name="description"]')
          if (!metaDesc) {
            metaDesc = document.createElement('meta')
            metaDesc.setAttribute('name', 'description')
            document.head.appendChild(metaDesc)
          }
          metaDesc.setAttribute('content', data.metadata.description)
        }
      }

      if (data.manifestUpdated && (window as unknown as WindowWithRari)['~rari']?.routeInfoCache)
        (window as unknown as WindowWithRari)['~rari'].routeInfoCache!.clear()

      window.dispatchEvent(new CustomEvent('rari:app-router-rerender', {
        detail: {
          routePath: data.routePath,
          affectedRoutes: data.affectedRoutes || [data.routePath],
          currentPath: window.location.pathname,
          preserveParams: true,
        },
      }))
    }
    catch (error) {
      console.error('[rari] HMR: App router update failed:', error)
    }
  })

  registerHandler('rari:server-action-updated', async (data) => {
    if (data?.filePath) {
      window.dispatchEvent(new CustomEvent('rari:rsc-invalidate', {
        detail: { filePath: data.filePath, type: 'server-action' },
      }))
    }
  })

  if (bufferedEvents.length > 0) {
    void (async () => {
      try {
        const eventsToReplay = [...bufferedEvents]
        bufferedEvents.length = 0
        for (const { event, data } of eventsToReplay) {
          const handler = handlers.get(event)
          if (handler)
            await handler(data)
        }
      }
      catch (error) {
        console.error('[rari] HMR: Error during event replay:', error)
      }
      finally {
        hmrListenersReady = true
      }
    })()
  }
  else {
    hmrListenersReady = true
  }
}
