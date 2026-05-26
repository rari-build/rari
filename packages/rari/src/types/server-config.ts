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

export interface ServerConfig {
  csp?: ServerCSPConfig
  cacheControl?: ServerCacheControlConfig
}
