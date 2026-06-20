export interface ServerCSPConfig {
  scriptSrc?: string[]
  styleSrc?: string[]
  imgSrc?: string[]
  fontSrc?: string[]
  connectSrc?: string[]
  defaultSrc?: string[]
  workerSrc?: string[]
}

export interface ServerCacheControlConfig {
  routes: Record<string, string>
}

export interface ServerCacheLayerConfig {
  handler?: string
  maxEntries?: number
  defaultTtlSecs?: number
}

export interface ServerCacheConfig {
  layers?: Record<string, ServerCacheLayerConfig>
}

export interface ServerConfig {
  csp?: ServerCSPConfig
  cacheControl?: ServerCacheControlConfig
  cache?: ServerCacheConfig
}
