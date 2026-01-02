export type {
  ApiRouteHandlers,
  RouteContext,
  RouteHandler,
} from './api-routes'

export { RariResponse } from './api-routes'

export type Request = globalThis.Request
export type Response = globalThis.Response

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
  NotFoundEntry,
  PageProps,
  RouteSegment,
  RouteSegmentType,
} from './router'

export {
  generateAppRouteManifest,
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
