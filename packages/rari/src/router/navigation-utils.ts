import type { RouteInfo } from './navigation-types'
import type { AppRouteManifest, LayoutEntry, RouteSegment } from './types'

const LEADING_TRAILING_SLASHES_REGEX = /(^\/+)|(\/+$)/g

export function parseRoutePath(path: string): string[] {
  const normalized = path.replace(LEADING_TRAILING_SLASHES_REGEX, '')
  return normalized ? normalized.split('/') : []
}

function safeDecodeURIComponent(value: string): string | null {
  try {
    return decodeURIComponent(value)
  }
  catch {
    return null
  }
}

function matchStaticSegment(actualSegment: string, segmentValue: string): boolean {
  return actualSegment === segmentValue
}

function matchDynamicSegment(
  actualSegment: string,
  segmentParam: string | undefined,
  params: Record<string, string | string[]>,
): boolean {
  if (!segmentParam)
    return false

  const decoded = safeDecodeURIComponent(actualSegment)
  if (decoded === null)
    return false

  params[segmentParam] = decoded
  return true
}

function matchCatchAllSegment(
  actualSegments: string[],
  actualIndex: number,
  segmentParam: string | undefined,
  params: Record<string, string | string[]>,
): { success: boolean, newIndex: number } {
  if (!segmentParam)
    return { success: false, newIndex: actualIndex }

  const decodedSegments: string[] = []
  for (const seg of actualSegments.slice(actualIndex)) {
    const decoded = safeDecodeURIComponent(seg)
    if (decoded === null)
      return { success: false, newIndex: actualIndex }
    decodedSegments.push(decoded)
  }

  params[segmentParam] = decodedSegments
  return { success: true, newIndex: actualSegments.length }
}

function handleOptionalCatchAll(
  actualIndex: number,
  actualSegmentsLength: number,
  segmentParam: string | undefined,
  params: Record<string, string | string[]>,
): void {
  if (actualIndex >= actualSegmentsLength && segmentParam) {
    params[segmentParam] = []
  }
}

function processSegment(
  segment: RouteSegment,
  actualSegments: string[],
  actualIndex: number,
  params: Record<string, string | string[]>,
): { success: boolean, newIndex: number } {
  if (actualIndex >= actualSegments.length) {
    if (segment.type === 'optional-catch-all') {
      if (!segment.param)
        return { success: false, newIndex: actualIndex }

      handleOptionalCatchAll(actualIndex, actualSegments.length, segment.param, params)
      return { success: true, newIndex: actualIndex }
    }

    return { success: false, newIndex: actualIndex }
  }

  switch (segment.type) {
    case 'static':
      if (!matchStaticSegment(actualSegments[actualIndex], segment.value))
        return { success: false, newIndex: actualIndex }

      return { success: true, newIndex: actualIndex + 1 }

    case 'dynamic':
      if (!matchDynamicSegment(actualSegments[actualIndex], segment.param, params))
        return { success: false, newIndex: actualIndex }

      return { success: true, newIndex: actualIndex + 1 }

    case 'catch-all':
    case 'optional-catch-all':
      return matchCatchAllSegment(actualSegments, actualIndex, segment.param, params)

    default:
      return { success: false, newIndex: actualIndex }
  }
}

export function matchRouteParams(
  _routePath: string,
  routeSegments: RouteSegment[],
  actualPath: string,
): Record<string, string | string[]> | null {
  const actualSegments = parseRoutePath(actualPath)
  const params: Record<string, string | string[]> = {}

  let actualIndex = 0

  for (const segment of routeSegments) {
    const result = processSegment(segment, actualSegments, actualIndex, params)
    if (!result.success)
      return null
    actualIndex = result.newIndex
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
    /* v8 ignore start - defensive check, matchedParams should always be non-null if route was found */
    if (matchedParams)
      Object.assign(params, matchedParams)
    /* v8 ignore stop */
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
