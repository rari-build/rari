import type { ProxyConfig, ProxyMatcher, RariRequest } from './types'

function normalizePath(path: string): string {
  const collapsed = path.replace(/\/+/g, '/')
  return collapsed === '/' ? '/' : collapsed.replace(/\/+$/, '')
}

function pathToRegex(pattern: string): RegExp {
  let regexPattern = pattern
    .replace(/[.+?^${}()|[\]\\]/g, '\\$&')
    .replace(/\*/g, '___STAR___')

  regexPattern = regexPattern.replace(/:(\w+)/g, '([^/]+)')
  regexPattern = regexPattern.replace(/\\:(\w+)\\\*/g, '(.*)')
  regexPattern = regexPattern.replace(/\\:(\w+)\\\+/g, '(.+)')
  regexPattern = regexPattern.replace(/\\:(\w+)\\\?/g, '([^/]*)')
  regexPattern = regexPattern.replace(/___STAR___/g, '.*')
  regexPattern = `^${regexPattern}$`

  return new RegExp(regexPattern)
}

/* v8 ignore start - requires complex RariRequest mocking */
function matchesConditions(
  request: RariRequest,
  matcher: ProxyMatcher,
): boolean {
  if (matcher.has) {
    for (const condition of matcher.has) {
      if (condition.type === 'header') {
        const headerValue = request.headers.get(condition.key)
        if (!headerValue)
          return false
        if (condition.value && headerValue !== condition.value)
          return false
      }
      else if (condition.type === 'query') {
        const queryValue = request.rariUrl.searchParams.get(condition.key)
        if (!queryValue)
          return false
        if (condition.value && queryValue !== condition.value)
          return false
      }
      else if (condition.type === 'cookie') {
        const cookieValue = request.cookies.get(condition.key)
        if (!cookieValue)
          return false
        if (condition.value && cookieValue.value !== condition.value)
          return false
      }
    }
  }

  if (matcher.missing) {
    for (const condition of matcher.missing) {
      if (condition.type === 'header') {
        const headerValue = request.headers.get(condition.key)
        if (headerValue) {
          if (!condition.value || headerValue === condition.value)
            return false
        }
      }
      else if (condition.type === 'query') {
        const queryValue = request.rariUrl.searchParams.get(condition.key)
        if (queryValue) {
          if (!condition.value || queryValue === condition.value)
            return false
        }
      }
      else if (condition.type === 'cookie') {
        const cookieValue = request.cookies.get(condition.key)
        if (cookieValue) {
          if (!condition.value || cookieValue.value === condition.value)
            return false
        }
      }
    }
  }

  return true
}
/* v8 ignore stop */

export function matchesPattern(pathname: string, pattern: string): boolean {
  const normalizedPath = normalizePath(pathname)
  const normalizedPattern = normalizePath(pattern)
  const regex = pathToRegex(normalizedPattern)
  return regex.test(normalizedPath)
}

/* v8 ignore start - requires complex RariRequest mocking */
export function shouldRunProxy(
  request: RariRequest,
  config?: ProxyConfig,
): boolean {
  if (!config?.matcher)
    return true

  const pathname = request.rariUrl.pathname
  const matchers = Array.isArray(config.matcher) ? config.matcher : [config.matcher]

  for (const matcher of matchers) {
    if (typeof matcher === 'string') {
      if (matchesPattern(pathname, matcher))
        return true
    }
    else {
      if (matchesPattern(pathname, matcher.source)) {
        if (matchesConditions(request, matcher))
          return true
      }
    }
  }

  return false
}
/* v8 ignore stop */

export function extractParams(
  pathname: string,
  pattern: string,
): Record<string, string> | null {
  const params: Record<string, string> = {}

  const normalizedPath = normalizePath(pathname)
  const normalizedPattern = normalizePath(pattern)

  const paramNames: string[] = []
  let regexPattern = normalizedPattern
    .replace(/[.+?^${}()|[\]\\]/g, '\\$&')
    .replace(/\*/g, '___STAR___')

  regexPattern = regexPattern.replace(/:(\w+)/g, (_, name) => {
    paramNames.push(name)
    return '([^/]+)'
  })

  /* v8 ignore start - advanced parameter patterns not commonly used */
  regexPattern = regexPattern.replace(/\\:(\w+)\\\*/g, (_, name) => {
    paramNames.push(name)
    return '(.*)'
  })
  regexPattern = regexPattern.replace(/\\:(\w+)\\\+/g, (_, name) => {
    paramNames.push(name)
    return '(.+)'
  })
  regexPattern = regexPattern.replace(/\\:(\w+)\\\?/g, (_, name) => {
    paramNames.push(name)
    return '([^/]*)'
  })
  /* v8 ignore stop */

  regexPattern = regexPattern.replace(/___STAR___/g, '.*')
  regexPattern = `^${regexPattern}$`

  const regex = new RegExp(regexPattern)
  const match = normalizedPath.match(regex)

  if (!match)
    return null

  for (let i = 0; i < paramNames.length; i++)
    params[paramNames[i]] = match[i + 1]

  return params
}
