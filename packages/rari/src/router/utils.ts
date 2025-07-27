import type {
  FileRouteInfo,
  Route,
  RouteMatch,
  RouteParams,
  SearchParams,
} from './types'

export function filePathToRoutePath(filePath: string): string {
  let routePath = filePath
    .replace(/^pages\//, '')
    .replace(/\.(tsx?|jsx?)$/, '')

  if (routePath === 'index') {
    routePath = '/'
  }
  else if (routePath.endsWith('/index')) {
    routePath = routePath.replace(/\/index$/, '')
  }

  routePath = routePath.replace(/\[([^\]]+)\]/g, (match, param) => {
    if (param.startsWith('...')) {
      return `:${param.slice(3)}*`
    }
    return `:${param}`
  })

  if (!routePath.startsWith('/')) {
    routePath = `/${routePath}`
  }

  return routePath
}

export function extractParamNames(routePath: string): string[] {
  const params: string[] = []
  const paramRegex = /:([^/]+)/g

  const matches = Array.from(routePath.matchAll(paramRegex))

  for (const match of matches) {
    let paramName = match[1]

    if (paramName.endsWith('*')) {
      paramName = paramName.slice(0, -1)
    }

    params.push(paramName)
  }

  return params
}

export function isDynamicRoute(routePath: string): boolean {
  return routePath.includes(':')
}

export function routePathToRegex(routePath: string): RegExp {
  let pattern = routePath.replace(/[.+?^${}()|[\]\\]/g, '\\$&')

  pattern = pattern.replace(/:([^/]+)/g, (match, paramName) => {
    if (paramName.endsWith('*')) {
      return '(.*)'
    }
    return '([^/]+)'
  })

  pattern = `^${pattern}$`

  return new RegExp(pattern)
}

export function matchRoute(pathname: string, route: Route): RouteMatch | null {
  const regex = routePathToRegex(route.path)
  const match = pathname.match(regex)

  if (!match) {
    return null
  }

  const params: RouteParams = {}
  const paramNames = route.paramNames || []

  for (let i = 0; i < paramNames.length; i++) {
    const paramName = paramNames[i]
    const paramValue = match[i + 1]

    if (paramValue !== undefined) {
      if (route.path.includes(`:${paramName}*`)) {
        const segments = paramValue.split('/').filter(Boolean)
        params[paramName] = segments
      }
      else {
        params[paramName] = decodeURIComponent(paramValue)
      }
    }
  }

  return {
    route,
    params,
    searchParams: {},
    pathname,
    search: '',
    hash: '',
  }
}

export function findMatchingRoute(
  pathname: string,
  routes: Route[],
): RouteMatch | null {
  const routeHierarchy = buildRouteHierarchy(routes)

  const match = findNestedRouteMatch(pathname, routeHierarchy)

  if (!match) {
    return null
  }

  return {
    route: match.route,
    params: match.params,
    searchParams: {},
    pathname,
    search: '',
    hash: '',
    parentMatches: match.parentMatches,
    layouts: match.layouts,
  }
}

interface NestedRouteMatch {
  route: Route
  params: RouteParams
  parentMatches: RouteMatch[]
  layouts: Route[]
}

function buildRouteHierarchy(routes: Route[]): Map<string, Route> {
  const routeMap = new Map<string, Route>()

  for (const route of routes) {
    routeMap.set(route.path, route)
  }

  return routeMap
}

function findNestedRouteMatch(
  pathname: string,
  routeHierarchy: Map<string, Route>,
): NestedRouteMatch | null {
  let bestMatch: NestedRouteMatch | null = null
  let highestPriority = -Infinity

  for (const route of routeHierarchy.values()) {
    const match = matchRouteWithHierarchy(pathname, route, routeHierarchy)

    if (match) {
      const priority = getRoutePriority(route)
      if (priority > highestPriority) {
        bestMatch = match
        highestPriority = priority
      }
    }
  }

  return bestMatch
}

function matchRouteWithHierarchy(
  pathname: string,
  route: Route,
  routeHierarchy: Map<string, Route>,
): NestedRouteMatch | null {
  const routeMatch = matchRoute(pathname, route)

  if (!routeMatch) {
    return null
  }

  const parentMatches: RouteMatch[] = []
  const layouts: Route[] = []

  let currentRoute = route.parent
  while (currentRoute) {
    parentMatches.unshift({
      route: currentRoute,
      params: {},
      searchParams: {},
      pathname: currentRoute.path,
      search: '',
      hash: '',
    })

    const layoutRoute = findLayoutForRoute(currentRoute, routeHierarchy)
    if (layoutRoute) {
      layouts.unshift(layoutRoute)
    }

    currentRoute = currentRoute.parent
  }

  const matchedRouteLayout = findLayoutForRoute(route, routeHierarchy)
  if (matchedRouteLayout) {
    layouts.push(matchedRouteLayout)
  }

  return {
    route,
    params: routeMatch.params,
    parentMatches,
    layouts,
  }
}

function findLayoutForRoute(route: Route, routeHierarchy: Map<string, Route>): Route | null {
  const routeDir = route.filePath.split('/').slice(0, -1).join('/')
  const possibleLayoutPaths = [
    `${routeDir}/layout.tsx`,
    `${routeDir}/layout.jsx`,
    `${routeDir}/_layout.tsx`,
    `${routeDir}/_layout.jsx`,
  ]

  for (const layoutPath of possibleLayoutPaths) {
    for (const candidateRoute of routeHierarchy.values()) {
      if (candidateRoute.filePath === layoutPath && candidateRoute.isLayout) {
        return candidateRoute
      }
    }
  }

  return null
}

