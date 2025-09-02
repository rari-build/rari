export { Link } from './router'

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
  ErrorBoundaryProps,
  FileRouteInfo,
  LayoutProps,
  LinkProps,
  LoadingProps,
  NavigationOptions,
  NavigationState,
  PageComponent,
  PageProps,
  RouteGenerationOptions,
  RouteMatch,
  RouteMeta,
  RouteParams,
  RouterConfig,
  RouterContext,
  RouterProviderProps,
  Route as RouteType,
  SearchParams,
} from './router'

export type { CacheConfig, PageCacheConfig } from './router/cache'

export {
  convertFilePatternToRoutePattern,
  createRouteManifest,
  FileRouteGenerator,
  generateFileRoutes,
  loadRouteManifest,
  validateRoutes,
  watchFileRoutes,
} from './router/file-routes'

export { getRoutePriority } from './router/utils'

export { rariRouter } from './router/vite-plugin'

export { createHttpRuntimeClient, HttpRuntimeClient } from './runtime-client'

export type { RuntimeClient } from './runtime-client'

export { defineRariConfig, defineRariOptions, rari } from './vite'
