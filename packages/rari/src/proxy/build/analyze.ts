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
const QUOTED_OBJECT_MATCHER_REGEX = /(['"`])matcher\1\s*:\s*\{/
const QUOTED_STRING_MATCHER_REGEX = /(['"`])matcher\1\s*:\s*(['"`])([^'"`]+)\2/
const QUOTED_ARRAY_MATCHER_REGEX = /(['"`])matcher\1\s*:\s*\[([^\]]*)\]/
const QUOTED_MATCHER_KEY_REGEX = /(['"`])matcher\1\s*:/
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
  | { readonly kind: 'regex' }
  | { readonly kind: 'regex-class' }

// oxlint-disable typescript/prefer-readonly-parameter-types
const REGEX_AFTER_KEYWORDS = new Set([
  'return',
  'throw',
  'case',
  'else',
  'do',
  'typeof',
  'delete',
  'void',
  'await',
  'yield',
  'new',
  'in',
  'of',
  'instanceof',
])

function isLineTerminator(ch: string): boolean {
  return ch === '\n' || ch === '\r' || ch === '\u2028' || ch === '\u2029'
}

function skipBackwardBlockCommentInCode(code: string, i: number): number {
  i -= 2
  while (i >= 1) {
    if (code.charAt(i - 1) === '/' && code.charAt(i) === '*') return i - 2
    i -= 1
  }
  return i
}

function findLineCommentStartInRange(code: string, lineStart: number, i: number): number {
  for (let j = lineStart; j < i; j++) {
    if (code.charAt(j) === '/' && code.charAt(j + 1) === '/') return j
    if (code.charAt(j) === '/' && code.charAt(j + 1) === '*') {
      j += 2
      while (j < i && (code.charAt(j) !== '*' || code.charAt(j + 1) !== '/')) j += 1
      j += 1
    }
  }
  return -1
}

function skipBackwardTriviaForRegex(code: string, slashIndex: number): number {
  let i = slashIndex - 1
  while (i >= 0) {
    while (i >= 0 && /\s/.test(code.charAt(i))) i -= 1
    if (i < 0) return i

    if (code.charAt(i) === '/' && i > 0 && code.charAt(i - 1) === '*') {
      i = skipBackwardBlockCommentInCode(code, i)
      continue
    }

    let lineStart = i
    while (lineStart > 0 && !isLineTerminator(code.charAt(lineStart - 1))) lineStart -= 1
    const lineCommentAt = findLineCommentStartInRange(code, lineStart, i)
    if (lineCommentAt !== -1) {
      i = lineCommentAt - 1
      continue
    }
    break
  }
  return i
}

function canStartRegexLiteral(code: string, slashIndex: number): boolean {
  const i = skipBackwardTriviaForRegex(code, slashIndex)
  if (i < 0) return true

  const prev = code.charAt(i)
  if (/[)}\]]/.test(prev)) return false
  if (!/[\w$]/.test(prev)) return true

  let start = i
  while (start >= 0 && /[\w$]/.test(code.charAt(start))) start -= 1
  return REGEX_AFTER_KEYWORDS.has(code.slice(start + 1, i + 1))
}

function isLineContinuationBreak(ch: string): boolean {
  return ch === '\n' || ch === '\r' || ch === '\u2028' || ch === '\u2029'
}

function decodeSimpleEscape(escaped: string): string | null {
  switch (escaped) {
    case 'n':
      return '\n'
    case 'r':
      return '\r'
    case 't':
      return '\t'
    case 'b':
      return '\b'
    case 'f':
      return '\f'
    case 'v':
      return '\v'
    case '0':
      return '\0'
    case '\\':
    case "'":
    case '"':
    case '`':
    case '/':
      return escaped
    default:
      return null
  }
}

