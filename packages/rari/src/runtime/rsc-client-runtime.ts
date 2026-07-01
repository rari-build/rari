/* eslint-disable node/prefer-global/process */
import type { GlobalWithRari, WindowWithRari } from './shared/types'
import { EXTENSION_REGEX } from '../shared/regex-constants'
import { getClientComponent as getClientComponentShared } from './shared/get-client-component'

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
  ;(globalThis as any).__rari_chunk_load__ = (chunkId: string) => {
    const clientComponents = (globalThis as unknown as GlobalWithRari)['~clientComponents'] || {}

    let componentInfo = clientComponents[chunkId]

    if (!componentInfo) {
      const normalized = chunkId.replace(/\\/g, '/')

      componentInfo = clientComponents[normalized]
        || Object.values(clientComponents).find((info: any) =>
          info && (info.path === chunkId || info.path === normalized),
        )

      if (!componentInfo) {
        const nameMatch = chunkId.match(/\/([A-Z][a-zA-Z0-9]*)(?:[_.]|$)/)
          || chunkId.match(/\/([a-z][a-zA-Z0-9-]*)(?:[_.]|$)/)
        if (nameMatch) {
          const componentName = nameMatch[1]
          componentInfo = Object.values(clientComponents).find((info: any) => {
            if (!info || !info.path)
              return false
            const p = info.path.replace(/\\/g, '/')
            return (
              p.endsWith(`/${componentName}.tsx`)
              || p.endsWith(`/${componentName}.ts`)
              || p.endsWith(`/${componentName}.jsx`)
              || p.endsWith(`/${componentName}.js`)
              || p.includes(`/${componentName}/index.`)
              || info.id === componentName
              || (info as any).exportName === componentName
            )
          }) as any
        }
      }
    }

    if (componentInfo && !componentInfo.component && componentInfo.loader && !componentInfo.loading) {
      componentInfo.loading = true
      componentInfo.loadPromise = componentInfo.loader()
        .then((mod: any) => {
          componentInfo.component = mod
          componentInfo.registered = true
          componentInfo.loading = false
          return mod
        })
        .catch((err: Error) => {
          componentInfo.loading = false
          componentInfo.loadPromise = undefined
          console.error(`[rari] Failed to load chunk ${chunkId}:`, err)
          throw err
        })
      return componentInfo.loadPromise
    }

    if (componentInfo && componentInfo.loadPromise)
      return componentInfo.loadPromise

    return Promise.resolve()
  }

  ;(globalThis as any).__rari_rsc_require__ = (id: string) => {
    const clientComponents = (globalThis as unknown as GlobalWithRari)['~clientComponents'] || {}

    let componentInfo = clientComponents[id]

    if (!componentInfo) {
      const hashIdx = id.indexOf('#')
      if (hashIdx > 0) {
        const pathPart = id.substring(0, hashIdx)
        componentInfo = clientComponents[pathPart]
      }
    }

    if (!componentInfo) {
      const matchingKey = Object.keys(clientComponents).find(key =>
        key === id
        || key.startsWith(`${id}#`)
        || key.endsWith(`/${id}`)
        || key.includes(id),
      )
      if (matchingKey)
        componentInfo = clientComponents[matchingKey]
    }

    if (componentInfo && componentInfo.component) {
      const mod = componentInfo.component
      if (typeof mod === 'object' && mod !== null && ('default' in mod || '__esModule' in mod))
        return mod
      if (typeof mod === 'function')
        return { default: mod, [(componentInfo as any).exportName || 'default']: mod }

      return mod
    }

    if (componentInfo && !componentInfo.component && componentInfo.loader) {
      if (!componentInfo.loading) {
        componentInfo.loading = true
        componentInfo.loadPromise = componentInfo.loader()
          .then((mod: any) => {
            componentInfo.component = mod
            componentInfo.registered = true
            componentInfo.loading = false
            return mod
          })
          .catch((err: Error) => {
            componentInfo.loading = false
            componentInfo.loadPromise = undefined
            console.error(`[rari] Failed to load component ${id}:`, err)
          })
      }

      const loadPromise = componentInfo.loadPromise!
      const SuspendingComponent = (props: any) => {
        if (componentInfo.component) {
          const mod = componentInfo.component
          const Component = mod.default ?? mod
          return Component(props)
        }
        throw loadPromise
      }
      SuspendingComponent.displayName = `Lazy(${(componentInfo as any).exportName || id})`

      return { default: SuspendingComponent, __esModule: true }
    }

    if (import.meta.env?.DEV && !componentInfo)
      console.warn(`[rari] __rari_rsc_require__: component "${id}" not found in registry`)

    return {}
  }

  ;(globalThis as any).__rari_rsc_client_require__ = (id: string) => {
    return (globalThis as any).__rari_rsc_require__(id)
  }
}

if (typeof window !== 'undefined') {
  if (!(window as unknown as WindowWithRari)['~rari'])
    (window as unknown as WindowWithRari)['~rari'] = (globalThis as unknown as GlobalWithRari)['~rari']

  if (!(window as unknown as WindowWithRari)['~rari'].streaming)
    (window as unknown as WindowWithRari)['~rari'].streaming = { bufferedRows: [], bufferedEvents: [] }
}

export function registerClientComponent(componentFunction: any, id: string, exportName: string): void {
  const componentName = exportName === 'default'
    ? (componentFunction.name || id.split('/').pop()?.replace(EXTENSION_REGEX, '') || 'DefaultComponent')
    : exportName

  const componentId = componentName

  const componentInfo = {
    id: componentId,
    path: id,
    type: 'client',
    component: componentFunction,
    registered: true,
  }

  if (typeof (globalThis as unknown as GlobalWithRari)['~clientComponents'] === 'undefined')
    (globalThis as unknown as GlobalWithRari)['~clientComponents'] = {}

  const fullId = `${id}#${exportName}`
  ;(globalThis as unknown as GlobalWithRari)['~clientComponents'][componentId] = componentInfo
  ;(globalThis as unknown as GlobalWithRari)['~clientComponents'][id] = componentInfo
  ;(globalThis as unknown as GlobalWithRari)['~clientComponents'][fullId] = componentInfo

  ;(globalThis as unknown as GlobalWithRari)['~clientComponentPaths'][id] = componentId
  ;(globalThis as unknown as GlobalWithRari)['~clientComponentNames'][componentName] = componentId

  if (componentFunction) {
    componentFunction['~isClientComponent'] = true
    componentFunction['~clientComponentId'] = componentId
  }

  if (typeof window !== 'undefined') {
    fetch('/_rari/register-client', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        component_id: componentId,
        file_path: id,
        export_name: exportName,
      }),
    }).catch((error) => {
      console.error('[rari] Failed to register client component with server:', error)
    })
  }
}

export function getClientComponent(id: string): any {
  return getClientComponentShared(id)
}

export function createClientModuleMap(): Record<string, any> {
  const moduleMap: Record<string, any> = {}
  for (const [componentId, componentInfo] of Object.entries((globalThis as unknown as GlobalWithRari)['~clientComponents'])) {
    moduleMap[componentId] = {
      id: componentId,
      chunks: [componentInfo.path],
      name: componentId,
      async: false,
      default: componentInfo.component,
    }
  }

  return moduleMap
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
