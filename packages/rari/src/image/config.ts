export interface RariImageConfig {
  remotePatterns?: RemotePattern[]
  deviceSizes?: number[]
  imageSizes?: number[]
  formats?: ('avif' | 'webp')[]
  quality?: number[]
  path?: string
  minimumCacheTTL?: number
  maxCacheSize?: number
}

export interface RemotePattern {
  protocol?: 'http' | 'https'
  hostname: string
  port?: string
  pathname?: string
}

export interface RariConfig {
  images?: RariImageConfig
}