function decodeHexOrUnicodeEscape(
  raw: string,
  i: number,
  escaped: string,
): { readonly char: string; readonly end: number } | null {
  if (escaped === 'x') {
    const hex = raw.slice(i + 1, i + 3)
    if (!/^[\da-f]{2}$/i.test(hex)) return null
    return { char: String.fromCharCode(Number.parseInt(hex, 16)), end: i + 2 }
  }
  if (escaped !== 'u') return null
  if (raw.charAt(i + 1) === '{') {
    const end = raw.indexOf('}', i + 2)
    if (end === -1) return null
    const hex = raw.slice(i + 2, end)
    if (!/^[\da-f]+$/i.test(hex)) return null
    return { char: String.fromCodePoint(Number.parseInt(hex, 16)), end }
  }
  const hex = raw.slice(i + 1, i + 5)
  if (!/^[\da-f]{4}$/i.test(hex)) return null
  return { char: String.fromCharCode(Number.parseInt(hex, 16)), end: i + 4 }
}

function appendDecodedEscape(
  raw: string,
  i: number,
  escaped: string,
  out: string,
): { readonly out: string; readonly i: number } | null {
  if (isLineContinuationBreak(escaped)) {
    let nextI = i
    if (escaped === '\r' && raw.charAt(i + 1) === '\n') nextI = i + 1
    return { out, i: nextI }
  }
  const simple = decodeSimpleEscape(escaped)
  if (simple != null) return { out: out + simple, i }
  const hex = decodeHexOrUnicodeEscape(raw, i, escaped)
  if (hex != null) return { out: out + hex.char, i: hex.end }
  if (escaped === 'x' || escaped === 'u') return null
  return { out: out + escaped, i }
}

function decodeJsStringLiteral(raw: string, quote: "'" | '"' | '`'): string | null {
  let out = ''
  for (let i = 0; i < raw.length; i++) {
    const ch = raw.charAt(i)
    if (quote === '`' && ch === '$' && raw.charAt(i + 1) === '{') return null
    if (ch !== '\\') {
      out += ch
      continue
    }
    i += 1
    if (i >= raw.length) return null
    const decoded = appendDecodedEscape(raw, i, raw.charAt(i), out)
    if (decoded == null) return null
    out = decoded.out
    i = decoded.i
  }
  return out
}

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

function advanceStringFrame(
  stack: ScanFrame[],
  kind: 'single' | 'double',
  ch: string,
  i: number,
): number {
  if (ch === '\\') return i + 2
  if ((kind === 'single' && ch === "'") || (kind === 'double' && ch === '"')) stack.pop()
  return i + 1
}

function advanceTemplateFrame(stack: ScanFrame[], ch: string, next: string, i: number): number {
  if (ch === '\\') return i + 2
  if (ch === '`') {
    stack.pop()
    return i + 1
  }
  if (ch === '$' && next === '{') {
    stack.push({ kind: 'code', braceDepth: 1 })
    return i + 2
  }
  return i + 1
}

function advanceRegexFrame(
  stack: ScanFrame[],
  code: string,
  ch: string,
  i: number,
  endLimit: number,
): number {
  if (ch === '\\') return i + 2
  if (ch === '[') {
    stack.push({ kind: 'regex-class' })
    return i + 1
  }
  if (ch === '/') {
    stack.pop()
    let j = i + 1
    while (j < endLimit && /[a-z]/i.test(code.charAt(j))) j += 1
    return j
  }
  return i + 1
}

function advanceRegexClassFrame(stack: ScanFrame[], ch: string, i: number): number {
  if (ch === '\\') return i + 2
  if (ch === ']') stack.pop()
  return i + 1
}

function advanceNonCodeFrame(
  stack: ScanFrame[],
  code: string,
  i: number,
  endLimit: number = code.length,
): number | null {
  const frame = stack.at(-1)
  if (frame === undefined || frame.kind === 'code') return null

  const ch = code.charAt(i)
  const next = code.charAt(i + 1)

  if (frame.kind === 'line-comment') {
    if (ch === '\n') stack.pop()
    return i + 1
  }
  if (frame.kind === 'block-comment') {
    if (ch === '*' && next === '/') {
      stack.pop()
      return i + 2
    }
    return i + 1
  }
  if (frame.kind === 'single' || frame.kind === 'double') {
    return advanceStringFrame(stack, frame.kind, ch, i)
  }
  if (frame.kind === 'template') return advanceTemplateFrame(stack, ch, next, i)
  if (frame.kind === 'regex') return advanceRegexFrame(stack, code, ch, i, endLimit)
  return advanceRegexClassFrame(stack, ch, i)
}

