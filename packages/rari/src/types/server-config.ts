export interface ServerCSPConfig {
  scriptSrc?: string[]
  styleSrc?: string[]
  imgSrc?: string[]
  fontSrc?: string[]
  connectSrc?: string[]
  defaultSrc?: string[]
  workerSrc?: string[]
}

export interface ServerRateLimitConfig {
  enabled?: boolean
  requestsPerSecond?: number
  burstSize?: number
  revalidateRequestsPerMinute?: number
}

export interface ServerSpamBlockerConfig {
  enabled?: boolean
}

export interface ServerCacheControlConfig {
  routes: Record<string, string>
}

export interface ServerConfig {
  csp?: ServerCSPConfig
  rateLimit?: ServerRateLimitConfig
  spamBlocker?: ServerSpamBlockerConfig
  cacheControl?: ServerCacheControlConfig
}
