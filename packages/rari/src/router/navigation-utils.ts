import type { RouteInfo } from './navigation-types'
import type { AppRouteManifest, LayoutEntry, RouteSegment } from './types'

export function parseRoutePath(path: string): string[] {
  const normalized = path.replace(/(^\/+)|(\/+$)/g, '')
  return normalized ? normalized.split('/') : []
}

export function matchRouteParams(
  _routePath: string,
  routeSegments: RouteSegment[],
  actualPath: string,
): Record<string, string | string[]> | null {
  const actualSegments = parseRoutePath(actualPath)
  const params: Record<string, string | string[]> = {}

  let actualIndex = 0

  for (let i = 0; i < routeSegments.length; i++) {
    const segment = routeSegments[i]

    if (actualIndex >= actualSegments.length) {
      if (segment.type === 'optional-catch-all') {
        if (segment.param)
          params[segment.param] = []
        continue
      }

      return null
    }

    switch (segment.type) {
      case 'static':
        if (actualSegments[actualIndex] !== segment.value)
          return null
        actualIndex++
        break

      case 'dynamic':
        if (segment.param)
          params[segment.param] = actualSegments[actualIndex]
        actualIndex++
        break

      case 'catch-all':
      case 'optional-catch-all':
        if (segment.param)
          params[segment.param] = actualSegments.slice(actualIndex)
        actualIndex = actualSegments.length
        break
    }
  }

  if (actualIndex !== actualSegments.length)
    return null

  return params
}

const layoutChainCache = new Map<string, LayoutEntry[]>()

let cachedManifestVersion: number | undefined

function getManifestVersion(manifest: AppRouteManifest): number {
  return manifest.layouts.length * 1000 + manifest.routes.length
}

function invalidateCacheIfNeeded(manifest: AppRouteManifest): void {
  const currentVersion = getManifestVersion(manifest)

  if (cachedManifestVersion !== currentVersion) {
    layoutChainCache.clear()
    cachedManifestVersion = currentVersion
  }
}

export function findLayoutChain(
  routePath: string,
  manifest: AppRouteManifest,
): LayoutEntry[] {
  invalidateCacheIfNeeded(manifest)

  const cached = layoutChainCache.get(routePath)
  if (cached)
    return cached

  const chain: LayoutEntry[] = []
  const segments = parseRoutePath(routePath)

  for (let i = 0; i <= segments.length; i++) {
    const currentPath = i === 0 ? '/' : `/${segments.slice(0, i).join('/')}`
    const layout = manifest.layouts.find(l => l.path === currentPath)
    if (layout)
      chain.push(layout)
  }

  layoutChainCache.set(routePath, chain)

  return chain
}

export function normalizePath(path: string): string {
  if (!path || path === '/')
    return '/'

  let normalized = path
  while (normalized.endsWith('/') && normalized.length > 1)
    normalized = normalized.slice(0, -1)

  if (!normalized.startsWith('/'))
    normalized = `/${normalized}`

  return normalized
}

export function createRouteInfo(
  path: string,
  manifest: AppRouteManifest,
  searchParams?: URLSearchParams,
): RouteInfo {
  const normalizedPath = normalizePath(path)
  const layoutChain = findLayoutChain(normalizedPath, manifest)

  const route = manifest.routes.find((r) => {
    const params = matchRouteParams(r.path, r.segments, normalizedPath)
    return params !== null
  })

  const params: Record<string, string | string[]> = {}
  if (route) {
    const matchedParams = matchRouteParams(route.path, route.segments, normalizedPath)
    if (matchedParams)
      Object.assign(params, matchedParams)
  }

  return {
    path: normalizedPath,
    params,
    searchParams: searchParams || new URLSearchParams(),
    layoutChain,
  }
}

export function isExternalUrl(url: string, currentOrigin?: string): boolean {
  try {
    const urlObj = new URL(url, currentOrigin || window.location.origin)
    return urlObj.origin !== (currentOrigin || window.location.origin)
  }
  catch {
    return false
  }
}

export function extractPathname(url: string): string {
  try {
    const urlObj = new URL(url, window.location.origin)
    return urlObj.pathname + urlObj.hash
  }
  catch {
    return url
  }
}
