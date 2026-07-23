import type { ReactNode } from 'react'

export type RouteSegmentType = 'static' | 'dynamic' | 'catch-all' | 'optional-catch-all'

export interface RouteSegment {
  type: RouteSegmentType
  value: string
  param?: string
}

export interface AppRouteEntry {
  path: string
  filePath: string
  css?: string[]
  componentId?: string
  segments: RouteSegment[]
  params: string[]
  isDynamic: boolean
  metadata?: RouteMetadata
  staticParams?: Array<Record<string, string | string[]>>
}

export interface LayoutEntry {
  path: string
  filePath: string
  css?: string[]
  componentId?: string
  parentPath?: string
  additionalPaths?: string[]
}

export interface LoadingEntry {
  path: string
  filePath: string
  css?: string[]
  componentId?: string
  additionalPaths?: string[]
}

export interface ErrorEntry {
  path: string
  filePath: string
  css?: string[]
  componentId?: string
  additionalPaths?: string[]
}

export interface NotFoundEntry {
  path: string
  filePath: string
  css?: string[]
  componentId?: string
  additionalPaths?: string[]
}

export interface OgImageEntry {
  path: string
  filePath: string
  width?: number
  height?: number
  contentType?: string
  additionalPaths?: string[]
}

export interface ApiRouteEntry {
  path: string
  filePath: string
  segments: RouteSegment[]
  params: string[]
  isDynamic: boolean
  methods: string[]
}

export interface TemplateEntry {
  path: string
  filePath: string
  css?: string[]
  componentId?: string
  parentPath?: string
  additionalPaths?: string[]
}

export interface AppRouteManifest {
  routes: AppRouteEntry[]
  layouts: LayoutEntry[]
  loading: LoadingEntry[]
  errors: ErrorEntry[]
  notFound: NotFoundEntry[]
  templates: TemplateEntry[]
  apiRoutes: ApiRouteEntry[]
  ogImages: OgImageEntry[]
  generated: string
}

export interface RouteMetadata {
  title?: string | { default?: string; template?: string; absolute?: string }
  description?: string
  keywords?: string | string[]
  openGraph?: {
    title?: string
    description?: string
    images?: string[] | Array<{ url: string; width?: number; height?: number; alt?: string }>
    url?: string
    siteName?: string
    locale?: string
    type?: string
  }
  twitter?: {
    card?: 'summary' | 'summary_large_image' | 'app' | 'player'
    title?: string
    description?: string
    images?: string[] | Array<{ url: string; alt?: string }>
    site?: string
    creator?: string
  }
  robots?:
    | {
        index?: boolean
        follow?: boolean
        noarchive?: boolean
        nosnippet?: boolean
        noimageindex?: boolean
        nocache?: boolean
      }
    | string
  icons?: {
    icon?: string | Array<{ url: string; type?: string; sizes?: string }>
    shortcut?: string
    apple?: string | Array<{ url: string; sizes?: string; type?: string }>
  }
  manifest?: string
  themeColor?: string | Array<{ media?: string; color: string }>
  viewport?:
    | string
    | {
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
  alternates?: {
    canonical?: string
    languages?: Record<string, string>
    types?: Record<string, string>
  }
}

export type Metadata = RouteMetadata

export interface RouteParams {
  readonly [key: string]: string | readonly string[]
}
export interface SearchParams {
  readonly [key: string]: string | readonly string[] | undefined
}

export type PageProps<
  TParams extends RouteParams = RouteParams,
  TSearchParams extends SearchParams = SearchParams,
> = Readonly<{
  params: TParams
  searchParams: TSearchParams
}>

export type LayoutProps<TParams extends RouteParams = RouteParams> = Readonly<{
  children: ReactNode
  params?: TParams
  pathname?: string
}>

export interface ErrorProps {
  readonly error: Error
  readonly reset: () => void
}

export interface AppRouteMatch {
  readonly route: AppRouteEntry
  readonly params: RouteParams
  readonly searchParams: SearchParams
  readonly layouts: readonly LayoutEntry[]
  readonly loading?: LoadingEntry
  readonly error?: ErrorEntry
  readonly templates: readonly TemplateEntry[]
  readonly pathname: string
}

export type GenerateMetadata<
  TParams extends RouteParams = RouteParams,
  TSearchParams extends SearchParams = SearchParams,
> = (
  props: Readonly<{
    params: TParams
    searchParams: TSearchParams
  }>,
) => RouteMetadata | Promise<RouteMetadata>

export type GenerateStaticParams<TParams extends RouteParams = RouteParams> = () =>
  | TParams[]
  | Promise<TParams[]>
