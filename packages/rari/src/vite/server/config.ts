export interface ServerCSPConfig {
  readonly scriptSrc?: readonly string[]
  readonly styleSrc?: readonly string[]
  readonly imgSrc?: readonly string[]
  readonly fontSrc?: readonly string[]
  readonly connectSrc?: readonly string[]
  readonly defaultSrc?: readonly string[]
  readonly workerSrc?: readonly string[]
  readonly frameAncestors?: readonly string[]
  readonly baseUri?: readonly string[]
  readonly formAction?: readonly string[]
  readonly useNonces?: boolean
}

export interface ServerCacheControlConfig {
  readonly routes: Readonly<Record<string, string>>
}

export interface ServerCacheLayerConfig {
  readonly handler?: string
  readonly url?: string
  readonly maxEntries?: number
  readonly defaultTtlSecs?: number
  readonly maxBytes?: number
}

export interface ServerCacheConfig {
  readonly layers?: Readonly<Record<string, ServerCacheLayerConfig>>
}

export interface ServerUseCacheConfig {
  readonly remote?: ServerCacheLayerConfig
  readonly buildId?: string
}

export interface ServerActionConfig {
  readonly allowedOrigins?: readonly string[]
}

export interface ServerConfig {
  csp?: ServerCSPConfig
  cacheControl?: ServerCacheControlConfig
  cache?: ServerCacheConfig
  useCache?: ServerUseCacheConfig
  action?: ServerActionConfig
  jsPoolSize?: number
  htmlLimitedBots?: string
}
