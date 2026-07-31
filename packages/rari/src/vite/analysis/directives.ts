const REGEX_KEYWORDS = new Set([
  'return',
  'throw',
  'case',
  'typeof',
  'instanceof',
  'new',
  'delete',
  'void',
  'in',
  'of',
])

const CH_SPACE = 32
const CH_TAB = 9
const CH_CR = 13
const CH_LF = 10
const CH_LINE_SEP = 0x2028
const CH_PARA_SEP = 0x2029
const CH_BOM = 65279 // U+FEFF
const CH_SLASH = 47
const CH_STAR = 42
const CH_BACKSLASH = 92
const CH_SINGLE_QUOTE = 39
const CH_DOUBLE_QUOTE = 34
const CH_BACKTICK = 96
const CH_SEMICOLON = 59
const CH_OPEN_BRACE = 123
const CH_CLOSE_BRACE = 125
const CH_OPEN_PAREN = 40
const CH_CLOSE_PAREN = 41
const CH_OPEN_BRACKET = 91
const CH_CLOSE_BRACKET = 93
const CH_COMMA = 44
const CH_EQUALS = 61
const CH_COLON = 58
const CH_QUESTION = 63
const CH_EXCL = 33
const CH_PLUS = 43
const CH_MINUS = 45
const CH_PERCENT = 37
const CH_AMP = 38
const CH_PIPE = 124
const CH_CARET = 94
const CH_TILDE = 126
const CH_LT = 60
const CH_GT = 62
const CH_DOT = 46
const CH_UNDERSCORE = 95
const CH_DOLLAR = 36

const CH_LOWER_A = 97
const CH_LOWER_Z = 122
const CH_UPPER_A = 65
const CH_UPPER_Z = 90
const CH_0 = 48
const CH_9 = 57

function isWhitespaceCode(ch: number): boolean {
  return (
    ch === CH_SPACE ||
    ch === CH_TAB ||
    ch === CH_CR ||
    ch === CH_LF ||
    ch === CH_LINE_SEP ||
    ch === CH_PARA_SEP ||
    ch === CH_BOM
  )
}

function isLineTerminatorCode(ch: number): boolean {
  return ch === CH_CR || ch === CH_LF || ch === CH_LINE_SEP || ch === CH_PARA_SEP
}

function isIdentifierPartCode(ch: number): boolean {
  return (
    (ch >= CH_LOWER_A && ch <= CH_LOWER_Z) ||
    (ch >= CH_UPPER_A && ch <= CH_UPPER_Z) ||
    (ch >= CH_0 && ch <= CH_9) ||
    ch === CH_UNDERSCORE ||
    ch === CH_DOLLAR
  )
}

function isIdentifierStartCode(ch: number): boolean {
  return (
    (ch >= CH_LOWER_A && ch <= CH_LOWER_Z) ||
    (ch >= CH_UPPER_A && ch <= CH_UPPER_Z) ||
    ch === CH_UNDERSCORE ||
    ch === CH_DOLLAR
  )
}

function skipWhitespace(source: string, i: number, len: number): number {
  while (i < len && isWhitespaceCode(source.charCodeAt(i))) {
    i++
  }

  return i
}

function skipTrivia(source: string, i: number, len: number): number {
  while (i < len) {
    const next = skipWhitespace(source, i, len)
    if (next !== i) {
      i = next
      continue
    }
    const ch = source.charCodeAt(i)
    if (ch === CH_SLASH && source.charCodeAt(i + 1) === CH_SLASH) {
      i = skipSingleLineComment(source, i, len)
      continue
    }
    if (ch === CH_SLASH && source.charCodeAt(i + 1) === CH_STAR) {
      i = skipMultiLineComment(source, i, len)
      continue
    }
    break
  }

  return i
}

function skipSingleLineComment(source: string, i: number, len: number): number {
  while (i < len && !isLineTerminatorCode(source.charCodeAt(i))) {
    i++
  }

  return i
}

function skipMultiLineComment(source: string, i: number, len: number): number {
  i += 2
  while (
    i < len - 1 &&
    (source.charCodeAt(i) !== CH_STAR || source.charCodeAt(i + 1) !== CH_SLASH)
  ) {
    i++
  }

  return i + 2
}

function skipString(source: string, i: number, len: number, quoteCode: number): number {
  i++
  while (i < len) {
    const ch = source.charCodeAt(i)
    if (ch === CH_BACKSLASH) {
      i += 2
      continue
    }
    if (ch === quoteCode) {
      return i + 1
    }
    i++
  }

  return i
}

