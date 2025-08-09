/* eslint-disable react-hooks-extra/no-unnecessary-use-prefix */

export interface SuspenseClientOptions {
  maxCacheSize?: number
  enableDebug?: boolean
  preloadEnabled?: boolean
  cacheTTL?: number
}

export interface CacheEntry<T = any> {
  value: T
  timestamp: number
  hitCount: number
  expiresAt?: number
}

export interface SuspenseBoundaryInfo {
  id: string
  fallback: React.ReactNode
  pending: Set<string>
  resolved: boolean
  error: Error | null
  createdAt: number
  resolvedAt?: number
}

export interface SuspensePromiseInfo {
  id: string
  componentId: string
  boundaryId: string
  cacheKey?: string
  status: 'pending' | 'resolved' | 'rejected'
  promise: Promise<any>
  createdAt: number
  resolvedAt?: number
}

export class SuspenseCache<T = any> {
  private storage = new Map<string, CacheEntry<T>>()
  private maxSize: number
  private accessOrder: string[] = []

  constructor(maxSize: number = 1000) {
    this.maxSize = maxSize
  }

  get(key: string): T | undefined {
    const entry = this.storage.get(key)
    if (!entry)
      return undefined

    if (entry.expiresAt && Date.now() > entry.expiresAt) {
      this.storage.delete(key)
      this.accessOrder = this.accessOrder.filter(k => k !== key)
      return undefined
    }

    this.accessOrder = this.accessOrder.filter(k => k !== key)
    this.accessOrder.push(key)
    entry.hitCount++

    return entry.value
  }

  set(key: string, value: T, ttl?: number): void {
    while (this.storage.size >= this.maxSize && this.accessOrder.length > 0) {
      const oldestKey = this.accessOrder.shift()!
      this.storage.delete(oldestKey)
    }

    const entry: CacheEntry<T> = {
      value,
      timestamp: Date.now(),
      hitCount: 0,
      expiresAt: ttl ? Date.now() + ttl : undefined,
    }

    this.storage.set(key, entry)
    this.accessOrder = this.accessOrder.filter(k => k !== key)
    this.accessOrder.push(key)
  }

  async preload(
    key: string,
    factory: () => Promise<T>,
    ttl?: number,
  ): Promise<T> {
    const existing = this.get(key)
    if (existing !== undefined) {
      return existing
    }

    const value = await factory()
    this.set(key, value, ttl)
    return value
  }

  clear(): void {
    this.storage.clear()
    this.accessOrder = []
  }

  getStats() {
    return {
      size: this.storage.size,
      maxSize: this.maxSize,
      hitRate: this.calculateHitRate(),
      totalEntries: this.storage.size,
    }
  }

  private calculateHitRate(): number {
    if (this.storage.size === 0)
      return 0

    const totalHits = Array.from(this.storage.values()).reduce(
      (sum, entry) => sum + entry.hitCount,
      0,
    )
    const totalAccesses = Array.from(this.storage.values()).reduce(
      (sum, entry) => sum + Math.max(entry.hitCount, 1),
      0,
    )

    return totalAccesses > 0 ? totalHits / totalAccesses : 0
  }
}

export class ClientSuspenseBoundary {
  public id: string
  public fallback: React.ReactNode
  public pending = new Set<string>()
  public resolved = false
  public errorState: Error | null = null
  public createdAt: number

  constructor(id: string, fallback: React.ReactNode) {
    this.id = id
    this.fallback = fallback
    this.createdAt = Date.now()
  }

  resolve(): void {
    this.resolved = true
    this.pending.clear()
  }

  setError(error: Error): void {
    this.errorState = error
    this.pending.clear()
  }
}

export class SuspenseClient {
  public cache: SuspenseCache
  private boundaries = new Map<string, ClientSuspenseBoundary>()
  private promises = new Map<string, SuspensePromiseInfo>()
  public options: Required<SuspenseClientOptions>

  constructor(options: SuspenseClientOptions = {}) {
    this.options = {
      maxCacheSize: options.maxCacheSize ?? 1000,
      enableDebug: options.enableDebug ?? false,
      preloadEnabled: options.preloadEnabled ?? true,
      cacheTTL: options.cacheTTL ?? 5 * 60 * 1000,
    }

    this.cache = new SuspenseCache(this.options.maxCacheSize)
    this.setupGlobalHandlers()
  }

