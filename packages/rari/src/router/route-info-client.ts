import type { RouteInfoError, RouteInfoRequest, RouteInfoResponse } from './route-info-types'

class RouteInfoCache {
  private cache = new Map<string, RouteInfoResponse>()
  private pendingRequests = new Map<string, Promise<RouteInfoResponse>>()

  async get(path: string): Promise<RouteInfoResponse> {
    const cached = this.cache.get(path)
    if (cached)
      return cached

    const pending = this.pendingRequests.get(path)
    if (pending)
      return pending

    const promise = this.fetchRouteInfo(path)
    this.pendingRequests.set(path, promise)

    try {
      const result = await promise
      this.cache.set(path, result)
      return result
    }
    finally {
      this.pendingRequests.delete(path)
    }
  }

  private async fetchRouteInfo(path: string): Promise<RouteInfoResponse> {
    const request: RouteInfoRequest = { path }

    const response = await fetch('/_rari/route-info', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    })

    if (!response.ok) {
      try {
        const error: RouteInfoError = await response.json()
        throw new Error(error.error || 'Failed to fetch route info')
      }
      catch {
        throw new Error(`Failed to fetch route info: ${response.status} ${response.statusText}`)
      }
    }

    try {
      return await response.json()
    }
    catch (error) {
      try {
        const text = await response.text()
        return JSON.parse(text)
      }
      catch (parseError) {
        console.error('[RouteInfo] Failed to parse response:', { error, parseError, path })
        throw new Error('Failed to parse route info response')
      }
    }
  }

  clear(): void {
    this.cache.clear()
    this.pendingRequests.clear()
  }

  invalidate(path: string): void {
    this.cache.delete(path)
  }
}

export const routeInfoCache = new RouteInfoCache()

if (typeof window !== 'undefined') {
  const globalRari = (window as any)['~rari'] || {}
  if (!(window as any)['~rari'])
    (window as any)['~rari'] = globalRari
  globalRari.routeInfoCache = routeInfoCache
}