function skipJSX(source: string, i: number, len: number): number {
  i++

  const isClosingTag = source.charCodeAt(i) === CH_SLASH
  if (isClosingTag) {
    i++
  }

  while (i < len) {
    const ch = source.charCodeAt(i)
    if (isIdentifierPartCode(ch) || ch === CH_DOT || ch === CH_MINUS) {
      i++
    } else {
      break
    }
  }

  let depth = isClosingTag ? 0 : 1

  while (i < len && depth > 0) {
    const ch = source.charCodeAt(i)
    if (ch === CH_SINGLE_QUOTE || ch === CH_DOUBLE_QUOTE || ch === CH_BACKTICK) {
      i = skipString(source, i, len, ch)
      continue
    }

    if (ch === CH_OPEN_BRACE) {
      i++
      let braceDepth = 1
      while (i < len && braceDepth > 0) {
        const bch = source.charCodeAt(i)
        if (bch === CH_SINGLE_QUOTE || bch === CH_DOUBLE_QUOTE || bch === CH_BACKTICK) {
          i = skipString(source, i, len, bch)
          continue
        }
        if (bch === CH_OPEN_BRACE) braceDepth++
        if (bch === CH_CLOSE_BRACE) braceDepth--
        i++
      }
      continue
    }

    if (ch === CH_SLASH && source.charCodeAt(i + 1) === CH_GT) {
      depth--
      i += 2
      continue
    }

    if (ch === CH_GT) {
      i++
      if (isClosingTag) {
        depth--
      }
      continue
    }

    if (ch === CH_LT) {
      const nextCh = source.charCodeAt(i + 1)
      if (
        nextCh === CH_SLASH ||
        nextCh === CH_DOT ||
        nextCh === CH_GT ||
        isIdentifierStartCode(nextCh)
      ) {
        if (nextCh === CH_SLASH) {
          depth--
          i++
        } else if (nextCh !== CH_EXCL) {
          depth++
        }
        i++
        while (i < len) {
          const tch = source.charCodeAt(i)
          if (isIdentifierPartCode(tch) || tch === CH_DOT || tch === CH_MINUS) {
            i++
          } else {
            break
          }
        }
        continue
      }
      i++
      continue
    }

    i++
  }

  return i
}

function regionEquals(source: string, offset: number, target: string): boolean {
  for (let k = 0; k < target.length; k++) {
    if (source.charCodeAt(offset + k) !== target.charCodeAt(k)) return false
  }

  return true
}

export interface DirectiveResult {
  readonly hasUseClient: boolean
  readonly hasUseServer: boolean
}

function isKeywordAt(source: string, i: number, keyword: string): boolean {
  if (i + keyword.length > source.length) return false

  for (let k = 0; k < keyword.length; k++) {
    if (source.charCodeAt(i + k) !== keyword.charCodeAt(k)) return false
  }

  const before = i > 0 ? source.charCodeAt(i - 1) : -1
  const afterIndex = i + keyword.length
  const after = afterIndex < source.length ? source.charCodeAt(afterIndex) : -1
  if (before !== -1 && isIdentifierPartCode(before)) return false
  if (after !== -1 && isIdentifierPartCode(after)) return false

  return true
}

function readImportModuleSpecifier(
  source: string,
  i: number,
  len: number,
): { source: string; end: number } | null {
  const pos = skipTrivia(source, i, len)
  if (pos >= len) return null

  const ch = source.charCodeAt(pos)
  if (ch !== CH_SINGLE_QUOTE && ch !== CH_DOUBLE_QUOTE) return null

  const strStart = pos + 1
  const strEnd = skipString(source, pos, len, ch)
  if (strEnd <= strStart) return null

  return {
    source: source.slice(strStart, strEnd - 1),
    end: strEnd,
  }
}

function collectImportSourcesAt(
  source: string,
  i: number,
  len: number,
): { sources: string[]; end: number } {
  if (!isKeywordAt(source, i, 'import')) return { sources: [], end: i + 6 }

  let pos = i + 6
  pos = skipTrivia(source, pos, len)

  if (isKeywordAt(source, pos, 'type')) pos = skipTrivia(source, pos + 4, len)

  if (source.charCodeAt(pos) === CH_DOT) {
    pos++
    while (pos < len && isIdentifierPartCode(source.charCodeAt(pos))) pos++

    while (pos < len && source.charCodeAt(pos) === CH_DOT) {
      pos++
      while (pos < len && isIdentifierPartCode(source.charCodeAt(pos))) pos++
    }

    return { sources: [], end: pos }
  }

  if (source.charCodeAt(pos) === CH_OPEN_PAREN) {
    pos++
    const specifier = readImportModuleSpecifier(source, pos, len)
    if (specifier) return { sources: [specifier.source], end: specifier.end }

    return { sources: [], end: pos }
  }

  const sideEffect = readImportModuleSpecifier(source, pos, len)
  if (sideEffect) return { sources: [sideEffect.source], end: sideEffect.end }

  let depth = 0
  while (pos < len) {
    pos = skipTrivia(source, pos, len)
    if (pos >= len) break

    const ch = source.charCodeAt(pos)

    if (depth === 0 && isKeywordAt(source, pos, 'from')) {
      const specifier = readImportModuleSpecifier(source, pos + 4, len)
      if (specifier) return { sources: [specifier.source], end: specifier.end }

      break
    }

    if (ch === CH_SINGLE_QUOTE || ch === CH_DOUBLE_QUOTE || ch === CH_BACKTICK) {
      pos = skipString(source, pos, len, ch)
      continue
    }

    if (ch === CH_OPEN_BRACE || ch === CH_OPEN_PAREN || ch === CH_OPEN_BRACKET) depth++
    else if (ch === CH_CLOSE_BRACE || ch === CH_CLOSE_PAREN || ch === CH_CLOSE_BRACKET)
      depth = Math.max(0, depth - 1)

    pos++
  }

  return { sources: [], end: pos }
}

