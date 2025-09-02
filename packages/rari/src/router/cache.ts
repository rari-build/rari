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

const cacheConfigs = new Map<string, PageCacheConfig>()

export function registerPageCacheConfig(pagePath: string, config: PageCacheConfig): void {
  cacheConfigs.set(pagePath, config)
}

export function getPageCacheConfig(pagePath: string): PageCacheConfig | undefined {
  return cacheConfigs.get(pagePath)
}

export function clearPageCacheConfigs(): void {
  cacheConfigs.clear()
}

export function getAllPageCacheConfigs(): Map<string, PageCacheConfig> {
  return new Map(cacheConfigs)
}

export function extractPageCacheConfig(pageModule: any): PageCacheConfig | undefined {
  if (pageModule && typeof pageModule === 'object') {
    return pageModule.cacheConfig
  }
  return undefined
}

export function registerPageCacheConfigFromModule(pagePath: string, pageModule: any): void {
  const cacheConfig = extractPageCacheConfig(pageModule)
  if (cacheConfig) {
    registerPageCacheConfig(pagePath, cacheConfig)
  }
}

export function applyCacheHeaders(
  headers: Record<string, string>,
  cacheConfig: PageCacheConfig,
): void {
  for (const [key, value] of Object.entries(cacheConfig)) {
    if (value !== undefined) {
      headers[key.toLowerCase()] = value
    }
  }
}