export function parseSearchParams(search: string): SearchParams {
  const params: SearchParams = {}

  if (!search || search === '?') {
    return params
  }

  const searchParams = new URLSearchParams(
    search.startsWith('?') ? search.slice(1) : search,
  )

  for (const [key, value] of searchParams.entries()) {
    if (params[key]) {
      if (Array.isArray(params[key])) {
        (params[key] as string[]).push(value)
      }
      else {
        params[key] = [params[key] as string, value]
      }
    }
    else {
      params[key] = value
    }
  }

  return params
}

export function buildSearchString(params: SearchParams): string {
  const searchParams = new URLSearchParams()

  for (const [key, value] of Object.entries(params)) {
    if (Array.isArray(value)) {
      value.forEach(v => searchParams.append(key, v))
    }
    else {
      searchParams.set(key, value)
    }
  }

  const search = searchParams.toString()
  return search ? `?${search}` : ''
}

export function parseUrl(url: string): {
  pathname: string
  search: string
  hash: string
  searchParams: SearchParams
} {
  try {
    const parsed = new URL(url, 'http://localhost')

    return {
      pathname: parsed.pathname,
      search: parsed.search,
      hash: parsed.hash,
      searchParams: parseSearchParams(parsed.search),
    }
  }
  catch {
    const [pathname, rest] = url.split('?', 2)
    const [search, hash] = rest ? rest.split('#', 2) : ['', '']

    return {
      pathname: pathname || '/',
      search: search ? `?${search}` : '',
      hash: hash ? `#${hash}` : '',
      searchParams: parseSearchParams(search || ''),
    }
  }
}

export function buildUrl(
  pathname: string,
  searchParams?: SearchParams,
  hash?: string,
): string {
  let url = pathname

  if (searchParams) {
    const search = buildSearchString(searchParams)
    if (search) {
      url += search
    }
  }

  if (hash) {
    url += hash.startsWith('#') ? hash : `#${hash}`
  }

  return url
}

export function analyzeFilePath(filePath: string): FileRouteInfo {
  const routePath = filePathToRoutePath(filePath)
  const isDynamic = isDynamicRoute(routePath)
  const paramNames = extractParamNames(routePath)

  const fileName = filePath.split('/').pop() || ''

  return {
    filePath,
    routePath,
    isDynamic,
    paramNames,
    isIndex:
      fileName === 'index.tsx'
      || fileName === 'index.jsx'
      || fileName === 'index.ts'
      || fileName === 'index.js'
      || filePath.endsWith('/index.tsx')
      || filePath.endsWith('/index.jsx')
      || filePath === 'pages/index.tsx'
      || filePath === 'pages/index.jsx',
    isLayout:
      fileName === 'layout.tsx'
      || fileName === 'layout.jsx'
      || fileName === '_layout.tsx'
      || fileName === '_layout.jsx'
      || filePath.includes('/layout.')
      || filePath.includes('/_layout.'),
    isNotFound: filePath.includes('404.') || filePath.includes('_error.'),
  }
}

export function sortRoutesBySpecificity(routes: Route[]): Route[] {
  return [...routes].sort((a, b) => {
    if (!a.isDynamic && b.isDynamic)
      return -1
    if (a.isDynamic && !b.isDynamic)
      return 1

    const aSegments = a.path.split('/').length
    const bSegments = b.path.split('/').length

    if (aSegments !== bSegments) {
      return aSegments - bSegments
    }

    const aParamCount = a.paramNames?.length || 0
    const bParamCount = b.paramNames?.length || 0

    if (aParamCount !== bParamCount) {
      return aParamCount - bParamCount
    }

    const aHasCatchAll = a.path.includes('*')
    const bHasCatchAll = b.path.includes('*')

    if (aHasCatchAll && !bHasCatchAll)
      return 1
    if (!aHasCatchAll && bHasCatchAll)
      return -1

    return 0
  })
}

export function routePathsEqual(a: string, b: string): boolean {
  return a === b
}

export function isPathActive(
  pathname: string,
  routePath: string,
  exact: boolean = false,
): boolean {
  if (exact) {
    return pathname === routePath
  }

  if (routePath === '/') {
    return pathname === '/'
  }

  return pathname === routePath || pathname.startsWith(`${routePath}/`)
}

export function normalizePathname(pathname: string): string {
  if (pathname === '/' || pathname === '') {
    return '/'
  }

  return pathname.replace(/\/+$/, '')
}

export function joinPaths(...segments: string[]): string {
  return (
    segments
      .filter(Boolean)
      .join('/')
      .replace(/\/+/g, '/')
      .replace(/\/$/, '') || '/'
  )
}

export function getParentPath(pathname: string): string {
  const segments = pathname.split('/').filter(Boolean)
  if (segments.length <= 1) {
    return '/'
  }

  return `/${segments.slice(0, -1).join('/')}`
}

export function getParentPaths(pathname: string): string[] {
  const segments = pathname.split('/').filter(Boolean)
  const parents: string[] = ['/']

  for (let i = 1; i < segments.length; i++) {
    parents.push(`/${segments.slice(0, i).join('/')}`)
  }

  return parents
}

export function getRoutePriority(route: Route): number {
  let priority = 0

  if (!route.isDynamic) {
    priority += 1000
  }

  const paramCount = route.paramNames?.length || 0
  priority -= paramCount * 100

  if (route.path.includes('*')) {
    priority -= 500
  }

  const segmentCount = route.path.split('/').length
  priority += segmentCount * 10

  return priority
}
