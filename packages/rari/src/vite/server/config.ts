export interface ServerCSPConfig {
  readonly scriptSrc?: readonly string[]
  readonly styleSrc?: readonly string[]
  readonly imgSrc?: readonly string[]
  readonly fontSrc?: readonly string[]
  readonly connectSrc?: readonly string[]
  readonly defaultSrc?: readonly string[]
  readonly workerSrc?: readonly string[]
}

export interface ServerCacheControlConfig {
  readonly routes: Readonly<Record<string, string>>
}

export interface ServerCacheLayerConfig {
  readonly handler?: string
  readonly url?: string
  readonly maxEntries?: number
  readonly defaultTtlSecs?: number
}

export interface ServerCacheConfig {
  readonly layers?: Readonly<Record<string, ServerCacheLayerConfig>>
}

export interface ServerUseCacheConfig {
  readonly remote?: ServerCacheLayerConfig
  readonly buildId?: string
}

export interface ServerConfig {
  csp?: ServerCSPConfig
  cacheControl?: ServerCacheControlConfig
  cache?: ServerCacheConfig
  useCache?: ServerUseCacheConfig
  jsPoolSize?: number
  htmlLimitedBots?: string
}