function tryEnterNonCodeToken(stack: ScanFrame[], code: string, i: number): number | null {
  const ch = code.charAt(i)
  const next = code.charAt(i + 1)

  if (ch === '/' && next === '/') {
    stack.push({ kind: 'line-comment' })
    return i + 2
  }
  if (ch === '/' && next === '*') {
    stack.push({ kind: 'block-comment' })
    return i + 2
  }
  if (ch === '/' && canStartRegexLiteral(code, i)) {
    stack.push({ kind: 'regex' })
    return i + 1
  }
  if (ch === "'") {
    stack.push({ kind: 'single' })
    return i + 1
  }
  if (ch === '"') {
    stack.push({ kind: 'double' })
    return i + 1
  }
  if (ch === '`') {
    stack.push({ kind: 'template' })
    return i + 1
  }
  return null
}

function applyCodeBraceChar(frame: CodeFrame, stack: ScanFrame[], ch: string): boolean {
  if (ch === '{') {
    frame.braceDepth += 1
    return true
  }
  if (ch === '}') {
    frame.braceDepth -= 1
    if (frame.braceDepth === 0 && stack.length > 1) stack.pop()
    return true
  }
  return false
}

function applyBalancedBraceClose(
  frame: CodeFrame,
  stack: ScanFrame[],
  code: string,
  braceStart: number,
  i: number,
): { readonly i: number; readonly done: string | null } {
  frame.braceDepth -= 1
  const nextI = i + 1
  if (frame.braceDepth !== 0) return { i: nextI, done: null }
  if (stack.length === 1) return { i: nextI, done: code.slice(braceStart, nextI) }
  stack.pop()
  return { i: nextI, done: null }
}

function advanceBalancedBraceScan(
  stack: ScanFrame[],
  code: string,
  i: number,
  braceStart: number,
): { readonly i: number; readonly done: string | null } | null {
  const frame = stack.at(-1)
  if (frame === undefined) return null

  const nonCode = advanceNonCodeFrame(stack, code, i)
  if (nonCode != null) return { i: nonCode, done: null }

  if (frame.kind !== 'code') return null

  const entered = tryEnterNonCodeToken(stack, code, i)
  if (entered != null) return { i: entered, done: null }

  const ch = code.charAt(i)
  if (ch === '{') {
    frame.braceDepth += 1
    return { i: i + 1, done: null }
  }
  if (ch === '}') return applyBalancedBraceClose(frame, stack, code, braceStart, i)
  return { i: i + 1, done: null }
}

function extractBalancedBraces(code: string, braceStart: number): string | null {
  const stack: ScanFrame[] = [{ kind: 'code', braceDepth: 0 }]

  for (let i = braceStart; i < code.length;) {
    const step = advanceBalancedBraceScan(stack, code, i, braceStart)
    if (step == null) return null
    if (step.done != null) return step.done
    i = step.i
  }

  return null
}

interface MatcherValueResult {
  readonly matcher?: ProxyConfig['matcher']
  readonly forceRuntime: boolean
  readonly endIndex: number
}

function readStaticStringMatcher(code: string, i: number, ch: "'" | '"' | '`'): MatcherValueResult {
  if (ch === '`') {
    const uncertain = skipStringLike(code, i, '`')
    if (uncertain == null) return { forceRuntime: true, endIndex: code.length }
  }
  const end = skipStringLike(code, i, ch)
  if (end == null) return { forceRuntime: true, endIndex: code.length }
  const raw = code.slice(i + 1, end - 1)
  if (ch === '`' && raw.includes('${')) return { forceRuntime: true, endIndex: end }
  const decoded = decodeJsStringLiteral(raw, ch)
  if (decoded == null || decoded === '') return { forceRuntime: true, endIndex: end }
  return { matcher: decoded, forceRuntime: false, endIndex: end }
}

