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

export { createHttpRuntimeClient, HttpRuntimeClient } from './runtime-client'

export type { RuntimeClient } from './runtime-client'
