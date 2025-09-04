export interface CacheConfig {
  routes?: Record<string, string>
  static?: string
  serverComponents?: string
}

export interface PageCacheConfig {
  'cache-control'?: string
  'vary'?: string
  [key: string]: string | undefined
}
