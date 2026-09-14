import type { ProxyConfig, ProxyManifest, ProxyRule } from '@/proxy/http/types'

const RUNTIME_MARKER_REGEX =
  /\.cookies\b|\.headers\b|\.searchParams\b|\.geo\b|\.ip\b|\bwaitUntil\b|\bfetch\s*\(|RariResponse\.json\b|\.startsWith\s*\(|\.includes\s*\(|\.endsWith\s*\(|\bawait\b|`[^`]*\$\{/

const REDIRECT_OR_REWRITE_CALL_REGEX = /RariResponse\.(?:redirect|rewrite)\s*\(/

const IF_ACTION_REGEX =
  /if\s*\(([^)]+)\)(?:\s*\{)?\s+return\s+RariResponse\.(redirect|rewrite)\(\s*new\s+URL\(\s*(['"`])([^'"`]+)\3\s*,[^)]+\)(?:\s*,\s*(\d+))?\s*\)/g

const PATH_EQUALITY_CLAUSE_REGEX =
  /^(?:(request\.rariUrl\.pathname|pathname|normalizedPath)\s*(?:===|==)\s*(['"`])([^'"`]+)\2|(['"`])([^'"`]+)\4\s*(?:===|==)\s*(request\.rariUrl\.pathname|pathname|normalizedPath))$/

const CONFIG_EXPORT_REGEX = /export\s+const\s+config\s*=/
const CONFIG_OBJECT_EXPORT_REGEX = /export\s+const\s+config\s*=\s*\{/
const OBJECT_MATCHER_REGEX = /matcher\s*:\s*\{/
const STRING_MATCHER_REGEX = /matcher\s*:\s*(['"`])([^'"`]+)\1/
const ARRAY_MATCHER_REGEX = /matcher\s*:\s*\[([^\]]*)\]/
const ARRAY_STRING_ITEM_REGEX = /(['"`])([^'"`]+)\1/g
const MATCHER_SHORTHAND_REGEX = /(?:^|[{,]\s*)matcher\s*[,}]/

export interface ProxyAnalysis {
  readonly requiresRuntime: boolean
  readonly rules: readonly ProxyRule[]
  readonly matcher?: ProxyManifest['matcher']
}

function isPermanentStatus(status: number | undefined): boolean {
  return status === 301 || status === 308
}

interface CodeFrame {
  readonly kind: 'code'
  braceDepth: number
}

type ScanFrame =
  | CodeFrame
  | { readonly kind: 'single' }
  | { readonly kind: 'double' }
  | { readonly kind: 'template' }
  | { readonly kind: 'line-comment' }
  | { readonly kind: 'block-comment' }

function skipStringLike(code: string, start: number, quote: "'" | '"' | '`'): number | null {
  for (let i = start + 1; i < code.length; i++) {
    const ch = code[i]
    if (ch === '\\') {
      i += 1
      continue
    }
    if (quote === '`' && ch === '$' && code[i + 1] === '{') return null
    if (ch === quote) return i + 1
  }
  return null
}

function extractBalancedBraces(code: string, braceStart: number): string | null {
  const stack: ScanFrame[] = [{ kind: 'code', braceDepth: 0 }]

  for (let i = braceStart; i < code.length;) {
    const frame = stack.at(-1)
    if (frame === undefined) return null

    const ch = code[i]
    const next = code[i + 1]

    if (frame.kind === 'line-comment') {
      if (ch === '\n') stack.pop()
      i += 1
      continue
    }

    if (frame.kind === 'block-comment') {
      if (ch === '*' && next === '/') {
        stack.pop()
        i += 2
      } else {
        i += 1
      }
      continue
    }

    if (frame.kind === 'single' || frame.kind === 'double') {
      if (ch === '\\') {
        i += 2
        continue
      }
      if ((frame.kind === 'single' && ch === "'") || (frame.kind === 'double' && ch === '"'))
        stack.pop()
      i += 1
      continue
    }

    if (frame.kind === 'template') {
      if (ch === '\\') {
        i += 2
        continue
      }
      if (ch === '`') {
        stack.pop()
        i += 1
        continue
      }
      if (ch === '$' && next === '{') {
        stack.push({ kind: 'code', braceDepth: 1 })
        i += 2
        continue
      }
      i += 1
      continue
    }

    if (ch === '/' && next === '/') {
      stack.push({ kind: 'line-comment' })
      i += 2
      continue
    }
    if (ch === '/' && next === '*') {
      stack.push({ kind: 'block-comment' })
      i += 2
      continue
    }
    if (ch === "'") {
      stack.push({ kind: 'single' })
      i += 1
      continue
    }
    if (ch === '"') {
      stack.push({ kind: 'double' })
      i += 1
      continue
    }
    if (ch === '`') {
      stack.push({ kind: 'template' })
      i += 1
      continue
    }
    if (ch === '{') {
      frame.braceDepth += 1
      i += 1
      continue
    }
    if (ch === '}') {
      frame.braceDepth -= 1
      i += 1
      if (frame.braceDepth === 0) {
        if (stack.length === 1) return code.slice(braceStart, i)
        stack.pop()
      }
      continue
    }

    i += 1
  }

  return null
}

function readStaticMatcherValue(
  code: string,
  equalsIndex: number,
): {
  readonly matcher?: ProxyConfig['matcher']
  readonly forceRuntime: boolean
  readonly endIndex: number
} | null {
  let i = equalsIndex + 1
  while (i < code.length && /\s/.test(code[i])) i += 1
  if (i >= code.length) return null

  const ch = code[i]
  if (ch === "'" || ch === '"' || ch === '`') {
    if (ch === '`') {
      const uncertain = skipStringLike(code, i, '`')
      if (uncertain == null) return { forceRuntime: true, endIndex: code.length }
    }
    const end = skipStringLike(code, i, ch)
    if (end == null) return { forceRuntime: true, endIndex: code.length }
    const raw = code.slice(i + 1, end - 1)
    if (raw === '' || (ch === '`' && raw.includes('${')))
      return { forceRuntime: true, endIndex: end }
    return { matcher: raw, forceRuntime: false, endIndex: end }
  }

  if (ch === '[') {
    const bodyStart = i + 1
    let depth = 1
    let j = bodyStart
    const stack: ScanFrame[] = [{ kind: 'code', braceDepth: 0 }]
    for (; j < code.length && depth > 0;) {
      const frame = stack.at(-1)
      if (frame === undefined) return { forceRuntime: true, endIndex: code.length }
      const c = code[j]
      const n = code[j + 1]

      if (frame.kind === 'line-comment') {
        if (c === '\n') stack.pop()
        j += 1
        continue
      }
      if (frame.kind === 'block-comment') {
        if (c === '*' && n === '/') {
          stack.pop()
          j += 2
        } else j += 1
        continue
      }
      if (frame.kind === 'single' || frame.kind === 'double' || frame.kind === 'template') {
        if (c === '\\') {
          j += 2
          continue
        }
        if (frame.kind === 'template' && c === '$' && n === '{')
          return { forceRuntime: true, endIndex: code.length }
        if (
          (frame.kind === 'single' && c === "'") ||
          (frame.kind === 'double' && c === '"') ||
          (frame.kind === 'template' && c === '`')
        )
          stack.pop()
        j += 1
        continue
      }

      if (c === '/' && n === '/') {
        stack.push({ kind: 'line-comment' })
        j += 2
        continue
      }
      if (c === '/' && n === '*') {
        stack.push({ kind: 'block-comment' })
        j += 2
        continue
      }
      if (c === "'") {
        stack.push({ kind: 'single' })
        j += 1
        continue
      }
      if (c === '"') {
        stack.push({ kind: 'double' })
        j += 1
        continue
      }
      if (c === '`') {
        stack.push({ kind: 'template' })
        j += 1
        continue
      }
      if (c === '[') {
        depth += 1
        j += 1
        continue
      }
      if (c === ']') {
        depth -= 1
        j += 1
        continue
      }
      if (c === '{') return { forceRuntime: true, endIndex: j }
      j += 1
    }
    if (depth !== 0) return { forceRuntime: true, endIndex: code.length }
    return { ...parseStaticStringArrayBody(code.slice(bodyStart, j - 1)), endIndex: j }
  }

  return { forceRuntime: true, endIndex: i + 1 }
}

function resolveModuleLevelMatcherBinding(code: string): {
  readonly matcher?: ProxyConfig['matcher']
  readonly forceRuntime: boolean
} | null {
  const stack: ScanFrame[] = [{ kind: 'code', braceDepth: 0 }]
  const bindings: Array<{
    readonly matcher?: ProxyConfig['matcher']
    readonly forceRuntime: boolean
  }> = []

  for (let i = 0; i < code.length;) {
    const frame = stack.at(-1)
    if (frame === undefined) return { forceRuntime: true }

    const ch = code[i]
    const next = code[i + 1]

    if (frame.kind === 'line-comment') {
      if (ch === '\n') stack.pop()
      i += 1
      continue
    }
    if (frame.kind === 'block-comment') {
      if (ch === '*' && next === '/') {
        stack.pop()
        i += 2
      } else i += 1
      continue
    }
    if (frame.kind === 'single' || frame.kind === 'double') {
      if (ch === '\\') {
        i += 2
        continue
      }
      if ((frame.kind === 'single' && ch === "'") || (frame.kind === 'double' && ch === '"'))
        stack.pop()
      i += 1
      continue
    }
    if (frame.kind === 'template') {
      if (ch === '\\') {
        i += 2
        continue
      }
      if (ch === '`') {
        stack.pop()
        i += 1
        continue
      }
      if (ch === '$' && next === '{') {
        stack.push({ kind: 'code', braceDepth: 1 })
        i += 2
        continue
      }
      i += 1
      continue
    }

    if (ch === '/' && next === '/') {
      stack.push({ kind: 'line-comment' })
      i += 2
      continue
    }
    if (ch === '/' && next === '*') {
      stack.push({ kind: 'block-comment' })
      i += 2
      continue
    }
    if (ch === "'") {
      stack.push({ kind: 'single' })
      i += 1
      continue
    }
    if (ch === '"') {
      stack.push({ kind: 'double' })
      i += 1
      continue
    }
    if (ch === '`') {
      stack.push({ kind: 'template' })
      i += 1
      continue
    }
    if (ch === '{') {
      frame.braceDepth += 1
      i += 1
      continue
    }
    if (ch === '}') {
      frame.braceDepth -= 1
      i += 1
      if (frame.braceDepth === 0 && stack.length > 1) stack.pop()
      continue
    }

    const prev = i === 0 ? '' : code.charAt(i - 1)
    const atBoundary = i === 0 || /[\s;{}]/.test(prev)

    if (
      frame.braceDepth === 0 &&
      stack.length === 1 &&
      atBoundary &&
      code.startsWith('const matcher', i)
    ) {
      const afterName = i + 'const matcher'.length
      if (afterName < code.length && /[\w$]/.test(code.charAt(afterName))) {
        i += 1
        continue
      }
      let j = afterName
      while (j < code.length && /\s/.test(code.charAt(j))) j += 1
      if (code.charAt(j) !== '=') {
        i += 1
        continue
      }
      const resolved = readStaticMatcherValue(code, j)
      if (resolved == null || resolved.forceRuntime) return { forceRuntime: true }
      bindings.push({ matcher: resolved.matcher, forceRuntime: false })
      i = resolved.endIndex
      continue
    }

    if (
      frame.braceDepth === 0 &&
      stack.length === 1 &&
      atBoundary &&
      (code.startsWith('let matcher', i) || code.startsWith('var matcher', i))
    ) {
      return { forceRuntime: true }
    }

    i += 1
  }

  if (bindings.length === 0) return null
  if (bindings.length > 1) return { forceRuntime: true }
  return bindings[0] ?? null
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
    const parsedStatus = Number.parseInt(match[5], 10)
    const status = Number.isFinite(parsedStatus) ? parsedStatus : undefined

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

function extractExportedConfigObject(code: string): string | null {
  const startMatch = CONFIG_OBJECT_EXPORT_REGEX.exec(code)
  if (startMatch == null) return null

  const braceStart = startMatch.index + startMatch[0].length - 1
  return extractBalancedBraces(code, braceStart)
}

function parseStaticStringArrayBody(body: string): {
  readonly matcher?: string[]
  readonly forceRuntime: boolean
} {
  if (body.includes('{')) return { forceRuntime: true }

  const items: string[] = []
  ARRAY_STRING_ITEM_REGEX.lastIndex = 0
  for (const item of body.matchAll(ARRAY_STRING_ITEM_REGEX)) {
    if (item[2] !== '') items.push(item[2])
  }

  const remainder = body.replace(ARRAY_STRING_ITEM_REGEX, '').replace(/[\s,]/g, '')
  if (remainder !== '') return { forceRuntime: true }

  if (items.length > 0) return { matcher: items, forceRuntime: false }
  return { forceRuntime: true }
}

function extractMatcher(code: string): {
  readonly matcher?: ProxyConfig['matcher']
  readonly forceRuntime: boolean
} {
  if (!CONFIG_EXPORT_REGEX.test(code)) return { forceRuntime: false }

  if (!CONFIG_OBJECT_EXPORT_REGEX.test(code)) return { forceRuntime: false }

  const configObject = extractExportedConfigObject(code)
  if (configObject == null) return { forceRuntime: true }

  if (OBJECT_MATCHER_REGEX.test(configObject)) {
    return { forceRuntime: true }
  }

  const stringMatch = STRING_MATCHER_REGEX.exec(configObject)
  if (stringMatch != null && stringMatch[2] !== '') {
    return { matcher: stringMatch[2], forceRuntime: false }
  }

  const arrayMatch = ARRAY_MATCHER_REGEX.exec(configObject)
  if (arrayMatch != null) return parseStaticStringArrayBody(arrayMatch[1])

  if (MATCHER_SHORTHAND_REGEX.test(configObject)) {
    return resolveModuleLevelMatcherBinding(code) ?? { forceRuntime: true }
  }

  if (/matcher\s*:/.test(configObject)) return { forceRuntime: true }

  return { forceRuntime: false }
}

export function analyzeProxySource(code: string): ProxyAnalysis {
  const { matcher, forceRuntime } = extractMatcher(code)

  if (forceRuntime || RUNTIME_MARKER_REGEX.test(code)) {
    return { requiresRuntime: true, rules: [], ...(matcher != null ? { matcher } : {}) }
  }

  const hasRedirectOrRewrite = REDIRECT_OR_REWRITE_CALL_REGEX.test(code)
  const rules = extractStaticRules(code)

  if (hasRedirectOrRewrite && rules.length === 0) {
    return { requiresRuntime: true, rules: [], ...(matcher != null ? { matcher } : {}) }
  }

  return { requiresRuntime: false, rules, ...(matcher != null ? { matcher } : {}) }
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
