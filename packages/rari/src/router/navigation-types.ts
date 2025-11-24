import type { LayoutEntry } from './app-types'

export type NavigationTrigger
  = | 'link-click'
    | 'browser-back'
    | 'browser-forward'
    | 'programmatic'
    | 'hmr'

export interface RouteInfo {
  path: string
  params: Record<string, string | string[]>
  searchParams: URLSearchParams
  layoutChain: LayoutEntry[]
}

export interface NavigationContext {
  from: RouteInfo
  to: RouteInfo
  trigger: NavigationTrigger
  timestamp: number
}

export interface NavigationOptions {
  replace?: boolean
  scroll?: boolean
  shallow?: boolean
  historyKey?: string
}
