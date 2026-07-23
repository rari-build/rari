import type { LayoutEntry, TemplateEntry } from '../build/types'

export interface RouteInfo {
  readonly path: string
  readonly params: Record<string, string | string[]>
  readonly searchParams: URLSearchParams
  readonly layoutChain: readonly LayoutEntry[]
  readonly templateChain: readonly TemplateEntry[]
}

export interface NavigationOptions {
  readonly replace?: boolean
  readonly scroll?: boolean
  readonly shallow?: boolean
  readonly historyKey?: string
}
