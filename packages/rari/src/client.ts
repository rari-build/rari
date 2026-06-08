export { ClientRouter } from './router/ClientRouter'

export type { ClientRouterProps } from './router/ClientRouter'

export { NavigationErrorHandler } from './router/navigation-error-handler'

export type { NavigationError, NavigationErrorHandlerOptions, NavigationErrorType } from './router/navigation-error-handler'

export {
  clearPropsCache,
  clearPropsCacheForComponent,
  extractMetadata,
  extractServerProps,
  extractServerPropsWithCache,
  extractStaticParams,
  hasServerSideDataFetching,
} from './router/props-extractor'

export type {
  MetadataResult,
  ServerSidePropsResult,
  StaticParamsResult,
} from './router/props-extractor'
export type * from './router/route-info-types'

export { StatePreserver } from './router/StatePreserver'

export type { PreservedState, ScrollPosition, StatePreserverConfig } from './router/StatePreserver'
export type {
  AppRouteEntry,
  AppRouteManifest,
  AppRouteMatch,
  ErrorEntry,
  ErrorProps,
  GenerateMetadata,
  GenerateStaticParams,
  LayoutEntry,
  LoadingEntry,
  NotFoundEntry,
  RouteSegment,
  RouteSegmentType,
  TemplateEntry,
} from './router/types'

export type {
  ServerConfig,
  ServerCSPConfig,
} from './types/server-config'
