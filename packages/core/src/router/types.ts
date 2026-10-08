// The route manifest the Rust host reads from `dist/server/routes.json`.
//
// Every framework adapter's build emits this shape (via `generateAppRouteManifest`
// with its own file conventions), so the host can match routes, resolve layout
// chains, key its response cache, and answer 404s without knowing the framework.
import type { AppIconEntry } from './app-icons'

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
  /** Layout variant the page selects (`page@name`); see `route-file.ts`. */
  layout?: string
  /** `page!`: the page renders without any layout. */
  skipLayouts?: boolean
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
  /** Variant name (`layout-name`); absent for the directory's default layout. */
  name?: string
  /** `layout!`: a top layout, the layouts above it are skipped. */
  skipParents?: boolean
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
  /** Layout variant the page selects (`not-found@name`). */
  layout?: string
  /** `not-found!`: the page renders without any layout. */
  skipLayouts?: boolean
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
  appIcons: AppIconEntry[]
  generated: string
}

/**
 * Page metadata the host merges into the document `<head>`. A plain data
 * schema (no framework types), so any adapter can produce it from its own
 * head/metadata conventions.
 */
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
    icon?:
      | string
      | { url: string; type?: string; sizes?: string; rel?: string }
      | Array<{ url: string; type?: string; sizes?: string; rel?: string }>
    shortcut?: string
    apple?:
      | string
      | { url: string; sizes?: string; type?: string; rel?: string }
      | Array<{ url: string; sizes?: string; type?: string; rel?: string }>
    other?:
      | { url: string; rel?: string; type?: string; sizes?: string }
      | Array<{ url: string; rel?: string; type?: string; sizes?: string }>
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
