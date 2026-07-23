import type { RouteInfoRequest, RouteInfoResponse } from './route-info-types'
import { getRariWindowBag } from '@/runtime/shared/rari-global'
import { isRecord, parseJsonRecord } from '@/shared/utils/type-guards'

class RouteInfoCache {
  private readonly cache = new Map<string, RouteInfoResponse>()
  private readonly pendingRequests = new Map<string, Promise<RouteInfoResponse>>()

  async get(path: string): Promise<RouteInfoResponse> {
    const cached = this.cache.get(path)
    if (cached) return cached

    const pending = this.pendingRequests.get(path)
    if (pending) return pending

    const promise = this.fetchRouteInfo(path)
    this.pendingRequests.set(path, promise)

    try {
      const result = await promise
      this.cache.set(path, result)
      return result
    } finally {
      this.pendingRequests.delete(path)
    }
  }

  private parseRouteInfoResponse(text: string): RouteInfoResponse {
    const parsed = parseJsonRecord(text)
    if (!parsed || !this.isRouteInfoResponse(parsed))
      throw new Error('Failed to parse route info response')

    return parsed
  }

  private isRouteInfoResponse(value: unknown): value is RouteInfoResponse {
    return (
      isRecord(value) &&
      typeof value.exists === 'boolean' &&
      Array.isArray(value.layouts) &&
      (value.loading === null || typeof value.loading === 'string') &&
      typeof value.isDynamic === 'boolean'
    )
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
      let errorMessage: string | undefined
      try {
        const text = await response.text()
        const parsed = parseJsonRecord(text)
        if (parsed && typeof parsed.error === 'string' && parsed.error !== '')
          errorMessage = parsed.error
      } catch {}

      if (errorMessage != null && errorMessage !== '') throw new Error(errorMessage)

      throw new Error(`Failed to fetch route info: ${response.status} ${response.statusText}`)
    }

    const clonedResponse = response.clone()

    try {
      const text = await response.text()
      return this.parseRouteInfoResponse(text)
    } catch (error) {
      try {
        const text = await clonedResponse.text()
        return this.parseRouteInfoResponse(text)
      } catch (parseError) {
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
  const windowRari = getRariWindowBag()!
  windowRari.routeInfoCache = routeInfoCache
}