function advanceArrayStringFrame(
  stack: ScanFrame[],
  frame: ScanFrame,
  code: string,
  j: number,
): { j: number; forceRuntime?: true } {
  const c = code[j]
  const n = code[j + 1]
  if (c === '\\') return { j: j + 2 }
  if (frame.kind === 'template' && c === '$' && n === '{') return { j, forceRuntime: true }
  if (
    (frame.kind === 'single' && c === "'") ||
    (frame.kind === 'double' && c === '"') ||
    (frame.kind === 'template' && c === '`')
  )
    stack.pop()
  return { j: j + 1 }
}

function advanceArrayScanFrame(
  stack: ScanFrame[],
  code: string,
  j: number,
): { j: number; forceRuntime?: true } | null {
  const frame = stack.at(-1)
  if (frame === undefined) return { j, forceRuntime: true }

  if (frame.kind === 'line-comment') {
    if (code[j] === '\n') stack.pop()
    return { j: j + 1 }
  }
  if (frame.kind === 'block-comment') {
    if (code[j] === '*' && code[j + 1] === '/') {
      stack.pop()
      return { j: j + 2 }
    }
    return { j: j + 1 }
  }
  if (frame.kind === 'single' || frame.kind === 'double' || frame.kind === 'template') {
    return advanceArrayStringFrame(stack, frame, code, j)
  }
  return null
}

function tryEnterArrayStringOrComment(stack: ScanFrame[], code: string, j: number): number | null {
  const c = code[j]
  const n = code[j + 1]
  if (c === '/' && n === '/') {
    stack.push({ kind: 'line-comment' })
    return j + 2
  }
  if (c === '/' && n === '*') {
    stack.push({ kind: 'block-comment' })
    return j + 2
  }
  if (c === "'") {
    stack.push({ kind: 'single' })
    return j + 1
  }
  if (c === '"') {
    stack.push({ kind: 'double' })
    return j + 1
  }
  if (c === '`') {
    stack.push({ kind: 'template' })
    return j + 1
  }
  return null
}

function advanceArrayBracketDepth(
  code: string,
  j: number,
  depth: number,
): { readonly j: number; readonly depth: number; readonly forceRuntime?: true } {
  const c = code[j]
  if (c === '[') return { j: j + 1, depth: depth + 1 }
  if (c === ']') return { j: j + 1, depth: depth - 1 }
  if (c === '{') return { j, depth, forceRuntime: true }
  return { j: j + 1, depth }
}

function readStaticArrayMatcher(code: string, i: number): MatcherValueResult {
  const bodyStart = i + 1
  let depth = 1
  let j = bodyStart
  const stack: ScanFrame[] = [{ kind: 'code', braceDepth: 0 }]

  for (; j < code.length && depth > 0;) {
    const advanced = advanceArrayScanFrame(stack, code, j)
    if (advanced != null) {
      if (advanced.forceRuntime) return { forceRuntime: true, endIndex: code.length }
      j = advanced.j
      continue
    }

    const entered = tryEnterArrayStringOrComment(stack, code, j)
    if (entered != null) {
      j = entered
      continue
    }

    const step = advanceArrayBracketDepth(code, j, depth)
    if (step.forceRuntime) return { forceRuntime: true, endIndex: step.j }
    j = step.j
    depth = step.depth
  }

  if (depth !== 0) return { forceRuntime: true, endIndex: code.length }
  return { ...parseStaticStringArrayBody(code.slice(bodyStart, j - 1)), endIndex: j }
}

function readStaticMatcherValue(code: string, equalsIndex: number): MatcherValueResult | null {
  let i = equalsIndex + 1
  while (i < code.length && /\s/.test(code[i])) i += 1
  if (i >= code.length) return null

  const ch = code[i]
  if (ch === "'" || ch === '"' || ch === '`') return readStaticStringMatcher(code, i, ch)
  if (ch === '[') return readStaticArrayMatcher(code, i)
  return { forceRuntime: true, endIndex: i + 1 }
}

