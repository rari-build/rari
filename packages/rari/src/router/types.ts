import type { ComponentType, ReactNode } from 'react'
import type { PageCacheConfig } from './cache'

export interface Route {
  path: string
  filePath: string
  component: ComponentType<any> | null
  isDynamic: boolean
  paramNames: string[]
  children?: Route[]
  parent?: Route
  isLayout?: boolean
  isIndex?: boolean
  meta?: RouteMeta
}

export interface RouteMeta {
  title?: string
  description?: string
  requiresAuth?: boolean
  cacheConfig?: PageCacheConfig
  [key: string]: any
}

export interface RouteParams {
  [key: string]: string | string[]
}

export interface SearchParams {
  [key: string]: string | string[]
}

export interface RouteMatch {
  route: Route
  params: RouteParams
  searchParams: SearchParams
  pathname: string
  search: string
  hash: string
  parentMatches?: RouteMatch[]
  childMatch?: RouteMatch | null
  layouts?: Route[]
}

export interface NavigationOptions {
  replace?: boolean
  state?: any
  scroll?: boolean
  scrollPosition?: { x: number, y: number }
}

export interface RouterConfig {
  basePath?: string
  useHash?: boolean
  caseSensitive?: boolean
  notFoundComponent?: ComponentType<any>
  loadingComponent?: ComponentType<any>
  errorComponent?: ComponentType<{ error: Error, retry: () => void }>
}

export interface PageProps {
  params: RouteParams
  searchParams: SearchParams
  meta?: RouteMeta
}

export type PageComponent<P = Record<string, unknown>> = ComponentType<
  PageProps & P
>

export interface RouterContext {
  currentRoute: RouteMatch | null
  routes: Route[]
  navigate: (path: string, options?: NavigationOptions) => void
  back: () => void
  forward: () => void
  replace: (path: string, options?: Omit<NavigationOptions, 'replace'>) => void
  isActive: (path: string, exact?: boolean) => boolean
  config: RouterConfig
  isReady: boolean
}

export interface NavigationState {
  isNavigating: boolean
  destination?: string
  error?: Error
}

export interface RouteGenerationOptions {
  pagesDir: string
  extensions: string[]
  transforms?: RouteTransform[]
}

export type RouteTransform = (route: Route) => Route

export interface LinkProps {
  to: string
  options?: NavigationOptions
  children: ReactNode
  className?: string
  activeClassName?: string
  exact?: boolean
  onClick?: (event: React.MouseEvent<HTMLAnchorElement>) => void
  [key: string]: any
}

export interface NavLinkProps extends Omit<LinkProps, 'activeClassName'> {
  activeClassName?: string
  activeStyle?: React.CSSProperties
  exact?: boolean
  isActive?: (pathname: string, to: string) => boolean
  disabled?: boolean
}

export interface RouterProviderProps {
  config?: RouterConfig
  routes?: Route[]
  children: ReactNode
}

export type RouteMatcher = (path: string, routes: Route[]) => RouteMatch | null

export type RouteGenerator = (
  options: RouteGenerationOptions,
) => Promise<Route[]>

export interface UseRouterReturn extends RouterContext {}

export interface UseNavigationReturn extends NavigationState {
  navigate: RouterContext['navigate']
  back: RouterContext['back']
  forward: RouterContext['forward']
  replace: RouterContext['replace']
}

export interface UseRouteReturn {
  route: RouteMatch | null
  params: RouteParams
  searchParams: SearchParams
  isLoading: boolean
  error: Error | null
}

export interface FileRouteInfo {
  filePath: string
  routePath: string
  isDynamic: boolean
  paramNames: string[]
  isIndex: boolean
  isLayout: boolean
  isNotFound: boolean
}

export interface LayoutProps {
  children: ReactNode
  route: RouteMatch
}

export type LayoutComponent = ComponentType<LayoutProps>

export interface ErrorBoundaryProps {
  error: Error
  retry: () => void
  route: RouteMatch
}

export interface LoadingProps {
  route: Route
  progress?: number
}