  private setupGlobalHandlers(): void {
    if (typeof window !== 'undefined') {
      window.addEventListener('suspense:resolve', (event: Event) => {
        const customEvent = event as CustomEvent
        const { boundaryId, content } = customEvent.detail
        this.resolveBoundary(boundaryId, content)
      })

      window.addEventListener('suspense:error', (event: Event) => {
        const customEvent = event as CustomEvent
        const { boundaryId, error } = customEvent.detail
        this.errorBoundary(
          boundaryId,
          new Error(error.message || 'Suspense error'),
        )
      })

      window.addEventListener('rsc:suspense-resolve', (event: Event) => {
        const customEvent = event as CustomEvent
        const { boundaryId, data } = customEvent.detail
        this.handleStreamedSuspenseResolution(boundaryId, data)
      })
    }

    if (this.options.enableDebug) {
      console.warn('[SuspenseClient] Global handlers setup complete')
    }
  }

  registerBoundary(
    id: string,
    fallback: React.ReactNode,
  ): ClientSuspenseBoundary {
    if (this.boundaries.has(id)) {
      if (this.options.enableDebug) {
        console.warn(
          `[SuspenseClient] Boundary ${id} already exists, updating fallback`,
        )
      }
      const existing = this.boundaries.get(id)!
      existing.fallback = fallback
      return existing
    }

    const boundary = new ClientSuspenseBoundary(id, fallback)
    this.boundaries.set(id, boundary)

    if (this.options.enableDebug) {
      console.warn(`[SuspenseClient] Registered boundary: ${id}`)
    }

    return boundary
  }

  resolveBoundary(boundaryId: string, _content?: any): void {
    const boundary = this.boundaries.get(boundaryId)
    if (!boundary) {
      if (this.options.enableDebug) {
        console.warn(
          `[SuspenseClient] Boundary ${boundaryId} not found for resolution`,
        )
      }
      return
    }

    boundary.resolve()

    if (this.options.enableDebug) {
      console.warn(`[SuspenseClient] Resolved boundary: ${boundaryId}`)
    }

    this.triggerBoundaryUpdate(boundaryId)
  }

  errorBoundary(boundaryId: string, error: Error): void {
    const boundary = this.boundaries.get(boundaryId)
    if (!boundary) {
      if (this.options.enableDebug) {
        console.warn(
          `[SuspenseClient] Boundary ${boundaryId} not found for error handling`,
        )
      }
      return
    }

    boundary.setError(error)

    if (this.options.enableDebug) {
      console.error(`[SuspenseClient] Error in boundary ${boundaryId}:`, error)
    }

    this.triggerBoundaryError(boundaryId, error)
  }

  async preloadResource<T>(key: string, factory: () => Promise<T>): Promise<T> {
    if (!this.options.preloadEnabled) {
      return factory()
    }

    return this.cache.preload(key, factory, this.options.cacheTTL)
  }

  getCachedValue<T>(key: string): T | undefined {
    return this.cache.get(key)
  }

  private handleStreamedSuspenseResolution(
    boundaryId: string,
    data: any,
  ): void {
    try {
      const parsedData = typeof data === 'string' ? JSON.parse(data) : data

      if (this.options.enableDebug) {
        console.warn(
          `[SuspenseClient] Received streamed resolution for ${boundaryId}:`,
          parsedData,
        )
      }

      this.resolveBoundary(boundaryId, parsedData)
    }
    catch (error) {
      if (this.options.enableDebug) {
        console.error(
          `[SuspenseClient] Failed to handle streamed resolution:`,
          error,
        )
      }
      this.errorBoundary(boundaryId, error as Error)
    }
  }

