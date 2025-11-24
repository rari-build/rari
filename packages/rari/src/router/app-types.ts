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
  generated: string
}

export interface RouteMetadata {
  title?: string
  description?: string
  openGraph?: {
    title?: string
    description?: string
    images?: string[]
  }
  twitter?: {
    card?: string
    title?: string
    description?: string
    images?: string[]
  }
}

export interface PageProps<
  TParams extends Record<string, string | string[]> = Record<string, string | string[]>,
  TSearchParams extends Record<string, string | string[] | undefined> = Record<string, string | string[] | undefined>,
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

export interface LoadingProps {
}

export interface NotFoundProps {
}

export interface ServerPropsResult {
  props: Record<string, any>
  revalidate?: number | false
  notFound?: boolean
  redirect?: {
    destination: string
    permanent?: boolean
  }
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

export interface AppRouterConfig {
  basePath?: string
  appDir?: string
  enabled?: boolean
  caseSensitive?: boolean
}

export type GenerateMetadata<TParams extends Record<string, string | string[]> = Record<string, string | string[]>> = (props: {
  params: TParams
  searchParams: Record<string, string | string[] | undefined>
}) => RouteMetadata | Promise<RouteMetadata>

export type GenerateStaticParams<TParams extends Record<string, string | string[]> = Record<string, string | string[]>> = () =>
  | TParams[]
  | Promise<TParams[]>

export type PageComponent<TParams extends Record<string, string | string[]> = Record<string, string | string[]>, TSearchParams extends Record<string, string | string[] | undefined> = Record<string, string | string[] | undefined>> = (
  props: PageProps<TParams, TSearchParams>,
) => ReactNode | Promise<ReactNode>

export type LayoutComponent<TParams extends Record<string, string | string[]> = Record<string, string | string[]>> = (
  props: LayoutProps & { params?: TParams },
) => ReactNode | Promise<ReactNode>

export type ErrorComponent = (props: ErrorProps) => ReactNode

export type LoadingComponent = (props?: LoadingProps) => ReactNode

export type NotFoundComponent = (props?: NotFoundProps) => ReactNode
