import type {
  AppRouteEntry,
  ErrorEntry,
  LayoutEntry,
  LoadingEntry,
  RouteMetadata,
  TemplateEntry,
} from '@rari/core/router'
import type { ReactNode } from 'react'

// The manifest schema is framework-agnostic and owned by @rari/core.
export type {
  ApiRouteEntry,
  AppIconEntry,
  AppRouteEntry,
  AppRouteManifest,
  ErrorEntry,
  LayoutEntry,
  LoadingEntry,
  NotFoundEntry,
  OgImageEntry,
  RouteMetadata,
  RouteSegment,
  RouteSegmentType,
  TemplateEntry,
} from '@rari/core/router'

export type Metadata = RouteMetadata

export interface RouteParams {
  readonly [key: string]: string | readonly string[]
}

export interface SearchParams {
  readonly [key: string]: string | readonly string[] | undefined
}

export interface PageProps<
  TParams extends RouteParams = RouteParams,
  TSearchParams extends SearchParams = SearchParams,
> {
  readonly params: TParams
  readonly searchParams: TSearchParams
}

export interface LayoutProps<TParams extends RouteParams = RouteParams> {
  readonly children: ReactNode
  readonly params?: TParams
  readonly pathname?: string
}

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