function tryResolveConstMatcherBinding(
  code: string,
  i: number,
  frame: CodeFrame,
  stack: ScanFrame[],
): {
  readonly nextI: number
  readonly binding?: { readonly matcher?: ProxyConfig['matcher']; readonly forceRuntime: boolean }
  readonly forceRuntime?: true
} | null {
  const prev = i === 0 ? '' : code.charAt(i - 1)
  const atBoundary = i === 0 || /[\s;{}]/.test(prev)
  if (frame.braceDepth !== 0 || stack.length !== 1 || !atBoundary) return null

  if (code.startsWith('let matcher', i) || code.startsWith('var matcher', i)) {
    return { nextI: i, forceRuntime: true }
  }

  if (!code.startsWith('const matcher', i)) return null

  const afterName = i + 'const matcher'.length
  if (afterName < code.length && /[\w$]/.test(code.charAt(afterName))) {
    return { nextI: i + 1 }
  }
  let j = afterName
  while (j < code.length && /\s/.test(code.charAt(j))) j += 1
  if (code.charAt(j) !== '=') return { nextI: i + 1 }

  const resolved = readStaticMatcherValue(code, j)
  if (resolved == null || resolved.forceRuntime) return { nextI: i, forceRuntime: true }
  return {
    nextI: resolved.endIndex,
    binding: { matcher: resolved.matcher, forceRuntime: false },
  }
}

function resolveMatcherBindingStep(
  stack: ScanFrame[],
  code: string,
  i: number,
  bindings: Array<{
    readonly matcher?: ProxyConfig['matcher']
    readonly forceRuntime: boolean
  }>,
): { readonly nextI: number; readonly forceRuntime?: true } | null {
  const frame = stack.at(-1)
  if (frame === undefined) return { nextI: i, forceRuntime: true }

  const nonCode = advanceNonCodeFrame(stack, code, i)
  if (nonCode != null) return { nextI: nonCode }

  if (frame.kind !== 'code') return { nextI: i, forceRuntime: true }

  const entered = tryEnterNonCodeToken(stack, code, i)
  if (entered != null) return { nextI: entered }

  const ch = code.charAt(i)
  if (applyCodeBraceChar(frame, stack, ch)) return { nextI: i + 1 }

  const resolved = tryResolveConstMatcherBinding(code, i, frame, stack)
  if (resolved != null) {
    if (resolved.forceRuntime) return { nextI: i, forceRuntime: true }
    if (resolved.binding) bindings.push(resolved.binding)
    return { nextI: resolved.nextI }
  }

  return { nextI: i + 1 }
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
    const step = resolveMatcherBindingStep(stack, code, i, bindings)
    if (step == null) return { forceRuntime: true }
    if (step.forceRuntime) return { forceRuntime: true }
    i = step.nextI
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
    const quote = item[1]
    const raw = item[2]
    if (raw === '' || (quote !== "'" && quote !== '"' && quote !== '`')) continue
    const decoded = decodeJsStringLiteral(raw, quote)
    if (decoded == null) return { forceRuntime: true }
    if (decoded !== '') items.push(decoded)
  }

  const remainder = body.replace(ARRAY_STRING_ITEM_REGEX, '').replace(/[\s,]/g, '')
  if (remainder !== '') return { forceRuntime: true }

  if (items.length > 0) return { matcher: items, forceRuntime: false }
  return { forceRuntime: true }
}

function getCodeBraceDepthAt(configObject: string, index: number): number | null {
  const stack: ScanFrame[] = [{ kind: 'code', braceDepth: 0 }]

  for (let i = 0; i < index;) {
    const frame = stack.at(-1)
    if (frame === undefined) return null

    const nonCode = advanceNonCodeFrame(stack, configObject, i, index)
    if (nonCode != null) {
      i = nonCode
      continue
    }

    if (frame.kind !== 'code') return null

    const entered = tryEnterNonCodeToken(stack, configObject, i)
    if (entered != null) {
      i = entered
      continue
    }

    const ch = configObject.charAt(i)
    if (applyCodeBraceChar(frame, stack, ch)) {
      i += 1
      continue
    }

    i += 1
  }

  const frame = stack.at(-1)
  if (frame?.kind !== 'code') return null
  return frame.braceDepth
}

