import type { LayoutEntry, TemplateEntry } from './types'

export interface RouteInfo {
  path: string
  params: Record<string, string | string[]>
  searchParams: URLSearchParams
  layoutChain: LayoutEntry[]
  templateChain: TemplateEntry[]
}

export interface NavigationOptions {
  replace?: boolean
  scroll?: boolean
  shallow?: boolean
  historyKey?: string
}
