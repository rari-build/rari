export {
  buildSearchString,
  buildUrl,
  extractParamNames,
  findMatchingRoute,
  isDynamicRoute,
  isPathActive,
  joinPaths,
  normalizePathname,
  parseSearchParams,
  parseUrl,
} from './router'

export {
  Navigate,
  Outlet,
  RouteComponent as Route,
  RouterProvider,
  Routes,
  useNavigation,
  useParams,
  usePathname,
  useRoute,
  useRouter,
  useSearchParams,
  withRouter,
} from './router'

export type {
  AppRouteEntry,
  AppRouteManifest,
  AppRouteMatch,
  LayoutEntry,
  LoadingEntry,
  ErrorEntry,
  NotFoundEntry,
  RouteSegment,
  RouteSegmentType,
  PageProps,
  LayoutProps,
  LoadingProps,
  ErrorProps,
  NotFoundProps,
  GenerateStaticParams,
  GenerateMetadata,
  RouterContext,
} from './router'

export {
  createHttpRuntimeClient,
  HttpRuntimeClient,
  ErrorBoundary,
  createErrorBoundary,
  createLoadingBoundary,
  NotFound,
  LoadingSpinner,
  DefaultLoading,
  DefaultError,
} from './runtime-client'

export type { RuntimeClient } from './runtime-client'

export {
  extractServerProps,
  extractServerPropsWithCache,
  extractMetadata,
  extractStaticParams,
  hasServerSideDataFetching,
  clearPropsCache,
  clearPropsCacheForComponent,
} from './router'

export type {
  ServerPropsResult,
  MetadataResult,
  StaticParamsResult,
} from './router'