  private triggerBoundaryUpdate(boundaryId: string): void {
    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent('rari:boundary-resolved', {
          detail: { boundaryId },
        }),
      )
    }
  }

  private triggerBoundaryError(boundaryId: string, error: Error): void {
    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent('rari:boundary-error', {
          detail: { boundaryId, error },
        }),
      )
    }
  }

  getStats() {
    return {
      boundaries: {
        total: this.boundaries.size,
        resolved: Array.from(this.boundaries.values()).filter(b => b.resolved).length,
        pending: Array.from(this.boundaries.values()).filter(
          b => !b.resolved && !b.errorState,
        ).length,
        errored: Array.from(this.boundaries.values()).filter(
          b => b.errorState,
        ).length,
      },
      promises: {
        total: this.promises.size,
        resolved: Array.from(this.promises.values()).filter(
          p => p.status === 'resolved',
        ).length,
        pending: Array.from(this.promises.values()).filter(
          p => p.status === 'pending',
        ).length,
        rejected: Array.from(this.promises.values()).filter(
          p => p.status === 'rejected',
        ).length,
      },
      cache: this.cache.getStats(),
    }
  }

  cleanup(): void {
    const cutoff = Date.now() - 5 * 60 * 1000

    for (const [id, boundary] of this.boundaries.entries()) {
      if (boundary.resolved && boundary.createdAt < cutoff) {
        this.boundaries.delete(id)
      }
    }

    for (const [id, promise] of this.promises.entries()) {
      if (
        (promise.status === 'resolved' || promise.status === 'rejected')
        && promise.createdAt < cutoff
      ) {
        this.promises.delete(id)
      }
    }

    if (this.options.enableDebug) {
      console.warn('[SuspenseClient] Cleanup completed')
    }
  }

  createUseHook() {
    return <T>(resource: Promise<T> | { read: () => T }): T => {
      if (resource && typeof (resource as any).then === 'function') {
        const promise = resource as Promise<T>

        const cacheKey = (promise as any).__cacheKey
        if (cacheKey) {
          const cached = this.getCachedValue<T>(cacheKey)
          if (cached !== undefined) {
            return cached
          }
        }

        throw promise
      }

      if (resource && typeof (resource as any).read === 'function') {
        return (resource as any).read()
      }

      throw new Error('use() called with unsupported resource type')
    }
  }
}

export function initializeSuspenseClient(
  options?: SuspenseClientOptions,
): SuspenseClient {
  if (typeof window !== 'undefined') {
    (window as any).__rari_suspense_client = new SuspenseClient(options)

    setInterval(() => {
      (window as any).__rari_suspense_client?.cleanup()
    }, 60000)

    return (window as any).__rari_suspense_client
  }

  return new SuspenseClient(options)
}

export function createCacheKey(
  component: string,
  props: Record<string, any>,
): string {
  const propsHash = JSON.stringify(props, Object.keys(props).sort())
  return `${component}:${btoa(propsHash).slice(0, 16)}`
}

export async function preloadSuspenseResource<T>(
  key: string,
  factory: () => Promise<T>,
  client?: SuspenseClient,
): Promise<T> {
  const suspenseClient
    = client
      || (typeof window !== 'undefined'
        ? (window as any).__rari_suspense_client
        : null)

  if (suspenseClient) {
    return suspenseClient.preloadResource(key, factory)
  }

  return factory()
}

export function getSuspenseClient(): SuspenseClient | null {
  if (typeof window !== 'undefined') {
    return (window as any).__rari_suspense_client || null
  }
  return null
}

export function createEnhancedResourceHook(suspenseClient: SuspenseClient) {
  return function useResource<T>(
    resource: Promise<T> | string | { $$typeof: symbol, _currentValue: T },
  ): T {
    if (resource && typeof (resource as any).then === 'function') {
      const promise = resource as Promise<T>

      let cacheKey = (promise as any).__cacheKey
      if (!cacheKey) {
        cacheKey = `promise-${Math.random().toString(36).substr(2, 9)}`;
        (promise as any).__cacheKey = cacheKey
      }

      const cached = suspenseClient.getCachedValue<T>(cacheKey)
      if (cached !== undefined) {
        return cached
      }

      promise.then(
        (value) => {
          suspenseClient.cache.set(
            cacheKey,
            value,
            suspenseClient.options.cacheTTL,
          )
        },
        (error) => {
          console.error('[use] Promise rejected:', error)
        },
      );

      (promise as any).$$typeof = Symbol.for('react.suspense.pending')
      throw promise
    }

    if (
      resource
      && typeof resource === 'object'
      && (resource as any).$$typeof
    ) {
      const context = resource as { $$typeof: symbol, _currentValue: T }
      if (context.$$typeof === Symbol.for('react.context')) {
        return context._currentValue
      }
    }

    if (typeof resource === 'string') {
      const cached = suspenseClient.getCachedValue<T>(resource)
      if (cached !== undefined) {
        return cached
      }
      throw new Error(`Cache miss for key: ${resource}`)
    }

    throw new Error('use() called with unsupported resource type')
  }
}

