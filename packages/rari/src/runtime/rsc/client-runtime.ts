import { isRecord } from '@/shared/utils/type-guards'
import { installRscChunkLoader, requireClientComponent } from '../shared/get-client-component'
import {
  getClientComponentNames,
  getClientComponentPaths,
  getClientComponents,
  getRariGlobal,
  getRariWindowBag,
} from '../shared/rari-global'

getRariGlobal().isDevelopment = process.env.NODE_ENV !== 'production'

getClientComponents()
getClientComponentNames()
getClientComponentPaths()

declare global {
  var __rari_rsc_require__: typeof requireClientComponent | undefined
}

if (typeof window !== 'undefined') {
  installRscChunkLoader()
  Reflect.set(globalThis, '__rari_rsc_require__', requireClientComponent)
}

if (typeof window !== 'undefined') {
  const windowRari = getRariWindowBag()!
  windowRari.streaming ??= { bufferedRows: [] }
}

if (import.meta.hot) {
  function resolveRariServerUrl(): string {
    if (import.meta.env.RARI_SERVER_URL != null && import.meta.env.RARI_SERVER_URL !== '')
      return import.meta.env.RARI_SERVER_URL
    if (typeof window !== 'undefined') return window.location.origin

    return 'http://localhost:3000'
  }

  function isServerComponent(filePath: string): boolean {
    if (!filePath) return false
    try {
      return getRariGlobal().serverComponents?.has(filePath) === true
    } catch {
      return false
    }
  }

  interface RegisterServerComponentPayload {
    filePath?: string
  }

  interface ServerComponentsRegistryPayload {
    serverComponents?: string[]
  }

  interface BeforeFullReloadPayload {
    path?: string
  }

  interface ServerComponentUpdatedPayload {
    id?: string
    componentId?: string
    t?: number
    timestamp?: number
    filePath?: string
    file?: string
    path?: string
  }

  interface AppRouterUpdatedPayload {
    routePath?: string
    affectedRoutes?: string[]
    filePath?: string
    manifestUpdated?: boolean
    metadataChanged?: boolean
    metadata?: {
      title?: string
      description?: string
    }
  }

  interface ServerActionUpdatedPayload {
    filePath?: string
  }

  let hmrListenersReady = false
  const bufferedEvents: Array<{ event: string; data: unknown }> = []
  const handlers = new Map<string, (data: unknown) => void | Promise<void>>()

  function registerHandler<T>(
    event: string,
    handler: (data: T) => void | Promise<void>,
    predicate: (data: unknown) => data is T,
  ) {
    handlers.set(event, async (data: unknown) => {
      if (!predicate(data)) return

      await handler(data)
    })
    import.meta.hot!.on(event, (data: unknown) => {
      void (async () => {
        if (!hmrListenersReady) {
          bufferedEvents.push({ event, data })
          return
        }
        try {
          const registeredHandler = handlers.get(event)
          if (registeredHandler) await registeredHandler(data)
        } catch (error) {
          console.error(`[rari] HMR: Error in handler for '${event}':`, error)
        }
      })()
    })
  }

  function hasFilePath(
    data: unknown,
  ): data is RegisterServerComponentPayload & { filePath: string } {
    return isRecord(data) && typeof data.filePath === 'string' && data.filePath !== ''
  }

  function hasServerComponents(
    data: unknown,
  ): data is ServerComponentsRegistryPayload & { serverComponents: string[] } {
    return isRecord(data) && Array.isArray(data.serverComponents)
  }

  function hasReloadPath(data: unknown): data is BeforeFullReloadPayload & { path: string } {
    return isRecord(data) && typeof data.path === 'string' && data.path !== ''
  }

  function isServerComponentUpdatedPayload(data: unknown): data is ServerComponentUpdatedPayload {
    return isRecord(data)
  }

  function isAppRouterUpdatedPayload(data: unknown): data is AppRouterUpdatedPayload {
    return isRecord(data)
  }

  function hasActionFilePath(
    data: unknown,
  ): data is ServerActionUpdatedPayload & { filePath: string } {
    return isRecord(data) && typeof data.filePath === 'string' && data.filePath !== ''
  }

  registerHandler(
    'rari:register-server-component',
    data => {
      const rari = getRariGlobal()
      rari.serverComponents ??= new Set()
      rari.serverComponents.add(data.filePath)
    },
    hasFilePath,
  )

  registerHandler(
    'rari:server-components-registry',
    data => {
      const rari = getRariGlobal()
      rari.serverComponents ??= new Set()
      for (const path of data.serverComponents) rari.serverComponents.add(path)
    },
    hasServerComponents,
  )

  registerHandler(
    'vite:beforeFullReload',
    data => {
      if (isServerComponent(data.path)) {
        window.dispatchEvent(
          new CustomEvent('rari:rsc-invalidate', {
            detail: { filePath: data.path },
          }),
        )
      }
    },
    hasReloadPath,
  )

  registerHandler(
    'rari:server-component-updated',
    data => {
      const componentId = data.id != null && data.id !== '' ? data.id : data.componentId
      const timestamp = data.t != null && data.t !== 0 ? data.t : data.timestamp

      if (componentId != null && componentId !== '') {
        window.dispatchEvent(
          new CustomEvent('rari:rsc-invalidate', {
            detail: {
              componentId,
              filePath: data.filePath != null && data.filePath !== '' ? data.filePath : data.file,
              type: 'server-component',
              timestamp,
            },
          }),
        )
      } else if (data.path != null && data.path !== '' && isServerComponent(data.path)) {
        window.dispatchEvent(
          new CustomEvent('rari:rsc-invalidate', {
            detail: { filePath: data.path },
          }),
        )
      }
    },
    isServerComponentUpdatedPayload,
  )

  registerHandler(
    'rari:app-router-updated',
    async data => {
      if (
        (data.routePath == null || data.routePath === '') &&
        (data.affectedRoutes == null || data.affectedRoutes.length === 0)
      )
        return

      try {
        const rariServerUrl = resolveRariServerUrl()

        const reloadResponse = await fetch(`${rariServerUrl}/_rari/hmr`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ action: 'register', file_path: data.filePath }),
        })

        if (!reloadResponse.ok) throw new Error(`Component reload failed: ${reloadResponse.status}`)

        const result: unknown = await reloadResponse.json()
        const success = isRecord(result) && result.success === true
        const reloaded = isRecord(result) && result.reloaded === true
        const errorMessage =
          isRecord(result) && typeof result.error === 'string'
            ? result.error
            : 'Component reload unsuccessful'
        if (!success && !reloaded) throw new Error(errorMessage)

        await fetch(`${rariServerUrl}/_rari/hmr`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            action: 'invalidate',
            componentId:
              data.routePath != null && data.routePath !== '' ? data.routePath : data.filePath,
            filePath: data.filePath,
          }),
        })

        if (data.metadataChanged && data.metadata) {
          if (data.metadata.title != null && data.metadata.title !== '')
            document.title = data.metadata.title
          if (data.metadata.description != null && data.metadata.description !== '') {
            let metaDesc = document.querySelector('meta[name="description"]')
            if (!metaDesc) {
              metaDesc = document.createElement('meta')
              metaDesc.setAttribute('name', 'description')
              document.head.appendChild(metaDesc)
            }
            metaDesc.setAttribute('content', data.metadata.description)
          }
        }

        const windowRari = getRariWindowBag()
        if (data.manifestUpdated && windowRari?.routeInfoCache) windowRari.routeInfoCache.clear()

        window.dispatchEvent(
          new CustomEvent('rari:app-router-rerender', {
            detail: {
              routePath: data.routePath,
              affectedRoutes:
                data.affectedRoutes ?? (data.routePath != null ? [data.routePath] : []),
              currentPath: window.location.pathname,
              preserveParams: true,
            },
          }),
        )
      } catch (error) {
        console.error('[rari] HMR: App router update failed:', error)
      }
    },
    isAppRouterUpdatedPayload,
  )

  registerHandler(
    'rari:server-action-updated',
    data => {
      window.dispatchEvent(
        new CustomEvent('rari:rsc-invalidate', {
          detail: { filePath: data.filePath, type: 'server-action' },
        }),
      )
    },
    hasActionFilePath,
  )

  if (bufferedEvents.length > 0) {
    void (async () => {
      try {
        const eventsToReplay = [...bufferedEvents]
        bufferedEvents.length = 0
        for (const { event, data } of eventsToReplay) {
          const handler = handlers.get(event)
          if (handler) await handler(data)
        }
      } catch (error) {
        console.error('[rari] HMR: Error during event replay:', error)
      } finally {
        hmrListenersReady = true
      }
    })()
  } else {
    hmrListenersReady = true
  }
}
