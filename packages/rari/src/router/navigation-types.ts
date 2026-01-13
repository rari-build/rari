import type { LayoutEntry } from './types'

export interface RouteInfo {
  path: string
  params: Record<string, string | string[]>
  searchParams: URLSearchParams
  layoutChain: LayoutEntry[]
}

export interface NavigationOptions {
  replace?: boolean
  scroll?: boolean
  shallow?: boolean
  historyKey?: string
}