const EXPORT_E = 101
const EXPORT_X = 120
const EXPORT_P = 112
const EXPORT_O = 111
const EXPORT_R = 114
const EXPORT_T = 116

const DEFAULT_D = 100
const DEFAULT_E2 = 101
const DEFAULT_F = 102
const DEFAULT_A = 97
const DEFAULT_U = 117
const DEFAULT_L = 108
const DEFAULT_T2 = 116

const AS_A = 97
const AS_S = 115

function isExportAt(source: string, i: number): boolean {
  return (
    source.charCodeAt(i) === EXPORT_E &&
    source.charCodeAt(i + 1) === EXPORT_X &&
    source.charCodeAt(i + 2) === EXPORT_P &&
    source.charCodeAt(i + 3) === EXPORT_O &&
    source.charCodeAt(i + 4) === EXPORT_R &&
    source.charCodeAt(i + 5) === EXPORT_T
  )
}

function isDefaultAt(source: string, i: number): boolean {
  return (
    source.charCodeAt(i) === DEFAULT_D &&
    source.charCodeAt(i + 1) === DEFAULT_E2 &&
    source.charCodeAt(i + 2) === DEFAULT_F &&
    source.charCodeAt(i + 3) === DEFAULT_A &&
    source.charCodeAt(i + 4) === DEFAULT_U &&
    source.charCodeAt(i + 5) === DEFAULT_L &&
    source.charCodeAt(i + 6) === DEFAULT_T2
  )
}

export interface ModuleAnalysis {
  readonly directives: DirectiveResult
  readonly topLevelUseClient: boolean
  readonly topLevelUseServer: boolean
  readonly hasDefaultExport: boolean
  readonly hasComponentExport: boolean
  readonly importSources: readonly string[]
}

export function analyzeModuleSource(source: string): ModuleAnalysis {
  const directives = { hasUseClient: false, hasUseServer: false }
  let topLevelUseClient = false
  let topLevelUseServer = false
  let hasDefaultExportResult = false
  let hasComponentExportResult = false
  const importSources: string[] = []
  let directivesPhase = true
  let sawFirstDirective = false

  let i = 0
  const len = source.length

  while (i < len) {
    const ch = source.charCodeAt(i)

    if (directivesPhase && (ch === CH_SINGLE_QUOTE || ch === CH_DOUBLE_QUOTE)) {
      const stringStart = i + 1
      const stringEnd = skipString(source, i, len, ch)
      if (stringEnd <= stringStart) {
        directivesPhase = false
        i++
        continue
      }

      const contentLen = stringEnd - 1 - stringStart
      const isUseClient = contentLen === 10 && regionEquals(source, stringStart, 'use client')
      const isUseServer = contentLen === 10 && regionEquals(source, stringStart, 'use server')

      if (!sawFirstDirective) {
        sawFirstDirective = true
        topLevelUseClient = isUseClient
        topLevelUseServer = isUseServer
      }

      if (isUseClient) directives.hasUseClient = true
      if (isUseServer) directives.hasUseServer = true

      let j = stringEnd
      let stillDirective = false
      while (j < len) {
        const jch = source.charCodeAt(j)
        if (isWhitespaceCode(jch) && !isLineTerminatorCode(jch)) {
          j++
          continue
        }
        if (isLineTerminatorCode(jch) || jch === CH_SEMICOLON) {
          stillDirective = true
          i = j + 1
          break
        }
        if (jch === CH_SLASH && source.charCodeAt(j + 1) === CH_SLASH) {
          j = skipSingleLineComment(source, j, len)
          continue
        }
        if (jch === CH_SLASH && source.charCodeAt(j + 1) === CH_STAR) {
          j = skipMultiLineComment(source, j, len)
          continue
        }

        directivesPhase = false
        stillDirective = false
        break
      }

      if (!stillDirective) {
        if (j >= len) directivesPhase = false
        i = directivesPhase ? stringEnd : j
        continue
      }

      continue
    }

    const skipped = skipNonCodeToken(source, i, len)
    if (skipped !== -1) {
      if (directivesPhase && !isTriviaOrCommentStart(source, i)) directivesPhase = false
      i = skipped
      continue
    }

    directivesPhase = false

    if (isKeywordAt(source, i, 'import')) {
      const collected = collectImportSourcesAt(source, i, len)
      for (const importSource of collected.sources) importSources.push(importSource)
      i = collected.end
      continue
    }

    if (isExportAt(source, i)) {
      const afterExport = i + 6
      if (afterExport < len) {
        const afterCh = source.charCodeAt(afterExport)
        if (
          isWhitespaceCode(afterCh) ||
          afterCh === CH_OPEN_BRACE ||
          (afterCh === CH_SLASH &&
            (source.charCodeAt(afterExport + 1) === CH_SLASH ||
              source.charCodeAt(afterExport + 1) === CH_STAR))
        ) {
          const j = skipTrivia(source, afterExport, len)

          if (isDefaultAt(source, j)) {
            hasDefaultExportResult = true
            hasComponentExportResult = true
            const afterDefault = j + 7
            if (afterDefault >= len || !isIdentifierPartCode(source.charCodeAt(afterDefault))) {
              i = afterExport
              continue
            }
          }

          if (source.charCodeAt(j) === CH_OPEN_BRACE) {
            let k = j + 1
            while (k < len) {
              k = skipTrivia(source, k, len)

              if (source.charCodeAt(k) === CH_CLOSE_BRACE) break

              const identStart = k
              while (k < len && isIdentifierPartCode(source.charCodeAt(k))) k++
              const identLen = k - identStart

              if (identLen === 0) break

              k = skipTrivia(source, k, len)

              let hasAlias = false
              if (source.charCodeAt(k) === AS_A && source.charCodeAt(k + 1) === AS_S) {
                hasAlias = true
                const afterAs = k + 2
                if (afterAs < len && !isIdentifierPartCode(source.charCodeAt(afterAs))) {
                  k = skipTrivia(source, afterAs, len)
                  const aliasStart = k
                  while (k < len && isIdentifierPartCode(source.charCodeAt(k))) k++
                  if (k - aliasStart === 7 && isDefaultAt(source, aliasStart)) {
                    hasDefaultExportResult = true
                    hasComponentExportResult = true
                  }
                }
              }

              if (!hasAlias && identLen === 7 && isDefaultAt(source, identStart)) {
                hasDefaultExportResult = true
                hasComponentExportResult = true
              }

              if (source.charCodeAt(k) === CH_COMMA) {
                k++
                continue
              }

              if (source.charCodeAt(k) === CH_CLOSE_BRACE) break

              k++
            }
          } else if (
            isKeywordAt(source, j, 'async') ||
            isKeywordAt(source, j, 'function') ||
            isKeywordAt(source, j, 'class')
          ) {
            hasComponentExportResult = true
          }
        }
      }
    }

    i++
  }

  return {
    directives,
    topLevelUseClient,
    topLevelUseServer,
    hasDefaultExport: hasDefaultExportResult,
    hasComponentExport: hasComponentExportResult,
    importSources: [...new Set(importSources)],
  }
}

