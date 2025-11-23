export type {
  ApiRouteHandlers,
  RouteContext,
  RouteHandler,
} from './api-routes'

export { RariResponse } from './api-routes'

export type Request = globalThis.Request
export type Response = globalThis.Response

export { headers } from './async-context'

export type {
  AppRouteEntry,
  AppRouteManifest,
  AppRouteMatch,
  ErrorEntry,
  ErrorProps,
  GenerateMetadata,
  GenerateStaticParams,
  LayoutEntry,
  LayoutProps,
  LoadingEntry,
  LoadingProps,
  NotFoundEntry,
  NotFoundProps,
  PageProps,
  RouteSegment,
  RouteSegmentType,
} from './router'

export {
  AppRouteGenerator,
  generateAppRouteManifest,
  loadManifest,
  writeManifest,
} from './router'

export {
  clearPropsCache,
  clearPropsCacheForComponent,
  extractMetadata,
  extractServerProps,
  extractServerPropsWithCache,
  extractStaticParams,
  hasServerSideDataFetching,
} from './router'

export type {
  MetadataResult,
  ServerPropsResult,
  StaticParamsResult,
} from './router'

export { rariRouter } from './router/vite-plugin'

export { createHttpRuntimeClient, HttpRuntimeClient } from './runtime-client'

export type { RuntimeClient } from './runtime-client'

export { defineRariConfig, defineRariOptions, rari } from './vite/index'
