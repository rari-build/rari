import type { ProxyConfig, ProxyMatcher, ProxyRuleCondition, RariRequest } from './types'
import {
  MULTIPLE_SLASHES_REGEX,
  PATH_TRAILING_SLASH_REGEX,
} from '../shared/regex-constants'

const ESCAPE_CHARS_REGEX = /[.+?^${}()|[\]\\]/g
const ASTERISK_REGEX = /\*/g
const PARAM_REGEX = /:(\w+)/g
const PARAM_ASTERISK_REGEX = /:(\w+)\*/g
const PARAM_PLUS_REGEX = /:(\w+)\+/g
const PARAM_QUESTION_REGEX = /:(\w+)\?/g
const PARAM_DOTSTAR_PLACEHOLDER_REGEX = /___PARAM_DOTSTAR___/g
const PARAM_DOTPLUS_PLACEHOLDER_REGEX = /___PARAM_DOTPLUS___/g
const PARAM_OPT_PLACEHOLDER_REGEX = /___PARAM_OPT___/g
const PARAM_SEG_PLACEHOLDER_REGEX = /___PARAM_SEG___/g
const STAR_PLACEHOLDER_REGEX = /___STAR___/g

function normalizePath(path: string): string {
  const collapsed = path.replace(MULTIPLE_SLASHES_REGEX, '/')
  return collapsed === '/' ? '/' : collapsed.replace(PATH_TRAILING_SLASH_REGEX, '')
}

function pathToRegex(pattern: string): RegExp {
  let regexPattern = pattern

  regexPattern = regexPattern.replace(PARAM_ASTERISK_REGEX, '___PARAM_DOTSTAR___')
  regexPattern = regexPattern.replace(PARAM_PLUS_REGEX, '___PARAM_DOTPLUS___')
  regexPattern = regexPattern.replace(PARAM_QUESTION_REGEX, '___PARAM_OPT___')
  regexPattern = regexPattern.replace(PARAM_REGEX, '___PARAM_SEG___')
  regexPattern = regexPattern.replace(ASTERISK_REGEX, '___STAR___')

  regexPattern = regexPattern.replace(ESCAPE_CHARS_REGEX, '\\$&')

  regexPattern = regexPattern
    .replace(PARAM_DOTSTAR_PLACEHOLDER_REGEX, '(.*)')
    .replace(PARAM_DOTPLUS_PLACEHOLDER_REGEX, '(.+)')
    .replace(PARAM_OPT_PLACEHOLDER_REGEX, '([^/]*)')
    .replace(PARAM_SEG_PLACEHOLDER_REGEX, '([^/]+)')
    .replace(STAR_PLACEHOLDER_REGEX, '.*')

  regexPattern = `^${regexPattern}$`

  return new RegExp(regexPattern)
}

/* v8 ignore start - requires complex RariRequest mocking */
function checkHeaderCondition(
  request: RariRequest,
  key: string,
): string | null {
  return request.headers.get(key)
}

function checkQueryCondition(
  request: RariRequest,
  key: string,
): string | null {
  return request.rariUrl.searchParams.get(key)
}

function checkCookieCondition(
  request: RariRequest,
  key: string,
): string | null {
  const cookie = request.cookies.get(key)
  return cookie ? cookie.value : null
}

function checkHostCondition(
  request: RariRequest,
  key: string,
): string | null {
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
      throw new Error(`Unknown condition type: ${(condition as any).type}`)
  }
}

function matchesHasCondition(
  request: RariRequest,
  condition: ProxyRuleCondition,
): boolean {
  const actualValue = getConditionActualValue(request, condition)

  if (actualValue === null)
    return false
  if (condition.value !== undefined && actualValue !== condition.value)
    return false

  return true
}

function matchesMissingCondition(
  request: RariRequest,
  condition: ProxyRuleCondition,
): boolean {
  const actualValue = getConditionActualValue(request, condition)

  if (actualValue === null)
    return true
  if (condition.value === undefined)
    return false

  return actualValue !== condition.value
}

function matchesConditions(
  request: RariRequest,
  matcher: ProxyMatcher,
): boolean {
  if (matcher.has) {
    for (const condition of matcher.has) {
      if (!matchesHasCondition(request, condition))
        return false
    }
  }

  if (matcher.missing) {
    for (const condition of matcher.missing) {
      if (!matchesMissingCondition(request, condition))
        return false
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
function matchesSingleMatcher(
  request: RariRequest,
  pathname: string,
  matcher: string | ProxyMatcher,
): boolean {
  if (typeof matcher === 'string')
    return matchesPattern(pathname, matcher)

  if (!matchesPattern(pathname, matcher.source))
    return false

  return matchesConditions(request, matcher)
}

export function shouldRunProxy(
  request: RariRequest,
  config?: ProxyConfig,
): boolean {
  if (!config?.matcher)
    return true

  const pathname = request.rariUrl.pathname
  const matchers = Array.isArray(config.matcher) ? config.matcher : [config.matcher]

  return matchers.some(matcher => matchesSingleMatcher(request, pathname, matcher))
}
/* v8 ignore stop */

export function extractParams(
  pathname: string,
  pattern: string,
): Record<string, string> | null {
  const params: Record<string, string> = {}

  const normalizedPath = normalizePath(pathname)
  const normalizedPattern = normalizePath(pattern)

  const paramInfo: Array<{ name: string, pos: number }> = []
  let regexPattern = normalizedPattern

  /* v8 ignore start - advanced parameter patterns not commonly used */
  regexPattern = regexPattern.replace(PARAM_ASTERISK_REGEX, (_match, name, offset) => {
    paramInfo.push({ name, pos: offset })
    return '___PARAM_DOTSTAR___'
  })
  regexPattern = regexPattern.replace(PARAM_PLUS_REGEX, (_match, name, offset) => {
    paramInfo.push({ name, pos: offset })
    return '___PARAM_DOTPLUS___'
  })
  regexPattern = regexPattern.replace(PARAM_QUESTION_REGEX, (_match, name, offset) => {
    paramInfo.push({ name, pos: offset })
    return '___PARAM_OPT___'
  })
  /* v8 ignore stop */

  regexPattern = regexPattern.replace(PARAM_REGEX, (_match, name, offset) => {
    paramInfo.push({ name, pos: offset })
    return '___PARAM_SEG___'
  })

  const paramNames = paramInfo.sort((a, b) => a.pos - b.pos).map(p => p.name)

  regexPattern = regexPattern.replace(ASTERISK_REGEX, '___STAR___')
  regexPattern = regexPattern.replace(ESCAPE_CHARS_REGEX, '\\$&')

  regexPattern = regexPattern
    .replace(PARAM_DOTSTAR_PLACEHOLDER_REGEX, '(.*)')
    .replace(PARAM_DOTPLUS_PLACEHOLDER_REGEX, '(.+)')
    .replace(PARAM_OPT_PLACEHOLDER_REGEX, '([^/]*)')
    .replace(PARAM_SEG_PLACEHOLDER_REGEX, '([^/]+)')
    .replace(STAR_PLACEHOLDER_REGEX, '.*')

  regexPattern = `^${regexPattern}$`

  const regex = new RegExp(regexPattern)
  const match = normalizedPath.match(regex)

  if (!match)
    return null

  for (let i = 0; i < paramNames.length; i++)
    params[paramNames[i]] = match[i + 1]

  return params
}