export interface ScannedImportSpecifier {
  readonly imported: string
  readonly local: string
  readonly typeOnly: boolean
}

export interface ScannedImport {
  /** Offset of the `import` keyword. */
  readonly start: number
  /** Offset past the statement (includes a trailing semicolon when present). */
  readonly end: number
  readonly source: string
  readonly typeOnly: boolean
  readonly sideEffectOnly: boolean
  readonly defaultBinding: string | null
  readonly namespaceBinding: string | null
  readonly named: readonly ScannedImportSpecifier[]
}

function readIdentifier(
  source: string,
  i: number,
  len: number,
): { name: string; end: number } | null {
  if (i >= len || !isIdentifierStartCode(source.charCodeAt(i))) return null

  let j = i + 1
  while (j < len && isIdentifierPartCode(source.charCodeAt(j))) j++

  return { name: source.slice(i, j), end: j }
}

function consumeTrailingSemicolon(source: string, end: number, len: number): number {
  let pos = end
  while (pos < len) {
    const ch = source.charCodeAt(pos)
    if (ch === CH_SPACE || ch === CH_TAB) {
      pos++
      continue
    }
    if (ch === CH_SEMICOLON) return pos + 1
    break
  }

  return end
}

interface NamedSpecifierParse {
  readonly named: ScannedImportSpecifier[]
  readonly end: number
}

function parseNamedImportSpecifiers(
  source: string,
  openBrace: number,
  len: number,
): NamedSpecifierParse | null {
  const named: ScannedImportSpecifier[] = []
  let pos = openBrace + 1

  for (;;) {
    pos = skipTrivia(source, pos, len)
    if (pos >= len) return null

    if (source.charCodeAt(pos) === CH_CLOSE_BRACE) return { named, end: pos + 1 }

    // Inline type specifier: `type` followed by another specifier name is a
    // modifier. A binding literally named `type` still parses as a name below.
    let specTypeOnly = false
    if (isKeywordAt(source, pos, 'type')) {
      const after = skipTrivia(source, pos + 4, len)
      const afterCh = source.charCodeAt(after)
      const startsSpecifier =
        afterCh === CH_SINGLE_QUOTE ||
        afterCh === CH_DOUBLE_QUOTE ||
        readIdentifier(source, after, len) !== null
      if (startsSpecifier && !isKeywordAt(source, after, 'as')) {
        specTypeOnly = true
        pos = after
      }
    }

    let imported: string
    const importedCh = source.charCodeAt(pos)
    if (importedCh === CH_SINGLE_QUOTE || importedCh === CH_DOUBLE_QUOTE) {
      const strEnd = skipString(source, pos, len, importedCh)
      imported = source.slice(pos + 1, strEnd - 1)
      pos = strEnd
    } else {
      const ident = readIdentifier(source, pos, len)
      if (!ident) return null
      imported = ident.name
      pos = ident.end
    }

    pos = skipTrivia(source, pos, len)

    let local = imported
    if (isKeywordAt(source, pos, 'as')) {
      pos = skipTrivia(source, pos + 2, len)
      const alias = readIdentifier(source, pos, len)
      if (!alias) return null
      local = alias.name
      pos = skipTrivia(source, alias.end, len)
    }

    named.push({ imported, local, typeOnly: specTypeOnly })

    const ch = source.charCodeAt(pos)
    if (ch === CH_COMMA) {
      pos++
      continue
    }
    if (ch === CH_CLOSE_BRACE) return { named, end: pos + 1 }

    return null
  }
}