function isTopLevelConfigMatcher(configObject: string, matcherIndex: number): boolean {
  return getCodeBraceDepthAt(configObject, matcherIndex) === 1
}

function isQuotedMatcherKeyMatch(matchText: string): boolean {
  return matchText.startsWith("'") || matchText.startsWith('"') || matchText.startsWith('`')
}

function findTopLevelMatcherMatch(configObject: string, pattern: RegExp): RegExpExecArray | null {
  const globalPattern = new RegExp(
    pattern.source,
    pattern.flags.includes('g') ? pattern.flags : `${pattern.flags}g`,
  )
  for (const match of configObject.matchAll(globalPattern)) {
    const keyIndex = isQuotedMatcherKeyMatch(match[0])
      ? match.index
      : match.index + match[0].indexOf('matcher')
    if (keyIndex >= match.index && isTopLevelConfigMatcher(configObject, keyIndex)) return match
  }
  return null
}

function isMatcherPropertyDelimiter(configObject: string, index: number): boolean {
  let i = index
  while (i < configObject.length) {
    const skipped = skipWhitespaceOrCommentAt(configObject, i)
    if (skipped != null) {
      i = skipped
      continue
    }
    const ch = configObject.charAt(i)
    return ch === ',' || ch === '}'
  }
  return true
}

function skipWhitespaceAndComments(configObject: string, start: number): number {
  let i = start
  while (i < configObject.length) {
    const skipped = skipWhitespaceOrCommentAt(configObject, i)
    if (skipped == null) break
    i = skipped
  }
  return i
}

function skipWhitespaceOrCommentAt(configObject: string, i: number): number | null {
  const ch = configObject.charAt(i)
  const next = configObject.charAt(i + 1)
  if (/\s/.test(ch)) return i + 1
  if (ch === '/' && next === '/') {
    let j = i + 2
    while (j < configObject.length && !isLineTerminator(configObject.charAt(j))) j += 1
    return j
  }
  if (ch === '/' && next === '*') {
    let j = i + 2
    while (
      j + 1 < configObject.length &&
      (configObject.charAt(j) !== '*' || configObject.charAt(j + 1) !== '/')
    ) {
      j += 1
    }
    return j + 2
  }
  return null
}

function parseComputedPropertyKey(
  configObject: string,
  openBracketIndex: number,
): { readonly endIndex: number; readonly forceRuntime: boolean } | null {
  if (configObject.charAt(openBracketIndex) !== '[') return null

  let i = skipWhitespaceAndComments(configObject, openBracketIndex + 1)
  if (i >= configObject.length) return { endIndex: configObject.length, forceRuntime: true }

  const quote = configObject.charAt(i)
  if (quote === "'" || quote === '"' || quote === '`') {
    const end = skipStringLike(configObject, i, quote)
    if (end == null) return { endIndex: configObject.length, forceRuntime: true }
    const raw = configObject.slice(i + 1, end - 1)
    if (quote === '`' && raw.includes('${')) return { endIndex: end, forceRuntime: true }
    const decoded = decodeJsStringLiteral(raw, quote)
    i = skipWhitespaceAndComments(configObject, end)
    if (configObject.charAt(i) !== ']') return { endIndex: i, forceRuntime: true }
    return { endIndex: i + 1, forceRuntime: decoded === 'matcher' }
  }

  return { endIndex: openBracketIndex + 1, forceRuntime: true }
}

