/* eslint-disable react-refresh/only-export-components */
import type { ComponentType, ReactNode } from 'react'
import type {
  RouterContext as IRouterContext,
  NavigationOptions,
  NavigationState,
  RouteMatch,
  RouterProviderProps,
  UseNavigationReturn,
  UseRouteReturn,
  UseRouterReturn,
} from './types'
import React, {
  createContext,
  use,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from 'react'

import {
  buildUrl,
  findMatchingRoute,
  isPathActive,
  matchRoute,
  normalizePathname,
  parseUrl,
} from './utils'

const RouterContext = createContext<IRouterContext | null>(null)

const DEFAULT_CONFIG = {}
const DEFAULT_ROUTES: any[] = []

export function RouterProvider({
  config = DEFAULT_CONFIG,
  routes = DEFAULT_ROUTES,
  children,
}: RouterProviderProps) {
  const mergedConfig = useMemo(
    () => ({
      basePath: '',
      useHash: false,
      caseSensitive: false,
      ...config,
    }),
    [config],
  )

  const resolveRoute = useCallback(
    (url: string): RouteMatch | null => {
      const { pathname, search, hash, searchParams } = parseUrl(url)
      const normalizedPathname = normalizePathname(pathname)

      const match = findMatchingRoute(normalizedPathname, routes)

      if (match) {
        const enhancedMatch = {
          ...match,
          searchParams,
          search,
          hash,
        }

        enhancedMatch.childMatch = findDeepestChildMatch(
          enhancedMatch,
          normalizedPathname,
        )
        return enhancedMatch
      }

      return null
    },
    [routes],
  )

  const [routerState, setRouterState] = useState<{
    currentRoute: RouteMatch | null
    isReady: boolean
  }>(() => {
    if (typeof window === 'undefined') {
      return { currentRoute: null, isReady: false }
    }

    const url = mergedConfig.useHash
      ? window.location.hash.slice(1) || '/'
      : window.location.pathname
        + window.location.search
        + window.location.hash

    let initialRoute
    try {
      initialRoute = resolveRoute(url)
    }
    catch (error) {
      console.error('Error resolving initial route:', error)
      initialRoute = null
    }

    const isReady = true
    return { currentRoute: initialRoute, isReady }
  })

  const currentRoute = routerState.currentRoute
  const isReady = routerState.isReady

  const updateCurrentRoute = useCallback(
    (url: string) => {
      const route = resolveRoute(url)
      setRouterState({ currentRoute: route, isReady: true })
    },
    [resolveRoute],
  )

  useEffect(() => {
    const handleLocationChange = () => {
      const url = mergedConfig.useHash
        ? window.location.hash.slice(1) || '/'
        : window.location.pathname
          + window.location.search
          + window.location.hash

      updateCurrentRoute(url)
    }

    const handlePopState = () => {
      handleLocationChange()
    }

    window.addEventListener('popstate', handlePopState)

    if (mergedConfig.useHash) {
      window.addEventListener('hashchange', handleLocationChange)
    }

    return () => {
      window.removeEventListener('popstate', handlePopState)
      if (mergedConfig.useHash) {
        window.removeEventListener('hashchange', handleLocationChange)
      }
    }
  }, [mergedConfig.useHash, routes, updateCurrentRoute])

  const navigate = useCallback(
    (path: string, options: NavigationOptions = DEFAULT_CONFIG) => {
      const { replace = false, state, scroll = true } = options

      try {
        const url = mergedConfig.basePath + path

        if (mergedConfig.useHash) {
          if (replace) {
            window.location.replace(`#${path}`)
          }
          else {
            window.location.hash = path
          }
        }
        else {
          if (replace) {
            window.history.replaceState(state, '', url)
          }
          else {
            window.history.pushState(state, '', url)
          }
        }

        updateCurrentRoute(path)

        if (scroll) {
          if (options.scrollPosition) {
            window.scrollTo(options.scrollPosition.x, options.scrollPosition.y)
          }
          else {
            window.scrollTo(0, 0)
          }
        }
      }
      catch (error) {
        console.error('Navigation failed:', error)
      }
    },
    [mergedConfig.basePath, mergedConfig.useHash, updateCurrentRoute],
  )

  const back = useCallback(() => {
    window.history.back()
  }, [])

  const forward = useCallback(() => {
    window.history.forward()
  }, [])

  const replace = useCallback(
    (
      path: string,
      options: Omit<NavigationOptions, 'replace'> = DEFAULT_CONFIG,
    ) => {
      navigate(path, { ...options, replace: true })
    },
    [navigate],
  )

  const isActive = useCallback(
    (path: string, exact = false) => {
      if (!currentRoute)
        return false
      return isPathActive(currentRoute.pathname, path, exact)
    },
    [currentRoute],
  )

  const contextValue: IRouterContext = useMemo(() => {
    return {
      currentRoute,
      routes,
      navigate,
      back,
      forward,
      replace,
      isActive,
      config: mergedConfig,
      isReady,
    }
  }, [
    currentRoute,
    routes,
    navigate,
    back,
    forward,
    replace,
    isActive,
    mergedConfig,
    isReady,
  ])

  return <RouterContext value={contextValue}>{children}</RouterContext>
}

export function useRouter(): UseRouterReturn {
  const context = use(RouterContext)
  if (!context) {
    throw new Error('useRouter must be used within a RouterProvider')
  }
  return context
}

export function useNavigation(): UseNavigationReturn {
  const { navigate, back, forward, replace } = useRouter()
  const [navigationState] = useState<NavigationState>({
    isNavigating: false,
  })

  return {
    ...navigationState,
    navigate,
    back,
    forward,
    replace,
  }
}

export function useRoute(): UseRouteReturn {
  const { currentRoute } = useRouter()
  const [isLoading] = useState(false)
  const [error] = useState<Error | null>(null)

  return {
    route: currentRoute,
    params: currentRoute?.params || {},
    searchParams: currentRoute?.searchParams || {},
    isLoading,
    error,
  }
}

export function useParams() {
  const { currentRoute } = useRouter()
  return currentRoute?.params || {}
}

export function useSearchParams() {
  const { currentRoute, navigate } = useRouter()

  const searchParams = currentRoute?.searchParams || {}

  const setSearchParams = useCallback(
    (
      params:
        | Record<string, string | string[]>
        | ((
          prev: Record<string, string | string[]>,
        ) => Record<string, string | string[]>),
      options: NavigationOptions = DEFAULT_CONFIG,
    ) => {
      const newParams
        = typeof params === 'function' ? params(searchParams) : params
      const url = buildUrl(currentRoute?.pathname || '/', newParams)
      navigate(url, options)
    },
    [searchParams, currentRoute?.pathname, navigate],
  )

  return [searchParams, setSearchParams] as const
}

export function usePathname() {
  const { currentRoute } = useRouter()
  return currentRoute?.pathname || '/'
}

export function withRouter<P extends object>(
  Component: ComponentType<P & { router: IRouterContext }>,
) {
  const WithRouterComponent = (props: P) => {
    const router = useRouter()
    return <Component {...props} router={router} />
  }

  WithRouterComponent.displayName = `withRouter(${Component.displayName || Component.name})`

  return WithRouterComponent
}

export function RouteComponent({
  path,
  component: Component,
  exact = false,
  render,
  children,
}: {
  path: string
  component?: ComponentType<any>
  exact?: boolean
  render?: (props: { match: RouteMatch | null }) => ReactNode
  children?: ReactNode
}) {
  const { currentRoute } = useRouter()

  const match = useMemo(() => {
    if (!currentRoute)
      return null

    const isMatch = exact
      ? currentRoute.pathname === path
      : isPathActive(currentRoute.pathname, path, exact)

    return isMatch ? currentRoute : null
  }, [currentRoute, path, exact])

  if (!match) {
    return null
  }

  if (Component) {
    return <Component {...match.params} searchParams={match.searchParams} />
  }

  if (render) {
    return render({ match })
  }

  return children as React.ReactElement
}

export function Routes({ children: _children }: { children: ReactNode }) {
  const { currentRoute } = useRouter()

  if (!currentRoute) {
    return null
  }

  return <RouteRenderer routeMatch={currentRoute} />
}

export function Outlet() {
  const { currentRoute } = useRouter()

  if (!currentRoute) {
    return null
  }

  const childRoute = findChildRouteForCurrentLevel(currentRoute)

  if (!childRoute) {
    return null
  }

  const ChildComponent = childRoute.route.component

  if (!ChildComponent) {
    return null
  }

  return (
    <ChildComponent
      {...childRoute.params}
      searchParams={childRoute.searchParams}
    />
  )
}

function findChildRouteForCurrentLevel(
  currentRoute: RouteMatch,
): RouteMatch | null {
  if (currentRoute.childMatch) {
    return currentRoute.childMatch
  }

  if (currentRoute.route.children) {
    const indexRoute = currentRoute.route.children.find(
      child => child.isIndex,
    )
    if (indexRoute) {
      return {
        route: indexRoute,
        params: currentRoute.params,
        searchParams: currentRoute.searchParams,
        pathname: currentRoute.pathname,
        search: currentRoute.search,
        hash: currentRoute.hash,
      }
    }
  }

  return null
}

export function RouteRenderer({ routeMatch }: { routeMatch: RouteMatch }) {
  if (!routeMatch.layouts || routeMatch.layouts.length === 0) {
    const Component = routeMatch.route.component
    return Component
      ? (
          <Component
            {...routeMatch.params}
            searchParams={routeMatch.searchParams}
          />
        )
      : null
  }

  let rendered: ReactNode = null
  const Component = routeMatch.route.component

  if (Component) {
    rendered = (
      <Component
        {...routeMatch.params}
        searchParams={routeMatch.searchParams}
      />
    )
  }

  for (let i = routeMatch.layouts.length - 1; i >= 0; i--) {
    const layout = routeMatch.layouts[i]
    const LayoutComponent = layout.component

    if (LayoutComponent) {
      rendered = (
        <LayoutComponent route={routeMatch}>{rendered}</LayoutComponent>
      )
    }
  }

  return <>{rendered}</>
}

export function Navigate({
  to,
  replace = false,
  state,
}: {
  to: string
  replace?: boolean
  state?: any
}) {
  const { navigate } = useRouter()

  useEffect(() => {
    navigate(to, { replace, state })
  }, [navigate, to, replace, state])

  return null
}

export default RouterProvider
export { RouterContext }

function findDeepestChildMatch(
  routeMatch: RouteMatch,
  pathname: string,
): RouteMatch | null {
  if (!routeMatch.route.children || routeMatch.route.children.length === 0) {
    return null
  }

  const routePathSegments = routeMatch.route.path.split('/').filter(Boolean)
  const pathSegments = pathname.split('/').filter(Boolean)

  if (routePathSegments.length >= pathSegments.length) {
    return null
  }

  const remainingPath = `/${pathSegments.slice(routePathSegments.length).join('/')}`

  for (const childRoute of routeMatch.route.children) {
    const childMatch = matchRoute(remainingPath, childRoute)
    if (childMatch) {
      const deeperChild = findDeepestChildMatch(childMatch, pathname)
      return deeperChild || childMatch
    }
  }

  return null
}

export function Link({
  to,
  children,
  className,
  activeClassName,
  exact = false,
  onClick,
  ...props
}: {
  to: string
  children: ReactNode
  className?: string
  activeClassName?: string
  exact?: boolean
  onClick?: (event: React.MouseEvent<HTMLAnchorElement>) => void
  [key: string]: any
}) {
  const { navigate, isActive } = useRouter()

  const handleClick = useCallback(
    (event: React.MouseEvent<HTMLAnchorElement>) => {
      event.preventDefault()

      if (onClick) {
        onClick(event)
      }

      navigate(to)
    },
    [navigate, onClick, to],
  )

  const isLinkActive = isActive(to, exact)
  const finalClassName = [className, isLinkActive && activeClassName]
    .filter(Boolean)
    .join(' ')

  return (
    <a href={to} className={finalClassName} onClick={handleClick} {...props}>
      {children}
    </a>
  )
}
