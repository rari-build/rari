import type { LayoutEntry } from './app-types'
import type { LayoutDiff } from './LayoutManager'

export interface LayoutDataCache {
  layoutPath: string
  data: any
  fetchedAt: number
  segment: string
  isValid: boolean
}

export interface FetchLayoutDataOptions {
  abortSignal?: AbortSignal
  force?: boolean
  cacheTTL?: number
}

export class LayoutDataManager {
  private dataCache: Map<string, LayoutDataCache>
  private readonly DEFAULT_CACHE_TTL = 5 * 60 * 1000

  constructor() {
    this.dataCache = new Map()
  }

  public hasSegmentChanged(
    layoutPath: string,
    currentRoute: string,
    targetRoute: string,
  ): boolean {
    const currentSegment = this.extractSegmentForLayout(layoutPath, currentRoute)
    const targetSegment = this.extractSegmentForLayout(layoutPath, targetRoute)

    return currentSegment !== targetSegment
  }

  private extractSegmentForLayout(layoutPath: string, route: string): string {
    const segment = layoutPath
      .replace(/^\/app\//, '')
      .replace(/\/layout\.tsx$/, '')
      .replace(/^layout\.tsx$/, '')

    if (!segment) {
      return ''
    }

    const routeParts = route.split('/').filter(Boolean)
    const segmentParts = segment.split('/').filter(Boolean)

    if (segmentParts.length === 0) {
      return ''
    }

    const segmentDepth = segmentParts.length - 1
    return routeParts[segmentDepth] || ''
  }

  public getLayoutsNeedingRefetch(
    layoutDiff: LayoutDiff,
    currentRoute: string,
    targetRoute: string,
  ): LayoutEntry[] {
    const layoutsToRefetch: LayoutEntry[] = []

    layoutsToRefetch.push(...layoutDiff.mountLayouts)

    for (const layout of layoutDiff.commonLayouts) {
      if (this.hasSegmentChanged(layout.filePath, currentRoute, targetRoute)) {
        layoutsToRefetch.push(layout)
      }
    }

    layoutsToRefetch.push(...layoutDiff.updateLayouts)

    return layoutsToRefetch
  }

  public getCachedData(
    layoutPath: string,
    segment: string,
    cacheTTL: number = this.DEFAULT_CACHE_TTL,
  ): any | undefined {
    const cached = this.dataCache.get(layoutPath)

    if (!cached) {
      return undefined
    }

    const now = Date.now()
    const age = now - cached.fetchedAt

    if (age > cacheTTL || cached.segment !== segment || !cached.isValid) {
      return undefined
    }

    return cached.data
  }

  public cacheData(layoutPath: string, segment: string, data: any): void {
    this.dataCache.set(layoutPath, {
      layoutPath,
      data,
      fetchedAt: Date.now(),
      segment,
      isValid: true,
    })
  }

  public invalidateCache(layoutPath: string): void {
    const cached = this.dataCache.get(layoutPath)
    if (cached) {
      cached.isValid = false
    }
  }

  public invalidateAllCache(): void {
    for (const cached of this.dataCache.values()) {
      cached.isValid = false
    }
  }

  public clearCache(layoutPath: string): void {
    this.dataCache.delete(layoutPath)
  }

  public clearAllCache(): void {
    this.dataCache.clear()
  }

  public getCacheSize(): number {
    return this.dataCache.size
  }

  public hasCachedData(layoutPath: string): boolean {
    return this.dataCache.has(layoutPath)
  }

  public getCachedLayoutPaths(): string[] {
    return Array.from(this.dataCache.keys())
  }
}
