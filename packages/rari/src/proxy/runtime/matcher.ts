import type { ProxyConfig, ProxyMatcher, ProxyRuleCondition, RariRequest } from '@/proxy/http/types'
import { normalizePath } from '@/shared/utils/path'

const PROXY_NORMALIZE_OPTIONS = { collapseSlashes: true, ensureLeadingSlash: false } as const
const PARAM_OR_WILDCARD_RE = /:(\w+)([?*+]?)|\*/g
const URL_PATTERN_LITERAL_ESCAPE_RE = /[\\+?(){}]/g
const PATHNAME_RESERVED_RE = /[?#]/g

interface CompiledProxyPattern {
  readonly urlPattern: URLPattern
  readonly paramNames: readonly string[]
}

const compiledPatterns = new Map<string, CompiledProxyPattern>()

export function clearCompiledProxyPatterns(): void {
  compiledPatterns.clear()
}

function encodePathnameReserved(value: string): string {
  return value.replace(PATHNAME_RESERVED_RE, char => encodeURIComponent(char))
}

function escapeUrlPatternLiteral(literal: string): string {
  return encodePathnameReserved(literal).replace(URL_PATTERN_LITERAL_ESCAPE_RE, '\\$&')
}

function toUrlPatternPathname(pattern: string): {
  readonly pathname: string
  readonly paramNames: readonly string[]
} {
  const paramNames: string[] = []
  let pathname = ''
  let lastIndex = 0

  for (const match of pattern.matchAll(PARAM_OR_WILDCARD_RE)) {
    const index = match.index
    pathname += escapeUrlPatternLiteral(pattern.slice(lastIndex, index))

    const token = match[0]
    if (token === '*') {
      pathname += '*'
    } else {
      const name = match[1]
      const modifier = match[2]
      paramNames.push(name)
      pathname += `:${name}${modifier}`
    }

    lastIndex = index + token.length
  }

  pathname += escapeUrlPatternLiteral(pattern.slice(lastIndex))
  return { pathname, paramNames }
}

function compileProxyPattern(pattern: string): CompiledProxyPattern {
  const normalizedPattern = normalizePath(pattern, PROXY_NORMALIZE_OPTIONS)
  const cached = compiledPatterns.get(normalizedPattern)
  if (cached) return cached

  const { pathname, paramNames } = toUrlPatternPathname(normalizedPattern)
  const compiled: CompiledProxyPattern = {
    urlPattern: new URLPattern({ pathname }),
    paramNames,
  }
  compiledPatterns.set(normalizedPattern, compiled)
  return compiled
}

function execProxyPattern(pathname: string, pattern: string): URLPatternResult | null {
  const normalizedPath = encodePathnameReserved(normalizePath(pathname, PROXY_NORMALIZE_OPTIONS))
  return compileProxyPattern(pattern).urlPattern.exec({ pathname: normalizedPath })
}

/* v8 ignore start - requires complex RariRequest mocking */
function checkHeaderCondition(request: RariRequest, key: string): string | null {
  return request.headers.get(key)
}

function checkQueryCondition(request: RariRequest, key: string): string | null {
  return request.rariUrl.searchParams.get(key)
}

function checkCookieCondition(request: RariRequest, key: string): string | null {
  const cookie = request.cookies.get(key)
  return cookie ? cookie.value : null
}

function checkHostCondition(request: RariRequest, key: string): string | null {
  return request.rariUrl.hostname === key ? request.rariUrl.hostname : null
}

function getConditionActualValue(
  request: RariRequest,
  condition: ProxyRuleCondition,
): string | null {
  switch (condition.type) {
    case 'header':
      return checkHeaderCondition(request, condition.key)
    case 'query':
      return checkQueryCondition(request, condition.key)
    case 'cookie':
      return checkCookieCondition(request, condition.key)
    case 'host':
      return checkHostCondition(request, condition.key)
    default:
      throw new Error(`Unknown condition type: ${(condition as { type: string }).type}`)
  }
}

function matchesHasCondition(request: RariRequest, condition: ProxyRuleCondition): boolean {
  const actualValue = getConditionActualValue(request, condition)

  if (actualValue === null) return false
  if (condition.value !== undefined && actualValue !== condition.value) return false

  return true
}

function matchesMissingCondition(request: RariRequest, condition: ProxyRuleCondition): boolean {
  const actualValue = getConditionActualValue(request, condition)

  if (actualValue === null) return true
  if (condition.value === undefined) return false

  return actualValue !== condition.value
}

function matchesConditions(request: RariRequest, matcher: ProxyMatcher): boolean {
  if (matcher.has) {
    for (const condition of matcher.has) {
      if (!matchesHasCondition(request, condition)) return false
    }
  }

  if (matcher.missing) {
    for (const condition of matcher.missing) {
      if (!matchesMissingCondition(request, condition)) return false
    }
  }

  return true
}
/* v8 ignore stop */

export function matchesPattern(pathname: string, pattern: string): boolean {
  return execProxyPattern(pathname, pattern) != null
}

/* v8 ignore start - requires complex RariRequest mocking */
function matchesSingleMatcher(
  request: RariRequest,
  pathname: string,
  matcher: string | ProxyMatcher,
): boolean {
  if (typeof matcher === 'string') return matchesPattern(pathname, matcher)

  if (!matchesPattern(pathname, matcher.source)) return false

  return matchesConditions(request, matcher)
}

export function shouldRunProxy(request: RariRequest, config?: ProxyConfig): boolean {
  const matcher = config?.matcher
  if (matcher == null || matcher === '' || (Array.isArray(matcher) && matcher.length === 0))
    return true

  const pathname = request.rariUrl.pathname
  const matchers: ReadonlyArray<string | ProxyMatcher> = Array.isArray(matcher)
    ? matcher
    : [matcher]

  return matchers.some(m => matchesSingleMatcher(request, pathname, m))
}
/* v8 ignore stop */

export function extractParams(pathname: string, pattern: string): Record<string, string> | null {
  const compiled = compileProxyPattern(pattern)
  const result = execProxyPattern(pathname, pattern)
  if (result == null) return null

  const params: Record<string, string> = {}
  const groups = result.pathname.groups

  for (const name of compiled.paramNames) {
    params[name] = groups[name] ?? ''
  }

  return params
}