function parseImportStatementAt(source: string, start: number, len: number): ScannedImport | null {
  let pos = skipTrivia(source, start + 6, len)
  if (pos >= len) return null

  const ch = source.charCodeAt(pos)
  // Dynamic import or import.meta — not a static statement.
  if (ch === CH_OPEN_PAREN || ch === CH_DOT) return null

  if (ch === CH_SINGLE_QUOTE || ch === CH_DOUBLE_QUOTE) {
    const spec = readImportModuleSpecifier(source, pos, len)
    if (!spec) return null

    return {
      start,
      end: consumeTrailingSemicolon(source, spec.end, len),
      source: spec.source,
      typeOnly: false,
      sideEffectOnly: true,
      defaultBinding: null,
      namespaceBinding: null,
      named: [],
    }
  }

  let typeOnly = false
  // `import type ...` is type-only unless `type` is itself the default
  // binding (`import type from './x'` or `import type, { x } from './x'`).
  if (isKeywordAt(source, pos, 'type')) {
    const after = skipTrivia(source, pos + 4, len)
    if (!isKeywordAt(source, after, 'from') && source.charCodeAt(after) !== CH_COMMA) {
      typeOnly = true
      pos = after
    }
  }

  let defaultBinding: string | null = null
  let namespaceBinding: string | null = null
  let named: ScannedImportSpecifier[] = []

  let clauseCh = source.charCodeAt(pos)

  if (clauseCh !== CH_OPEN_BRACE && clauseCh !== CH_STAR) {
    const ident = readIdentifier(source, pos, len)
    if (!ident) return null

    defaultBinding = ident.name
    pos = skipTrivia(source, ident.end, len)

    if (source.charCodeAt(pos) === CH_COMMA) {
      pos = skipTrivia(source, pos + 1, len)
      clauseCh = source.charCodeAt(pos)
    } else {
      clauseCh = -1
    }
  }

  if (clauseCh === CH_STAR) {
    pos = skipTrivia(source, pos + 1, len)
    if (!isKeywordAt(source, pos, 'as')) return null

    pos = skipTrivia(source, pos + 2, len)
    const ns = readIdentifier(source, pos, len)
    if (!ns) return null

    namespaceBinding = ns.name
    pos = ns.end
  } else if (clauseCh === CH_OPEN_BRACE) {
    const parsed = parseNamedImportSpecifiers(source, pos, len)
    if (!parsed) return null

    named = parsed.named
    pos = parsed.end
  }

  pos = skipTrivia(source, pos, len)
  if (!isKeywordAt(source, pos, 'from')) return null

  const spec = readImportModuleSpecifier(source, pos + 4, len)
  if (!spec) return null

  return {
    start,
    end: consumeTrailingSemicolon(source, spec.end, len),
    source: spec.source,
    typeOnly,
    sideEffectOnly: false,
    defaultBinding,
    namespaceBinding,
    named,
  }
}

/**
 * Scan static import statements with byte spans and full specifier structure
 * (default, namespace, named with aliases, type-only, side-effect, multi-line).
 * Same lexer discipline as analyzeModuleSource: comments, strings, regex
 * literals, and JSX are skipped, so imports inside them never match.
 */
export function scanImportStatements(source: string): ScannedImport[] {
  const imports: ScannedImport[] = []
  let i = 0
  const len = source.length

  while (i < len) {
    const skipped = skipNonCodeToken(source, i, len)
    if (skipped !== -1) {
      i = skipped
      continue
    }

    if (isKeywordAt(source, i, 'import')) {
      const parsed = parseImportStatementAt(source, i, len)
      if (parsed) {
        imports.push(parsed)
        i = parsed.end
        continue
      }
      i += 6
      continue
    }

    i++
  }

  return imports
}

export function getDirectives(source: string): DirectiveResult {
  return analyzeModuleSource(source).directives
}

export function hasTopLevelUseServerDirective(source: string): boolean {
  return analyzeModuleSource(source).topLevelUseServer
}

export function hasTopLevelUseClientDirective(source: string): boolean {
  return analyzeModuleSource(source).topLevelUseClient
}

export function hasDefaultExport(source: string): boolean {
  return analyzeModuleSource(source).hasDefaultExport
}

function canPrecedeRegexCode(ch: number): boolean {
  return (
    ch === CH_OPEN_PAREN ||
    ch === CH_OPEN_BRACKET ||
    ch === CH_OPEN_BRACE ||
    ch === CH_COMMA ||
    ch === CH_SEMICOLON ||
    ch === CH_EQUALS ||
    ch === CH_COLON ||
    ch === CH_QUESTION ||
    ch === CH_EXCL ||
    ch === CH_PLUS ||
    ch === CH_MINUS ||
    ch === CH_STAR ||
    ch === CH_PERCENT ||
    ch === CH_AMP ||
    ch === CH_PIPE ||
    ch === CH_CARET ||
    ch === CH_TILDE ||
    ch === CH_LT ||
    ch === CH_GT
  )
}

