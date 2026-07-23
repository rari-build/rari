export interface CookieOptions {
  readonly path?: string
  readonly domain?: string
  readonly expires?: Date
  readonly maxAge?: number
  readonly httpOnly?: boolean
  readonly secure?: boolean
  readonly sameSite?: boolean | 'lax' | 'strict' | 'none'
  readonly priority?: 'low' | 'medium' | 'high'
  readonly partitioned?: boolean
}

export interface ReadonlyCookie {
  readonly name: string
  readonly value: string
}

export interface ReadonlyHeaders {
  readonly get: (name: string) => string | null
  readonly has: (name: string) => boolean
  readonly entries: () => IterableIterator<[string, string]>
  readonly forEach: (callback: (value: string, key: string) => void) => void
  readonly keys: () => IterableIterator<string>
  readonly values: () => IterableIterator<string>
}

export interface CookieStore {
  readonly get: (name: string) => ReadonlyCookie | undefined
  readonly getAll: (name?: string) => readonly ReadonlyCookie[]
  readonly has: (name: string) => boolean
  readonly set: ((name: string, value: string, options?: Readonly<CookieOptions>) => void) &
    ((options: Readonly<{ name: string; value: string } & CookieOptions>) => void)
  readonly delete: (name: string) => void
  readonly toString: () => string
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
  exportName?: string
  displayName?: string
}

export type RariGlobalBag = NonNullable<GlobalWithRari['~rari']>

export interface GlobalWithRari {
  '~rari'?: {
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
    routeInfoCache?: { clear: () => void; invalidate?: (path: string) => void }
    cookies?: () => CookieStore
    headers?: () => ReadonlyHeaders
    useCacheDynamicDepth?: number
    useCacheBuildId?: string
    useCachePrivateKey?: string
    pageCacheTags?: Set<string>
    invalidateUseCache?: (input: Readonly<{ tag?: string; path?: string }>) => Promise<void>
    markUseCacheDynamic?: () => void
  }
  '~clientComponents'?: Record<string, ComponentInfo>
  '~clientComponentPaths'?: Record<string, string>
  '~clientComponentNames'?: Record<string, string>
}

export interface WindowWithRari extends Window {
  '~rari'?: GlobalWithRari['~rari']
  '~clientComponents'?: GlobalWithRari['~clientComponents']
  '~clientComponentPaths'?: GlobalWithRari['~clientComponentPaths']
  '~clientComponentNames'?: GlobalWithRari['~clientComponentNames']
}

declare global {
  interface GlobalThis {
    '~rari'?: GlobalWithRari['~rari']
    '~clientComponents'?: GlobalWithRari['~clientComponents']
    '~clientComponentPaths'?: GlobalWithRari['~clientComponentPaths']
    '~clientComponentNames'?: GlobalWithRari['~clientComponentNames']
    '~rariExecuteProxy'?: (
      request: Readonly<{
        readonly url: string
        readonly method: string
        readonly headers: { readonly [key: string]: string }
      }>,
    ) => Promise<{
      continue: boolean
      redirect?: { destination: string; permanent: boolean }
      rewrite?: string
      requestHeaders?: Record<string, string | string[]>
      responseHeaders?: Record<string, string | string[]>
      response?: {
        status: number
        headers: Record<string, string | string[]>
        body?: string
      }
    }>
    'registerClientComponent'?: (key: string, id: string, reference: unknown) => void
    '__rari_rsc_require__'?: (id: string) => unknown
    '__rari_client_ready'?: boolean
    '__rari_f'?: ReadonlyArray<0 | string | readonly [2, string]>
    '__rari_chunk_load__'?: (chunkId: string) => Promise<unknown>
  }

  interface Window {
    '~rari'?: GlobalWithRari['~rari']
    '~clientComponents'?: GlobalWithRari['~clientComponents']
    '~clientComponentPaths'?: GlobalWithRari['~clientComponentPaths']
    '~clientComponentNames'?: GlobalWithRari['~clientComponentNames']
  }
}
