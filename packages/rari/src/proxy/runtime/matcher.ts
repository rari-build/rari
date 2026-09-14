import type { ProxyConfig, ProxyMatcher, ProxyRuleCondition, RariRequest } from '@/proxy/http/types'
import { MULTIPLE_SLASHES_REGEX, PATH_TRAILING_SLASH_REGEX } from '@/shared/regex-constants'

const ESCAPE_CHARS_REGEX = /[.+?^${}()|[\]\\]/g
const ASTERISK_REGEX = /\*/g
const PARAM_TOKEN_REGEX = /\/:(\w+)\*|\/:(\w+)\?|:(\w+)\*|:(\w+)\+|:(\w+)\?|:(\w+)/g

const PLACEHOLDER_REPLACEMENTS: ReadonlyArray<readonly [RegExp, string]> = [
  [/___PARAM_DOTSTAR_SLASH___/g, '(?:/(.*))?'],
  [/___PARAM_OPT_SLASH___/g, '(?:/([^/]*))?'],
  [/___PARAM_DOTSTAR___/g, '(.*)'],
  [/___PARAM_DOTPLUS___/g, '(.+)'],
  [/___PARAM_OPT___/g, '([^/]*)'],
  [/___PARAM_SEG___/g, '([^/]+)'],
  [/___STAR___/g, '.*'],
]

function paramTokenForMatch(match: string): string {
  if (match.startsWith('/:')) {
    if (match.endsWith('*')) return '___PARAM_DOTSTAR_SLASH___'
    if (match.endsWith('?')) return '___PARAM_OPT_SLASH___'
  }
  if (match.endsWith('*')) return '___PARAM_DOTSTAR___'
  if (match.endsWith('+')) return '___PARAM_DOTPLUS___'
  if (match.endsWith('?')) return '___PARAM_OPT___'
  return '___PARAM_SEG___'
}

function normalizePath(path: string): string {
  const collapsed = path.replace(MULTIPLE_SLASHES_REGEX, '/')
  return collapsed === '/' ? '/' : collapsed.replace(PATH_TRAILING_SLASH_REGEX, '')
}

function compilePattern(pattern: string): {
  readonly regex: RegExp
  readonly paramNames: readonly string[]
} {
  const paramNames: string[] = []

  let regexPattern = pattern.replace(
    PARAM_TOKEN_REGEX,
    (
      match: string,
      slashStar: string | undefined,
      slashOpt: string | undefined,
      star: string | undefined,
      plus: string | undefined,
      opt: string | undefined,
      seg: string | undefined,
    ) => {
      paramNames.push(slashStar ?? slashOpt ?? star ?? plus ?? opt ?? seg ?? '')
      return paramTokenForMatch(match)
    },
  )

  regexPattern = regexPattern.replace(ASTERISK_REGEX, '___STAR___')
  regexPattern = regexPattern.replace(ESCAPE_CHARS_REGEX, '\\$&')

  for (const [placeholder, replacement] of PLACEHOLDER_REPLACEMENTS) {
    regexPattern = regexPattern.replace(placeholder, replacement)
  }

  return {
    regex: new RegExp(`^${regexPattern}$`),
    paramNames,
  }
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
  const normalizedPath = normalizePath(pathname)
  const normalizedPattern = normalizePath(pattern)
  const { regex } = compilePattern(normalizedPattern)
  return regex.test(normalizedPath)
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
  const params: Record<string, string> = {}

  const normalizedPath = normalizePath(pathname)
  const normalizedPattern = normalizePath(pattern)
  const { regex, paramNames } = compilePattern(normalizedPattern)
  const match = normalizedPath.match(regex)

  if (!match) return null

  for (let i = 0; i < paramNames.length; i++) params[paramNames[i]] = match[i + 1] ?? ''

  return params
}