function getPreviousToken(source: string, pos: number): string | undefined {
  let i = pos - 1

  while (i >= 0) {
    const ch = source.charCodeAt(i)
    if (isWhitespaceCode(ch)) {
      i--
      continue
    }

    if (i >= 1 && ch === CH_SLASH && source.charCodeAt(i - 1) === CH_STAR) {
      i -= 2
      while (i >= 1) {
        if (source.charCodeAt(i) === CH_STAR && source.charCodeAt(i - 1) === CH_SLASH) {
          i -= 2
          break
        }
        i--
      }
      if (i < 0) return undefined
      continue
    }

    if (i >= 1 && ch === CH_SLASH && source.charCodeAt(i - 1) === CH_SLASH) {
      i -= 2
      continue
    }

    let checkPos = i
    while (
      checkPos >= 0 &&
      source.charCodeAt(checkPos) !== CH_LF &&
      source.charCodeAt(checkPos) !== CH_CR
    ) {
      checkPos--
    }
    let afterNewline = checkPos + 1
    while (
      afterNewline < i &&
      (source.charCodeAt(afterNewline) === CH_SPACE || source.charCodeAt(afterNewline) === CH_TAB)
    ) {
      afterNewline++
    }
    if (
      afterNewline < i &&
      source.charCodeAt(afterNewline) === CH_SLASH &&
      source.charCodeAt(afterNewline + 1) === CH_SLASH
    ) {
      i = afterNewline - 1
      continue
    }

    break
  }

  if (i < 0) return undefined
  if (!isIdentifierPartCode(source.charCodeAt(i))) return undefined

  const end = i
  while (i >= 0 && isIdentifierPartCode(source.charCodeAt(i))) {
    i--
  }

  return source.slice(i + 1, end + 1)
}

function getPreviousNonTriviaCharCode(source: string, pos: number): number {
  let i = pos - 1
  while (i >= 0) {
    const ch = source.charCodeAt(i)
    if (isWhitespaceCode(ch)) {
      i--
      continue
    }

    if (i >= 1 && ch === CH_SLASH && source.charCodeAt(i - 1) === CH_STAR) {
      i -= 2
      while (i >= 1) {
        if (source.charCodeAt(i) === CH_STAR && source.charCodeAt(i - 1) === CH_SLASH) {
          i -= 2
          break
        }
        i--
      }
      if (i < 0) return -1
      continue
    }

    if (i >= 1 && ch === CH_SLASH && source.charCodeAt(i - 1) === CH_SLASH) {
      i -= 2
      continue
    }

    let checkPos = i
    while (
      checkPos >= 0 &&
      source.charCodeAt(checkPos) !== CH_LF &&
      source.charCodeAt(checkPos) !== CH_CR
    ) {
      checkPos--
    }
    let afterNewline = checkPos + 1
    while (
      afterNewline < i &&
      (source.charCodeAt(afterNewline) === CH_SPACE || source.charCodeAt(afterNewline) === CH_TAB)
    ) {
      afterNewline++
    }
    if (
      afterNewline < i &&
      source.charCodeAt(afterNewline) === CH_SLASH &&
      source.charCodeAt(afterNewline + 1) === CH_SLASH
    ) {
      i = afterNewline - 1
      continue
    }

    return ch
  }

  return -1
}

function canPrecedeRegexWithKeywords(source: string, pos: number): boolean {
  const prevCharCode = getPreviousNonTriviaCharCode(source, pos)

  if (prevCharCode === -1 || canPrecedeRegexCode(prevCharCode)) {
    return true
  }

  const prevToken = getPreviousToken(source, pos)
  if (prevToken != null && prevToken !== '') {
    return REGEX_KEYWORDS.has(prevToken)
  }

  return false
}

function skipRegex(source: string, i: number, len: number): number {
  i++
  let inCharClass = false

  while (i < len) {
    const ch = source.charCodeAt(i)
    if (ch === CH_BACKSLASH) {
      i += 2
      continue
    }

    if (inCharClass) {
      if (ch === CH_CLOSE_BRACKET) {
        inCharClass = false
      }
      i++
      continue
    }

    if (ch === CH_OPEN_BRACKET) {
      inCharClass = true
      i++
      continue
    }

    if (ch === CH_SLASH) {
      i++
      while (i < len && isIdentifierPartCode(source.charCodeAt(i))) {
        i++
      }

      return i
    }

    if (isLineTerminatorCode(ch)) {
      return i
    }

    i++
  }

  return i
}

/**
 * If `source[i]` starts whitespace, a comment, string, regex literal, or JSX,
 * return the offset past that token. Returns -1 when the position is code.
 */
