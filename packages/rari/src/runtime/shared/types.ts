export interface CookieOptions {
  path?: string
  domain?: string
  expires?: Date
  maxAge?: number
  httpOnly?: boolean
  secure?: boolean
  sameSite?: boolean | 'lax' | 'strict' | 'none'
  priority?: 'low' | 'medium' | 'high'
  partitioned?: boolean
}

export interface ReadonlyCookie {
  name: string
  value: string
}

export interface ReadonlyHeaders {
  get: (name: string) => string | null
  has: (name: string) => boolean
  entries: () => IterableIterator<[string, string]>
  forEach: (callback: (value: string, key: string) => void) => void
  keys: () => IterableIterator<string>
  values: () => IterableIterator<string>
}

export interface CookieStore {
  get: (name: string) => ReadonlyCookie | undefined
  getAll: (name?: string) => ReadonlyCookie[]
  has: (name: string) => boolean
  set: ((name: string, value: string, options?: CookieOptions) => void) & ((options: { name: string, value: string } & CookieOptions) => void)
  delete: (name: string) => void
  toString: () => string
}

export interface ComponentInfo {
  id: string
  path: string
  type: string
  component?: any
  registered: boolean
  loader?: () => Promise<any>
  loading?: boolean
  loadPromise?: Promise<any>
  loadError?: unknown
}

export interface GlobalWithRari {
  '~rari': {
    isDevelopment?: boolean
    navigationId?: number
    AppRouterProvider?: any
    ClientRouter?: any
    getClientComponent?: (id: string) => Promise<any>
    preloadClientComponent?: (id: string) => Promise<void>
    streaming?: {
      enabled?: boolean
      complete?: boolean
      bufferedRows: string[]
      streamingBridgeInstalled?: boolean
    }
    serverComponents?: Set<string>
    routeInfoCache?: { clear: () => void, invalidate?: (path: string) => void }
    cookies?: () => CookieStore
    headers?: () => ReadonlyHeaders
    useCacheDynamicDepth?: number
    useCacheBuildId?: string
    useCachePrivateKey?: string
    pageCacheTags?: Set<string>
    invalidateUseCache?: (input: { tag?: string, path?: string }) => Promise<void>
    markUseCacheDynamic?: () => void
  }
  '~clientComponents': Record<string, ComponentInfo>
  '~clientComponentPaths': Record<string, string>
  '~clientComponentNames': Record<string, string>
}

export interface WindowWithRari extends Window {
  '~rari': GlobalWithRari['~rari']
  '~clientComponents': GlobalWithRari['~clientComponents']
  '~clientComponentPaths': GlobalWithRari['~clientComponentPaths']
  '~clientComponentNames': GlobalWithRari['~clientComponentNames']
}
