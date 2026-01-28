import type { ReactNode } from 'react'

export type RouteSegmentType
  = | 'static'
    | 'dynamic'
    | 'catch-all'
    | 'optional-catch-all'

export interface RouteSegment {
  type: RouteSegmentType
  value: string
  param?: string
}

export interface AppRouteEntry {
  path: string
  filePath: string
  segments: RouteSegment[]
  params: string[]
  isDynamic: boolean
  metadata?: RouteMetadata
}

export interface LayoutEntry {
  path: string
  filePath: string
  parentPath?: string
}

export interface LoadingEntry {
  path: string
  filePath: string
  componentId: string
}

export interface ErrorEntry {
  path: string
  filePath: string
}

export interface NotFoundEntry {
  path: string
  filePath: string
}

export interface OgImageEntry {
  path: string
  filePath: string
  width?: number
  height?: number
  contentType?: string
}

export interface ApiRouteEntry {
  path: string
  filePath: string
  segments: RouteSegment[]
  params: string[]
  isDynamic: boolean
  methods: string[]
}

export interface AppRouteManifest {
  routes: AppRouteEntry[]
  layouts: LayoutEntry[]
  loading: LoadingEntry[]
  errors: ErrorEntry[]
  notFound: NotFoundEntry[]
  apiRoutes: ApiRouteEntry[]
  ogImages: OgImageEntry[]
  generated: string
}

export interface RouteMetadata {
  title?: string | { default?: string, template?: string, absolute?: string }
  description?: string
  keywords?: string | string[]
  openGraph?: {
    title?: string
    description?: string
    images?: string[] | Array<{ url: string, width?: number, height?: number, alt?: string }>
    url?: string
    siteName?: string
    locale?: string
    type?: string
  }
  twitter?: {
    card?: 'summary' | 'summary_large_image' | 'app' | 'player'
    title?: string
    description?: string
    images?: string[] | Array<{ url: string, alt?: string }>
    site?: string
    creator?: string
  }
  robots?: {
    index?: boolean
    follow?: boolean
    noarchive?: boolean
    nosnippet?: boolean
    noimageindex?: boolean
    nocache?: boolean
  } | string
  icons?: {
    icon?: string | Array<{ url: string, type?: string, sizes?: string }>
    shortcut?: string
    apple?: string | Array<{ url: string, sizes?: string, type?: string }>
  }
  manifest?: string
  themeColor?: string | Array<{ media?: string, color: string }>
  viewport?: string | {
    width?: string | number
    height?: string | number
    initialScale?: number
    minimumScale?: number
    maximumScale?: number
    userScalable?: boolean
  }
  appleWebApp?: {
    capable?: boolean
    title?: string
    statusBarStyle?: 'default' | 'black' | 'black-translucent'
  }
  canonical?: string
}

export type Metadata = RouteMetadata

export type RouteParams = Record<string, string | string[]>
export type SearchParams = Record<string, string | string[] | undefined>

export interface PageProps<
  TParams extends RouteParams = RouteParams,
  TSearchParams extends SearchParams = SearchParams,
> {
  params: TParams
  searchParams: TSearchParams
}

export interface LayoutProps {
  children: ReactNode
  params?: Record<string, string | string[]>
  pathname?: string
}

export interface ErrorProps {
  error: Error
  reset: () => void
}

export interface AppRouteMatch {
  route: AppRouteEntry
  params: Record<string, string | string[]>
  searchParams: Record<string, string | string[] | undefined>
  layouts: LayoutEntry[]
  loading?: LoadingEntry
  error?: ErrorEntry
  pathname: string
}

export type GenerateMetadata<TParams extends Record<string, string | string[]> = Record<string, string | string[]>> = (props: {
  params: TParams
  searchParams: Record<string, string | string[] | undefined>
}) => RouteMetadata | Promise<RouteMetadata>

export type GenerateStaticParams<TParams extends Record<string, string | string[]> = Record<string, string | string[]>> = () =>
  | TParams[]
  | Promise<TParams[]>
