import type { ProxyManifest, ProxyRule } from '@/proxy/http/types'

const RUNTIME_MARKER_REGEX =
  /\.cookies\b|\.headers\b|\.searchParams\b|\.geo\b|\.ip\b|\bwaitUntil\b|\bfetch\s*\(|RariResponse\.json\b|\.startsWith\s*\(|\.includes\s*\(|\.endsWith\s*\(|\bawait\b|`[^`]*\$\{/

const REDIRECT_OR_REWRITE_CALL_REGEX = /RariResponse\.(?:redirect|rewrite)\s*\(/

const IF_ACTION_REGEX =
  /if\s*\(([^)]+)\)(?:\s*\{)?\s+return\s+RariResponse\.(redirect|rewrite)\(\s*new\s+URL\(\s*(['"`])([^'"`]+)\3\s*,[^)]+\)(?:\s*,\s*(\d+))?\s*\)/g

const PATH_EQUALITY_CLAUSE_REGEX =
  /^(?:(request\.rariUrl\.pathname|pathname|normalizedPath)\s*(?:===|==)\s*(['"`])([^'"`]+)\2|(['"`])([^'"`]+)\4\s*(?:===|==)\s*(request\.rariUrl\.pathname|pathname|normalizedPath))$/

export interface ProxyAnalysis {
  readonly requiresRuntime: boolean
  readonly rules: readonly ProxyRule[]
  readonly matcher?: ProxyManifest['matcher']
}

function isPermanentStatus(status: number | undefined): boolean {
  return status === 301 || status === 308
}

function extractStaticPathSources(condition: string): string[] | null {
  const trimmed = condition.trim()
  if (trimmed === '' || trimmed.includes('&&')) return null

  const clauses = trimmed.split(/\|\|/)
  const paths: string[] = []
  let pathRef: string | undefined

  for (const clause of clauses) {
    const match = PATH_EQUALITY_CLAUSE_REGEX.exec(clause.trim())
    if (match == null) return null

    const ref = match[1] || match[6] || ''
    const path = match[3] || match[5] || ''
    if (ref === '' || path === '') return null
    if (pathRef == null) pathRef = ref
    else if (pathRef !== ref) return null

    paths.push(path)
  }

  return paths.length > 0 ? paths : null
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

    const sources = extractStaticPathSources(condition)
    if (sources == null) return []

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
