import type { ProxyConfig, ProxyMatcher, RariRequest } from './types'
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
