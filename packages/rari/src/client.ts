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
} from './router/app-types'

export { ClientRouter } from './router/ClientRouter'

export type { ClientRouterProps } from './router/ClientRouter'

export { LayoutErrorBoundary } from './router/LayoutErrorBoundary'

export { createNavigationError, fetchWithTimeout, NavigationErrorHandler } from './router/navigation-error-handler'

export type { NavigationError, NavigationErrorHandlerOptions, NavigationErrorType } from './router/navigation-error-handler'
export { NavigationErrorOverlay } from './router/NavigationErrorOverlay'

export type { NavigationErrorOverlayProps } from './router/NavigationErrorOverlay'

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
  ServerPropsResult,
  StaticParamsResult,
} from './router/props-extractor'

export { routeInfoCache } from './router/route-info-client'
export type * from './router/route-info-types'

export { StatePreserver } from './router/StatePreserver'
export type { PreservedState, ScrollPosition, StatePreserverConfig } from './router/StatePreserver'

export {
  createErrorBoundary,
  createHttpRuntimeClient,
  createLoadingBoundary,
  DefaultError,
  DefaultLoading,
  ErrorBoundary,
  HttpRuntimeClient,
  LoadingSpinner,
  NotFound,
} from './runtime-client'

export type { RuntimeClient } from './runtime-client'
