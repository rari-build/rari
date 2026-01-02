export {
  generateAppRouteManifest,
} from './app-routes'

export type {
  ApiRouteEntry,
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
  RouteMetadata,
  RouteSegment,
  RouteSegmentType,
} from './app-types'

export { ClientRouter } from './ClientRouter'

export type { ClientRouterProps } from './ClientRouter'

export { LayoutErrorBoundary } from './LayoutErrorBoundary'

export {
  createNavigationError,
  fetchWithTimeout,
  NavigationErrorHandler,
} from './navigation-error-handler'

export type {
  NavigationError,
  NavigationErrorHandlerOptions,
  NavigationErrorType,
} from './navigation-error-handler'

export { NavigationErrorOverlay } from './NavigationErrorOverlay'

export type { NavigationErrorOverlayProps } from './NavigationErrorOverlay'

export {
  clearPropsCache,
  clearPropsCacheForComponent,
  extractMetadata,
  extractServerProps,
  extractServerPropsWithCache,
  extractStaticParams,
  hasServerSideDataFetching,
} from './props-extractor'

export type {
  MetadataResult,
  ServerPropsResult,
  StaticParamsResult,
} from './props-extractor'

export { routeInfoCache } from './route-info-client'
export type * from './route-info-types'

export { StatePreserver } from './StatePreserver'

export type {
  PreservedState,
  ScrollPosition,
  StatePreserverConfig,
} from './StatePreserver'

export { rariRouter } from './vite-plugin'