function skipNonCodeToken(source: string, i: number, len: number): number {
  if (i >= len) return -1

  const ch = source.charCodeAt(i)

  if (isWhitespaceCode(ch)) return i + 1

  if (ch === CH_SLASH && source.charCodeAt(i + 1) === CH_SLASH)
    return skipSingleLineComment(source, i, len)

  if (ch === CH_SLASH && source.charCodeAt(i + 1) === CH_STAR)
    return skipMultiLineComment(source, i, len)

  if (ch === CH_SINGLE_QUOTE || ch === CH_DOUBLE_QUOTE || ch === CH_BACKTICK)
    return skipString(source, i, len, ch)

  if (
    ch === CH_SLASH &&
    source.charCodeAt(i + 1) !== CH_SLASH &&
    source.charCodeAt(i + 1) !== CH_STAR &&
    canPrecedeRegexWithKeywords(source, i)
  ) {
    return skipRegex(source, i, len)
  }

  if (ch === CH_LT) {
    const nextCh = source.charCodeAt(i + 1)
    if (
      nextCh === CH_SLASH ||
      nextCh === CH_DOT ||
      nextCh === CH_GT ||
      isIdentifierStartCode(nextCh)
    ) {
      return skipJSX(source, i, len)
    }
  }

  return -1
}

function isTriviaOrCommentStart(source: string, i: number): boolean {
  const ch = source.charCodeAt(i)
  if (isWhitespaceCode(ch)) return true
  return (
    ch === CH_SLASH &&
    (source.charCodeAt(i + 1) === CH_SLASH || source.charCodeAt(i + 1) === CH_STAR)
  )
}

export interface ExportDefaultValueLocation {
  /** Start of the `export` keyword. */
  readonly exportStart: number
  /** Start of the exported value / declaration after `default`. */
  readonly valueStart: number
  /** End of the exported value (before an optional trailing semicolon). */
  readonly valueEnd: number
  /** End of the statement including a trailing semicolon when present. */
  readonly statementEnd: number
  /**
   * Local binding name when the export is a named function/class declaration;
   * null for expression exports that need a temporary binding.
   */
  readonly bindingName: string | null
}

/**
 * Locate the first `export default …` statement with a correctly spanned
 * expression body (brace/paren/bracket depth, strings, comments, JSX, regex).
 * Avoids the classic `[^;]+` trap that truncates arrow-function bodies.
 */
export function locateExportDefaultValue(source: string): ExportDefaultValueLocation | null {
  const len = source.length
  let i = 0

  while (i < len) {
    const skipped = skipNonCodeToken(source, i, len)
    if (skipped !== -1) {
      i = skipped
      continue
    }

    if (isKeywordAt(source, i, 'export')) {
      const afterExport = skipTrivia(source, i + 6, len)
      if (!isKeywordAt(source, afterExport, 'default')) {
        i++
        continue
      }

      const valueStart = skipTrivia(source, afterExport + 7, len)
      let pos = valueStart
      let bindingName: string | null = null

      if (isKeywordAt(source, pos, 'async')) {
        const afterAsync = skipTrivia(source, pos + 5, len)
        if (isKeywordAt(source, afterAsync, 'function')) pos = afterAsync
      }

      if (isKeywordAt(source, pos, 'function') || isKeywordAt(source, pos, 'class')) {
        const keywordLen = isKeywordAt(source, pos, 'function') ? 8 : 5
        let afterKeyword = skipTrivia(source, pos + keywordLen, len)
        if (source.charCodeAt(afterKeyword) === CH_STAR) {
          afterKeyword = skipTrivia(source, afterKeyword + 1, len)
        }
        const name = readIdentifier(source, afterKeyword, len)
        if (name) bindingName = name.name
      }

      const valueEnd = scanExportDefaultValueEnd(source, valueStart, len)
      const statementEnd = consumeTrailingSemicolon(source, valueEnd, len)

      return {
        exportStart: i,
        valueStart,
        valueEnd,
        statementEnd,
        bindingName,
      }
    }

    i++
  }

  return null
}

function isExpressionContinuationAt(source: string, i: number, len: number): boolean {
  if (i >= len) return false

  const ch = source.charCodeAt(i)
  const next = i + 1 < len ? source.charCodeAt(i + 1) : -1

  if (ch === CH_DOT) return true
  if (ch === CH_QUESTION && next === CH_DOT) return true
  if (ch === CH_OPEN_PAREN || ch === CH_OPEN_BRACKET || ch === CH_BACKTICK) return true
  if (ch === CH_EQUALS && next === CH_GT) return true

  if (
    ch === CH_PLUS ||
    ch === CH_MINUS ||
    ch === CH_STAR ||
    ch === CH_SLASH ||
    ch === CH_PERCENT ||
    ch === CH_AMP ||
    ch === CH_PIPE ||
    ch === CH_CARET ||
    ch === CH_LT ||
    ch === CH_GT ||
    ch === CH_EQUALS ||
    ch === CH_EXCL ||
    ch === CH_QUESTION ||
    ch === CH_COLON ||
    ch === CH_COMMA ||
    ch === CH_TILDE
  ) {
    return true
  }

  return (
    isKeywordAt(source, i, 'instanceof') ||
    isKeywordAt(source, i, 'in') ||
    isKeywordAt(source, i, 'of') ||
    isKeywordAt(source, i, 'as')
  )
}

const NON_TERMINATING_EXPR_KEYWORDS = new Set(['typeof', 'void', 'delete', 'await', 'yield', 'new'])