function tryEnterQuotedPropertyKey(
  stack: ScanFrame[],
  frame: CodeFrame,
  ch: string,
  expectPropertyKey: boolean,
): { readonly entered: boolean; readonly expectPropertyKey: boolean } {
  if (ch === "'") {
    return {
      entered: true,
      expectPropertyKey: expectPropertyKey && frame.braceDepth === 1 ? false : expectPropertyKey,
    }
  }
  if (ch === '"') {
    return {
      entered: true,
      expectPropertyKey: expectPropertyKey && frame.braceDepth === 1 ? false : expectPropertyKey,
    }
  }
  if (ch === '`') {
    return {
      entered: true,
      expectPropertyKey: expectPropertyKey && frame.braceDepth === 1 ? false : expectPropertyKey,
    }
  }
  return { entered: false, expectPropertyKey }
}

function applyUnsafeKeyBraceOrPunct(
  frame: CodeFrame,
  stack: ScanFrame[],
  ch: string,
  expectPropertyKey: boolean,
): { readonly handled: boolean; readonly expectPropertyKey: boolean } {
  if (ch === '{') {
    frame.braceDepth += 1
    return { handled: true, expectPropertyKey: frame.braceDepth === 1 ? true : expectPropertyKey }
  }
  if (ch === '}') {
    frame.braceDepth -= 1
    if (frame.braceDepth === 0 && stack.length > 1) stack.pop()
    return { handled: true, expectPropertyKey: false }
  }
  if (ch === ',' && frame.braceDepth === 1) {
    return { handled: true, expectPropertyKey: true }
  }
  if (ch === ':' && frame.braceDepth === 1) {
    return { handled: true, expectPropertyKey: false }
  }
  return { handled: false, expectPropertyKey }
}

function checkUnsafeComputedPropertyAt(
  configObject: string,
  i: number,
  frame: CodeFrame,
  expectPropertyKey: boolean,
  ch: string,
  next: string,
): { readonly unsafe: boolean; readonly nextI?: number; readonly clearExpect?: boolean } | null {
  if (!expectPropertyKey || frame.braceDepth !== 1 || /\s/.test(ch)) return null

  if (ch === '.' && next === '.' && configObject.charAt(i + 2) === '.') {
    return { unsafe: true }
  }
  if (ch === '[') {
    const parsed = parseComputedPropertyKey(configObject, i)
    if (parsed?.forceRuntime === true) return { unsafe: true }
    if (parsed != null) return { unsafe: false, nextI: parsed.endIndex, clearExpect: true }
    return { unsafe: true }
  }
  return { unsafe: false, clearExpect: true }
}

function advanceUnsafeMatcherKeyScan(
  stack: ScanFrame[],
  configObject: string,
  i: number,
  expectPropertyKey: boolean,
): { readonly nextI: number; readonly expectPropertyKey: boolean; readonly unsafe?: true } | null {
  const frame = stack.at(-1)
  if (frame === undefined) return null

  const nonCode = advanceNonCodeFrame(stack, configObject, i)
  if (nonCode != null) return { nextI: nonCode, expectPropertyKey }

  if (frame.kind !== 'code') return null

  const ch = configObject.charAt(i)
  const next = configObject.charAt(i + 1)

  const quoteEnter = tryEnterQuotedPropertyKey(stack, frame, ch, expectPropertyKey)
  if (quoteEnter.entered) {
    const entered = tryEnterNonCodeToken(stack, configObject, i)
    return { nextI: entered ?? i + 1, expectPropertyKey: quoteEnter.expectPropertyKey }
  }

  const entered = tryEnterNonCodeToken(stack, configObject, i)
  if (entered != null) return { nextI: entered, expectPropertyKey }

  const punct = applyUnsafeKeyBraceOrPunct(frame, stack, ch, expectPropertyKey)
  if (punct.handled) return { nextI: i + 1, expectPropertyKey: punct.expectPropertyKey }

  const unsafe = checkUnsafeComputedPropertyAt(configObject, i, frame, expectPropertyKey, ch, next)
  if (unsafe != null) {
    if (unsafe.unsafe) return { nextI: i, expectPropertyKey, unsafe: true }
    return {
      nextI: unsafe.nextI ?? i + 1,
      expectPropertyKey: unsafe.clearExpect ? false : expectPropertyKey,
    }
  }

  return { nextI: i + 1, expectPropertyKey }
}