export function setupRSCStreamHandlers(suspenseClient: SuspenseClient): void {
  if (typeof window === 'undefined')
    return

  const originalFetch = window.fetch
  window.fetch = async (input, init) => {
    const response = await originalFetch(input, init)

    if (response.headers.get('content-type')?.includes('text/x-component')) {
      const reader = response.body?.getReader()
      if (reader) {
        processRSCStream(reader, suspenseClient)
      }
    }

    return response
  }
}

async function processRSCStream(
  reader: ReadableStreamDefaultReader<Uint8Array>,
  suspenseClient: SuspenseClient,
): Promise<void> {
  const decoder = new TextDecoder()
  let buffer = ''

  try {
    while (true) {
      const { done, value } = await reader.read()
      if (done)
        break

      buffer += decoder.decode(value, { stream: true })

      const lines = buffer.split('\n')
      buffer = lines.pop() || ''

      for (const line of lines) {
        if (line.trim()) {
          await processSuspenseStreamLine(line, suspenseClient)
        }
      }
    }

    if (buffer.trim()) {
      await processSuspenseStreamLine(buffer, suspenseClient)
    }
  }
  catch (error) {
    console.error('[SuspenseClient] Error processing RSC stream:', error)
  }
  finally {
    reader.releaseLock()
  }
}

async function processSuspenseStreamLine(
  line: string,
  suspenseClient: SuspenseClient,
): Promise<void> {
  try {
    const [, data] = line.split(':', 2)
    const parsed = JSON.parse(data)

    if (Array.isArray(parsed) && parsed[1] && typeof parsed[1] === 'string') {
      if (parsed[1].startsWith('$')) {
        const boundaryId = parsed[1].slice(1)
        const content = parsed[3]?.children

        if (content) {
          suspenseClient.resolveBoundary(boundaryId, content)
        }
      }
      else if (parsed[1] === 'react.suspense.resolved') {
        const boundaryId = parsed[2]
        const content = parsed[3]?.children

        if (boundaryId && content) {
          suspenseClient.resolveBoundary(boundaryId, content)
        }
      }
      else if (parsed[1] === 'react.suspense.error') {
        const boundaryId = parsed[2]
        const errorInfo = parsed[3]

        if (boundaryId && errorInfo) {
          const error = new Error(
            errorInfo.message || 'Suspense boundary error',
          )
          suspenseClient.errorBoundary(boundaryId, error)
        }
      }
    }
  }
  catch (error) {
    console.error(
      '[SuspenseClient] Failed to process stream line:',
      line,
      error,
    )
  }
}

let globalSuspenseClient: SuspenseClient | null = null

export function getGlobalSuspenseClient(): SuspenseClient | null {
  return globalSuspenseClient
}

export function useSuspenseClient(): SuspenseClient {
  const client = getGlobalSuspenseClient()
  if (!client) {
    throw new Error(
      'SuspenseClient not initialized. Make sure to call initializeSuspenseClient() first.',
    )
  }
  return client
}

export function setGlobalSuspenseClient(client: SuspenseClient): void {
  globalSuspenseClient = client

  if (typeof window !== 'undefined') {
    (window as any).__rari_suspense_client = client
  }
}

export function debugSuspenseState(): void {
  if (typeof window !== 'undefined' && (window as any).__rari_suspense_client) {
    const client = (window as any).__rari_suspense_client as SuspenseClient
    console.warn('[SuspenseClient] Current state:', client.getStats())
  }
}

export default function createSuspenseClient(
  options?: SuspenseClientOptions,
): SuspenseClient {
  const client = new SuspenseClient(options)
  setGlobalSuspenseClient(client)
  return client
}