function scanExportDefaultValueEnd(source: string, start: number, len: number): number {
  let i = start
  let paren = 0
  let brace = 0
  let bracket = 0
  let lastCanTerminate = false

  while (i < len) {
    const ch = source.charCodeAt(i)

    if (ch === CH_SINGLE_QUOTE || ch === CH_DOUBLE_QUOTE || ch === CH_BACKTICK) {
      i = skipString(source, i, len, ch)
      lastCanTerminate = true
      continue
    }

    if (ch === CH_SLASH && source.charCodeAt(i + 1) === CH_SLASH) {
      i = skipSingleLineComment(source, i, len)
      continue
    }

    if (ch === CH_SLASH && source.charCodeAt(i + 1) === CH_STAR) {
      i = skipMultiLineComment(source, i, len)
      continue
    }

    if (
      ch === CH_SLASH &&
      source.charCodeAt(i + 1) !== CH_SLASH &&
      source.charCodeAt(i + 1) !== CH_STAR &&
      canPrecedeRegexWithKeywords(source, i)
    ) {
      i = skipRegex(source, i, len)
      lastCanTerminate = true
      continue
    }

    if (ch === CH_LT && paren === 0 && brace === 0 && bracket === 0) {
      const nextCh = source.charCodeAt(i + 1)
      if (
        nextCh === CH_SLASH ||
        nextCh === CH_DOT ||
        nextCh === CH_GT ||
        isIdentifierStartCode(nextCh)
      ) {
        i = skipJSX(source, i, len)
        lastCanTerminate = true
        continue
      }
    }

    if (ch === CH_OPEN_PAREN) {
      paren++
      lastCanTerminate = false
      i++
      continue
    }
    if (ch === CH_CLOSE_PAREN) {
      paren = Math.max(0, paren - 1)
      lastCanTerminate = true
      i++
      continue
    }
    if (ch === CH_OPEN_BRACE) {
      brace++
      lastCanTerminate = false
      i++
      continue
    }
    if (ch === CH_CLOSE_BRACE) {
      brace = Math.max(0, brace - 1)
      lastCanTerminate = true
      i++
      // Declaration bodies (`function () {}`, `class {}`) end when the outer
      // brace closes at depth 0.
      if (paren === 0 && brace === 0 && bracket === 0) return i
      continue
    }
    if (ch === CH_OPEN_BRACKET) {
      bracket++
      lastCanTerminate = false
      i++
      continue
    }
    if (ch === CH_CLOSE_BRACKET) {
      bracket = Math.max(0, bracket - 1)
      lastCanTerminate = true
      i++
      continue
    }

    if (paren === 0 && brace === 0 && bracket === 0) {
      if (ch === CH_SEMICOLON) return i

      if (isLineTerminatorCode(ch)) {
        const afterNl = skipTrivia(source, i + 1, len)
        if (!lastCanTerminate || isExpressionContinuationAt(source, afterNl, len)) {
          i++
          continue
        }
        return i
      }
    }

    if (isIdentifierStartCode(ch)) {
      const id = readIdentifier(source, i, len)
      if (id) {
        lastCanTerminate = !NON_TERMINATING_EXPR_KEYWORDS.has(id.name)
        i = id.end
        continue
      }
    }

    if (ch >= CH_0 && ch <= CH_9) {
      lastCanTerminate = true
      i++
      while (i < len) {
        const d = source.charCodeAt(i)
        if (
          (d >= CH_0 && d <= CH_9) ||
          d === CH_DOT ||
          d === 110 /* n */ ||
          d === 101 /* e */ ||
          d === 69 /* E */
        ) {
          i++
          continue
        }
        break
      }
      continue
    }

    if (ch === CH_EQUALS && source.charCodeAt(i + 1) === CH_GT) {
      lastCanTerminate = false
      i += 2
      continue
    }

    if (
      ch === CH_DOT ||
      ch === CH_COMMA ||
      ch === CH_COLON ||
      ch === CH_QUESTION ||
      ch === CH_EQUALS ||
      ch === CH_PLUS ||
      ch === CH_MINUS ||
      ch === CH_STAR ||
      ch === CH_SLASH ||
      ch === CH_PERCENT ||
      ch === CH_AMP ||
      ch === CH_PIPE ||
      ch === CH_CARET ||
      ch === CH_EXCL ||
      ch === CH_TILDE ||
      ch === CH_LT ||
      ch === CH_GT
    ) {
      lastCanTerminate = false
      i++
      continue
    }

    i++
  }

  return i
}

/**
 * Rewrite `export default <expr>` to
 * `const <tempVar> = <expr>;\nexport default <tempVar>` so the binding can be
 * passed to registerServerReference. Named function/class defaults are left
 * alone (caller should register the declaration name directly).
 */
export function rewriteExportDefaultAsBinding(source: string, tempVarName: string): string | null {
  const located = locateExportDefaultValue(source)
  if (located == null || located.bindingName != null) return null

  const value = source.slice(located.valueStart, located.valueEnd).trimEnd()
  const rewritten = `const ${tempVarName} = ${value};\nexport default ${tempVarName}${source.slice(located.statementEnd)}`

  return source.slice(0, located.exportStart) + rewritten
}
