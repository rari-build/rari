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
    routeInfoCache?: Map<string, any>
    cookies?: () => CookieStore
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