function hasUnsafeTopLevelComputedMatcherKey(configObject: string): boolean {
  const stack: ScanFrame[] = [{ kind: 'code', braceDepth: 0 }]
  let expectPropertyKey = false

  for (let i = 0; i < configObject.length;) {
    const step = advanceUnsafeMatcherKeyScan(stack, configObject, i, expectPropertyKey)
    if (step == null) return false
    if (step.unsafe) return true
    expectPropertyKey = step.expectPropertyKey
    i = step.nextI
  }

  return false
}

function extractStringMatcherFromMatch(
  configObject: string,
  stringMatch: RegExpExecArray,
): { readonly matcher?: ProxyConfig['matcher']; readonly forceRuntime: boolean } | null {
  const isQuotedKey = isQuotedMatcherKeyMatch(stringMatch[0])
  const quote = isQuotedKey ? stringMatch[2] : stringMatch[1]
  const raw = isQuotedKey ? stringMatch[3] : stringMatch[2]
  if (quote !== "'" && quote !== '"' && quote !== '`') return null
  if (quote === '`' && raw.includes('${')) return { forceRuntime: true }
  const matchEnd = stringMatch.index + stringMatch[0].length
  if (!isMatcherPropertyDelimiter(configObject, matchEnd)) return { forceRuntime: true }
  const decoded = decodeJsStringLiteral(raw, quote)
  if (decoded == null || decoded === '') return { forceRuntime: true }
  return { matcher: decoded, forceRuntime: false }
}

function extractMatcher(code: string): {
  readonly matcher?: ProxyConfig['matcher']
  readonly forceRuntime: boolean
} {
  if (!CONFIG_EXPORT_REGEX.test(code)) return { forceRuntime: false }
  if (!CONFIG_OBJECT_EXPORT_REGEX.test(code)) return { forceRuntime: false }

  const configObject = extractExportedConfigObject(code)
  if (configObject == null) return { forceRuntime: true }
  if (hasUnsafeTopLevelComputedMatcherKey(configObject)) return { forceRuntime: true }

  if (
    findTopLevelMatcherMatch(configObject, OBJECT_MATCHER_REGEX) != null ||
    findTopLevelMatcherMatch(configObject, QUOTED_OBJECT_MATCHER_REGEX) != null
  ) {
    return { forceRuntime: true }
  }

  const stringMatch =
    findTopLevelMatcherMatch(configObject, STRING_MATCHER_REGEX) ??
    findTopLevelMatcherMatch(configObject, QUOTED_STRING_MATCHER_REGEX)
  if (stringMatch != null) {
    return extractStringMatcherFromMatch(configObject, stringMatch) ?? { forceRuntime: false }
  }

  const arrayMatch =
    findTopLevelMatcherMatch(configObject, ARRAY_MATCHER_REGEX) ??
    findTopLevelMatcherMatch(configObject, QUOTED_ARRAY_MATCHER_REGEX)
  if (arrayMatch != null) {
    const matchEnd = arrayMatch.index + arrayMatch[0].length
    if (!isMatcherPropertyDelimiter(configObject, matchEnd)) return { forceRuntime: true }
    const isQuotedKey = isQuotedMatcherKeyMatch(arrayMatch[0])
    return parseStaticStringArrayBody(isQuotedKey ? arrayMatch[2] : arrayMatch[1])
  }

  if (findTopLevelMatcherMatch(configObject, MATCHER_SHORTHAND_REGEX) != null) {
    return resolveModuleLevelMatcherBinding(code) ?? { forceRuntime: true }
  }

  if (
    findTopLevelMatcherMatch(configObject, /matcher\s*:/) != null ||
    findTopLevelMatcherMatch(configObject, QUOTED_MATCHER_KEY_REGEX) != null
  ) {
    return { forceRuntime: true }
  }

  return { forceRuntime: false }
}
// oxlint-enable typescript/prefer-readonly-parameter-types

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
