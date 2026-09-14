import type { ProxyManifest, ProxyRule } from '@/proxy/http/types'

const RUNTIME_MARKER_REGEX =
  /\.cookies\b|\.headers\b|\.searchParams\b|\.geo\b|\.ip\b|\bwaitUntil\b|\bfetch\s*\(|RariResponse\.json\b|\.startsWith\s*\(|\.includes\s*\(|\.endsWith\s*\(|\bawait\b|`[^`]*\$\{/

const REDIRECT_OR_REWRITE_CALL_REGEX = /RariResponse\.(?:redirect|rewrite)\s*\(/

const IF_ACTION_REGEX =
  /if\s*\(([^)]+)\)(?:\s*\{)?\s+return\s+RariResponse\.(redirect|rewrite)\(\s*new\s+URL\(\s*(['"`])([^'"`]+)\3\s*,[^)]+\)(?:\s*,\s*(\d+))?\s*\)/g

const PATH_EQ_RIGHT_REGEX = /(?:===|==)\s*(['"`])([^'"`]+)\1/g
const PATH_EQ_LEFT_REGEX = /(['"`])([^'"`]+)\1\s*(?:===|==)/g

export interface ProxyAnalysis {
  readonly requiresRuntime: boolean
  readonly rules: readonly ProxyRule[]
  readonly matcher?: ProxyManifest['matcher']
}

function isPermanentStatus(status: number | undefined): boolean {
  return status === 301 || status === 308
}

function extractPathsFromCondition(condition: string): string[] {
  const paths: string[] = []

  PATH_EQ_RIGHT_REGEX.lastIndex = 0
  for (const match of condition.matchAll(PATH_EQ_RIGHT_REGEX)) {
    const path = match[2]
    if (path !== '') paths.push(path)
  }

  PATH_EQ_LEFT_REGEX.lastIndex = 0
  for (const match of condition.matchAll(PATH_EQ_LEFT_REGEX)) {
    const path = match[2]
    if (path !== '') paths.push(path)
  }

  return paths
}

function extractStaticRules(code: string): ProxyRule[] {
  const rules: ProxyRule[] = []
  IF_ACTION_REGEX.lastIndex = 0

  for (const match of code.matchAll(IF_ACTION_REGEX)) {
    const condition = match[1]
    const action = match[2]
    const destination = match[4]
    const statusToken = match[5]
    const status = statusToken !== '' ? Number.parseInt(statusToken, 10) : undefined

    if (condition === '' || destination === '') continue
    if (action !== 'redirect' && action !== 'rewrite') continue

    const sources = extractPathsFromCondition(condition)
    if (sources.length === 0) return []

    for (const source of sources) {
      rules.push({
        source,
        type: action,
        destination,
        ...(action === 'redirect' ? { permanent: isPermanentStatus(status) } : {}),
      })
    }
  }

  return rules
}

export function analyzeProxySource(code: string): ProxyAnalysis {
  if (RUNTIME_MARKER_REGEX.test(code)) {
    return { requiresRuntime: true, rules: [] }
  }

  const hasRedirectOrRewrite = REDIRECT_OR_REWRITE_CALL_REGEX.test(code)
  const rules = extractStaticRules(code)

  if (hasRedirectOrRewrite && rules.length === 0) {
    return { requiresRuntime: true, rules: [] }
  }

  return { requiresRuntime: false, rules }
}

export function buildProxyManifest(options: {
  readonly proxyFile: string
  readonly code: string
  readonly bundlePath?: string
  readonly generated?: string
}): ProxyManifest {
  const analysis = analyzeProxySource(options.code)
  const requiresRuntime = analysis.requiresRuntime

  return {
    proxyFile: options.proxyFile,
    enabled: true,
    generated: options.generated ?? new Date().toISOString(),
    rules: analysis.rules,
    requiresRuntime,
    ...(requiresRuntime && options.bundlePath != null && options.bundlePath !== ''
      ? { bundlePath: options.bundlePath }
      : {}),
    ...(analysis.matcher != null ? { matcher: analysis.matcher } : {}),
  }
}
