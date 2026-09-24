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

function skipJsxTagName(source: string, i: number, len: number): number {
  while (i < len) {
    const ch = source.charCodeAt(i)
    if (isIdentifierPartCode(ch) || ch === CH_DOT || ch === CH_MINUS) {
      i++
    } else {
      break
    }
  }
  return i
}

function skipJsxBraceExpression(source: string, i: number, len: number): number {
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
  return i
}

function applyJsxOpenOrCloseAtLt(
  source: string,
  i: number,
  len: number,
  depth: number,
): { i: number; depth: number } {
  const nextCh = source.charCodeAt(i + 1)
  if (
    nextCh !== CH_SLASH &&
    nextCh !== CH_DOT &&
    nextCh !== CH_GT &&
    !isIdentifierStartCode(nextCh)
  ) {
    return { i: i + 1, depth }
  }

  let nextDepth = depth
  if (nextCh === CH_SLASH) {
    nextDepth--
    i++
  } else if (nextCh !== CH_EXCL) {
    nextDepth++
  }
  i++
  return { i: skipJsxTagName(source, i, len), depth: nextDepth }
}

function advanceJsxInsideTag(
  source: string,
  i: number,
  len: number,
  depth: number,
  isClosingTag: boolean,
): { i: number; depth: number } {
  const ch = source.charCodeAt(i)
  if (ch === CH_SINGLE_QUOTE || ch === CH_DOUBLE_QUOTE || ch === CH_BACKTICK) {
    return { i: skipString(source, i, len, ch), depth }
  }
  if (ch === CH_OPEN_BRACE) {
    return { i: skipJsxBraceExpression(source, i, len), depth }
  }
  if (ch === CH_SLASH && source.charCodeAt(i + 1) === CH_GT) {
    return { i: i + 2, depth: depth - 1 }
  }
  if (ch === CH_GT) {
    return { i: i + 1, depth: isClosingTag ? depth - 1 : depth }
  }
  if (ch === CH_LT) {
    return applyJsxOpenOrCloseAtLt(source, i, len, depth)
  }
  return { i: i + 1, depth }
}

function skipJSX(source: string, i: number, len: number): number {
  i++

  const isClosingTag = source.charCodeAt(i) === CH_SLASH
  if (isClosingTag) {
    i++
  }

  i = skipJsxTagName(source, i, len)

  let depth = isClosingTag ? 0 : 1

  while (i < len && depth > 0) {
    ;({ i, depth } = advanceJsxInsideTag(source, i, len, depth, isClosingTag))
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

function skipImportMetaChain(source: string, pos: number, len: number): number {
  pos++
  while (pos < len && isIdentifierPartCode(source.charCodeAt(pos))) pos++

  while (pos < len && source.charCodeAt(pos) === CH_DOT) {
    pos++
    while (pos < len && isIdentifierPartCode(source.charCodeAt(pos))) pos++
  }

  return pos
}

function adjustImportClauseDepth(ch: number, depth: number): number {
  if (ch === CH_OPEN_BRACE || ch === CH_OPEN_PAREN || ch === CH_OPEN_BRACKET) return depth + 1
  if (ch === CH_CLOSE_BRACE || ch === CH_CLOSE_PAREN || ch === CH_CLOSE_BRACKET)
    return Math.max(0, depth - 1)
  return depth
}

function scanImportFromClause(
  source: string,
  pos: number,
  len: number,
): { sources: string[]; end: number } {
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

    depth = adjustImportClauseDepth(ch, depth)
    pos++
  }

  return { sources: [], end: pos }
}

function collectImportSourcesAt(
  source: string,
  i: number,
  len: number,
): { sources: string[]; end: number } {
  if (!isKeywordAt(source, i, 'import')) return { sources: [], end: i + 6 }

  let pos = skipTrivia(source, i + 6, len)

  if (isKeywordAt(source, pos, 'type')) pos = skipTrivia(source, pos + 4, len)

  if (source.charCodeAt(pos) === CH_DOT) {
    return { sources: [], end: skipImportMetaChain(source, pos, len) }
  }

  if (source.charCodeAt(pos) === CH_OPEN_PAREN) {
    pos++
    const specifier = readImportModuleSpecifier(source, pos, len)
    if (specifier) return { sources: [specifier.source], end: specifier.end }
    return { sources: [], end: pos }
  }

  const sideEffect = readImportModuleSpecifier(source, pos, len)
  if (sideEffect) return { sources: [sideEffect.source], end: sideEffect.end }

  return scanImportFromClause(source, pos, len)
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

interface ModuleScanState {
  directives: { hasUseClient: boolean; hasUseServer: boolean }
  topLevelUseClient: boolean
  topLevelUseServer: boolean
  hasDefaultExportResult: boolean
  hasComponentExportResult: boolean
  importSources: string[]
  directivesPhase: boolean
  sawFirstDirective: boolean
}

// oxlint-disable typescript/prefer-readonly-parameter-types
function markDirectiveFlags(
  state: ModuleScanState,
  isUseClient: boolean,
  isUseServer: boolean,
): void {
  if (!state.sawFirstDirective) {
    state.sawFirstDirective = true
    state.topLevelUseClient = isUseClient
    state.topLevelUseServer = isUseServer
  }
  if (isUseClient) state.directives.hasUseClient = true
  if (isUseServer) state.directives.hasUseServer = true
}

function advancePastDirectiveTerminator(
  source: string,
  stringEnd: number,
  len: number,
): { stillDirective: boolean; nextI: number; endDirectivesPhase: boolean } {
  let j = stringEnd
  while (j < len) {
    const jch = source.charCodeAt(j)
    if (isWhitespaceCode(jch) && !isLineTerminatorCode(jch)) {
      j++
      continue
    }
    if (isLineTerminatorCode(jch) || jch === CH_SEMICOLON) {
      return { stillDirective: true, nextI: j + 1, endDirectivesPhase: false }
    }
    if (jch === CH_SLASH && source.charCodeAt(j + 1) === CH_SLASH) {
      j = skipSingleLineComment(source, j, len)
      continue
    }
    if (jch === CH_SLASH && source.charCodeAt(j + 1) === CH_STAR) {
      j = skipMultiLineComment(source, j, len)
      continue
    }
    return { stillDirective: false, nextI: j, endDirectivesPhase: true }
  }
  return { stillDirective: false, nextI: j, endDirectivesPhase: j >= len }
}

function tryConsumeDirectiveString(
  source: string,
  i: number,
  len: number,
  state: ModuleScanState,
): number | null {
  if (!state.directivesPhase) return null
  const ch = source.charCodeAt(i)
  if (ch !== CH_SINGLE_QUOTE && ch !== CH_DOUBLE_QUOTE) return null

  const stringStart = i + 1
  const stringEnd = skipString(source, i, len, ch)
  if (stringEnd <= stringStart) {
    state.directivesPhase = false
    return i + 1
  }

  const contentLen = stringEnd - 1 - stringStart
  const isUseClient = contentLen === 10 && regionEquals(source, stringStart, 'use client')
  const isUseServer = contentLen === 10 && regionEquals(source, stringStart, 'use server')
  markDirectiveFlags(state, isUseClient, isUseServer)

  const term = advancePastDirectiveTerminator(source, stringEnd, len)
  if (term.stillDirective) return term.nextI

  if (term.endDirectivesPhase) state.directivesPhase = false
  return state.directivesPhase ? stringEnd : term.nextI
}

function isExportKeywordBoundary(source: string, afterExport: number, len: number): boolean {
  if (afterExport >= len) return false
  const afterCh = source.charCodeAt(afterExport)
  if (isWhitespaceCode(afterCh) || afterCh === CH_OPEN_BRACE) return true
  return (
    afterCh === CH_SLASH &&
    (source.charCodeAt(afterExport + 1) === CH_SLASH ||
      source.charCodeAt(afterExport + 1) === CH_STAR)
  )
}

function markDefaultExportFlags(state: ModuleScanState): void {
  state.hasDefaultExportResult = true
  state.hasComponentExportResult = true
}

function consumeExportAsAlias(
  source: string,
  k: number,
  len: number,
  state: ModuleScanState,
): { k: number; hasAlias: boolean } {
  if (source.charCodeAt(k) !== AS_A || source.charCodeAt(k + 1) !== AS_S) {
    return { k, hasAlias: false }
  }

  const afterAs = k + 2
  if (afterAs >= len || isIdentifierPartCode(source.charCodeAt(afterAs))) {
    return { k, hasAlias: true }
  }

  k = skipTrivia(source, afterAs, len)
  const aliasStart = k
  while (k < len && isIdentifierPartCode(source.charCodeAt(k))) k++
  if (k - aliasStart === 7 && isDefaultAt(source, aliasStart)) {
    markDefaultExportFlags(state)
  }
  return { k, hasAlias: true }
}

function advanceExportBraceSpecifier(
  source: string,
  k: number,
  len: number,
  state: ModuleScanState,
): number | null {
  const identStart = k
  while (k < len && isIdentifierPartCode(source.charCodeAt(k))) k++
  const identLen = k - identStart
  if (identLen === 0) return null

  k = skipTrivia(source, k, len)
  const alias = consumeExportAsAlias(source, k, len, state)
  k = alias.k

  if (!alias.hasAlias && identLen === 7 && isDefaultAt(source, identStart)) {
    markDefaultExportFlags(state)
  }

  if (source.charCodeAt(k) === CH_COMMA) return k + 1
  if (source.charCodeAt(k) === CH_CLOSE_BRACE) return k
  return k + 1
}

function scanExportBraceListForDefault(
  source: string,
  openBrace: number,
  len: number,
  state: ModuleScanState,
): void {
  let k = openBrace + 1
  while (k < len) {
    k = skipTrivia(source, k, len)
    if (source.charCodeAt(k) === CH_CLOSE_BRACE) break

    const next = advanceExportBraceSpecifier(source, k, len, state)
    if (next === null) break
    k = next
  }
}

function noteExportAt(
  source: string,
  i: number,
  len: number,
  state: ModuleScanState,
): number | null {
  if (!isExportAt(source, i)) return null
  const afterExport = i + 6
  if (!isExportKeywordBoundary(source, afterExport, len)) return null

  const j = skipTrivia(source, afterExport, len)

  if (isDefaultAt(source, j)) {
    markDefaultExportFlags(state)
    const afterDefault = j + 7
    if (afterDefault >= len || !isIdentifierPartCode(source.charCodeAt(afterDefault))) {
      return afterExport
    }
  }

  if (source.charCodeAt(j) === CH_OPEN_BRACE) {
    scanExportBraceListForDefault(source, j, len, state)
    return null
  }

  if (
    isKeywordAt(source, j, 'async') ||
    isKeywordAt(source, j, 'function') ||
    isKeywordAt(source, j, 'class')
  ) {
    state.hasComponentExportResult = true
  }

  return null
}

function advanceModuleScanAt(
  source: string,
  i: number,
  len: number,
  state: ModuleScanState,
): number {
  const directiveNext = tryConsumeDirectiveString(source, i, len, state)
  if (directiveNext !== null) return directiveNext

  const skipped = skipNonCodeToken(source, i, len)
  if (skipped !== -1) {
    if (state.directivesPhase && !isTriviaOrCommentStart(source, i)) state.directivesPhase = false
    return skipped
  }

  state.directivesPhase = false

  if (isKeywordAt(source, i, 'import')) {
    const collected = collectImportSourcesAt(source, i, len)
    for (const importSource of collected.sources) state.importSources.push(importSource)
    return collected.end
  }

  const exportAdvance = noteExportAt(source, i, len, state)
  if (exportAdvance !== null) return exportAdvance

  return i + 1
}

export function analyzeModuleSource(source: string): ModuleAnalysis {
  const state: ModuleScanState = {
    directives: { hasUseClient: false, hasUseServer: false },
    topLevelUseClient: false,
    topLevelUseServer: false,
    hasDefaultExportResult: false,
    hasComponentExportResult: false,
    importSources: [],
    directivesPhase: true,
    sawFirstDirective: false,
  }

  let i = 0
  const len = source.length

  while (i < len) {
    i = advanceModuleScanAt(source, i, len, state)
  }

  return {
    directives: state.directives,
    topLevelUseClient: state.topLevelUseClient,
    topLevelUseServer: state.topLevelUseServer,
    hasDefaultExport: state.hasDefaultExportResult,
    hasComponentExport: state.hasComponentExportResult,
    importSources: [...new Set(state.importSources)],
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

function tryParseTypeOnlySpecifierModifier(
  source: string,
  pos: number,
  len: number,
): number | null {
  if (!isKeywordAt(source, pos, 'type')) return null
  const after = skipTrivia(source, pos + 4, len)
  const afterCh = source.charCodeAt(after)
  const startsSpecifier =
    afterCh === CH_SINGLE_QUOTE ||
    afterCh === CH_DOUBLE_QUOTE ||
    readIdentifier(source, after, len) !== null
  if (startsSpecifier && !isKeywordAt(source, after, 'as')) return after
  return null
}

function readImportedName(
  source: string,
  pos: number,
  len: number,
): { imported: string; end: number } | null {
  const importedCh = source.charCodeAt(pos)
  if (importedCh === CH_SINGLE_QUOTE || importedCh === CH_DOUBLE_QUOTE) {
    const strEnd = skipString(source, pos, len, importedCh)
    return { imported: source.slice(pos + 1, strEnd - 1), end: strEnd }
  }
  const ident = readIdentifier(source, pos, len)
  if (!ident) return null
  return { imported: ident.name, end: ident.end }
}

function parseOneNamedImportSpecifier(
  source: string,
  pos: number,
  len: number,
): { specifier: ScannedImportSpecifier; end: number } | null {
  const typeOnlyPos = tryParseTypeOnlySpecifierModifier(source, pos, len)
  const specTypeOnly = typeOnlyPos !== null
  if (typeOnlyPos !== null) pos = typeOnlyPos

  const importedName = readImportedName(source, pos, len)
  if (!importedName) return null
  const imported = importedName.imported
  pos = skipTrivia(source, importedName.end, len)

  let local = imported
  if (isKeywordAt(source, pos, 'as')) {
    pos = skipTrivia(source, pos + 2, len)
    const alias = readIdentifier(source, pos, len)
    if (!alias) return null
    local = alias.name
    pos = skipTrivia(source, alias.end, len)
  }

  return { specifier: { imported, local, typeOnly: specTypeOnly }, end: pos }
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

    const parsed = parseOneNamedImportSpecifier(source, pos, len)
    if (!parsed) return null
    named.push(parsed.specifier)
    pos = parsed.end

    const ch = source.charCodeAt(pos)
    if (ch === CH_COMMA) {
      pos++
      continue
    }
    if (ch === CH_CLOSE_BRACE) return { named, end: pos + 1 }

    return null
  }
}

function parseSideEffectImport(
  source: string,
  start: number,
  pos: number,
  len: number,
): ScannedImport | null {
  const ch = source.charCodeAt(pos)
  if (ch !== CH_SINGLE_QUOTE && ch !== CH_DOUBLE_QUOTE) return null
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

function maybeAdvancePastImportTypeKeyword(
  source: string,
  pos: number,
  len: number,
): {
  typeOnly: boolean
  pos: number
} {
  if (!isKeywordAt(source, pos, 'type')) return { typeOnly: false, pos }
  const after = skipTrivia(source, pos + 4, len)
  if (!isKeywordAt(source, after, 'from') && source.charCodeAt(after) !== CH_COMMA) {
    return { typeOnly: true, pos: after }
  }
  return { typeOnly: false, pos }
}

function parseImportDefaultAndRestClause(
  source: string,
  pos: number,
  len: number,
): {
  defaultBinding: string | null
  namespaceBinding: string | null
  named: ScannedImportSpecifier[]
  pos: number
} | null {
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

  return { defaultBinding, namespaceBinding, named, pos }
}

function parseImportStatementAt(source: string, start: number, len: number): ScannedImport | null {
  let pos = skipTrivia(source, start + 6, len)
  if (pos >= len) return null

  const ch = source.charCodeAt(pos)
  if (ch === CH_OPEN_PAREN || ch === CH_DOT) return null

  const sideEffect = parseSideEffectImport(source, start, pos, len)
  if (sideEffect) return sideEffect

  const typeKw = maybeAdvancePastImportTypeKeyword(source, pos, len)
  pos = typeKw.pos

  const clause = parseImportDefaultAndRestClause(source, pos, len)
  if (!clause) return null
  pos = skipTrivia(source, clause.pos, len)
  if (!isKeywordAt(source, pos, 'from')) return null

  const spec = readImportModuleSpecifier(source, pos + 4, len)
  if (!spec) return null

  return {
    start,
    end: consumeTrailingSemicolon(source, spec.end, len),
    source: spec.source,
    typeOnly: typeKw.typeOnly,
    sideEffectOnly: false,
    defaultBinding: clause.defaultBinding,
    namespaceBinding: clause.namespaceBinding,
    named: clause.named,
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

function addExportStarAsName(
  source: string,
  pos: number,
  len: number,
  exports: Set<string>,
): number {
  let afterStar = skipTrivia(source, pos + 1, len)
  if (isKeywordAt(source, afterStar, 'as')) {
    afterStar = skipTrivia(source, afterStar + 2, len)
    const ns = readIdentifier(source, afterStar, len)
    if (ns) exports.add(ns.name)
    return ns?.end ?? afterStar + 1
  }
  return pos + 1
}

function addExportBraceListNames(
  source: string,
  pos: number,
  len: number,
  exports: Set<string>,
): number {
  const close = skipBalancedBraceList(source, pos, len)
  const list = source.slice(pos + 1, close - 1)
  for (const part of list.split(',')) {
    const trimmed = part.trim()
    if (!trimmed || trimmed.startsWith('type ') || trimmed.startsWith('typeof ')) continue
    const asParts = trimmed.split(/\s+as\s+/)
    const exportedName = (asParts.at(-1) ?? '').trim()
    if (exportedName !== '' && exportedName !== 'type') exports.add(exportedName)
  }
  return close
}

function declarationKeywordLen(source: string, pos: number): number {
  if (isKeywordAt(source, pos, 'function')) return 8
  if (isKeywordAt(source, pos, 'class')) return 5
  if (isKeywordAt(source, pos, 'const')) return 5
  return 3
}

function addExportDeclarationName(
  source: string,
  pos: number,
  len: number,
  exports: Set<string>,
): number | null {
  if (isKeywordAt(source, pos, 'async')) {
    pos = skipTrivia(source, pos + 5, len)
  }

  if (
    !isKeywordAt(source, pos, 'function') &&
    !isKeywordAt(source, pos, 'class') &&
    !isKeywordAt(source, pos, 'const') &&
    !isKeywordAt(source, pos, 'let') &&
    !isKeywordAt(source, pos, 'var')
  ) {
    return null
  }

  const keywordLen = declarationKeywordLen(source, pos)
  let after = skipTrivia(source, pos + keywordLen, len)
  if (source.charCodeAt(after) === CH_STAR) after = skipTrivia(source, after + 1, len)
  const name = readIdentifier(source, after, len)
  if (name) exports.add(name.name)
  return name?.end ?? after + 1
}

function consumeOneExportAt(source: string, i: number, len: number, exports: Set<string>): number {
  const pos = skipTrivia(source, i + 6, len)

  if (isKeywordAt(source, pos, 'type') || isKeywordAt(source, pos, 'interface')) {
    return pos + 1
  }

  if (source.charCodeAt(pos) === CH_STAR) {
    return addExportStarAsName(source, pos, len, exports)
  }

  if (isKeywordAt(source, pos, 'default')) {
    exports.add('default')
    return pos + 7
  }

  if (source.charCodeAt(pos) === CH_OPEN_BRACE) {
    return addExportBraceListNames(source, pos, len, exports)
  }

  const declEnd = addExportDeclarationName(source, pos, len, exports)
  if (declEnd !== null) return declEnd

  return i + 1
}

export function collectExportNames(source: string): string[] {
  const exports = new Set<string>()
  let i = 0
  const len = source.length

  while (i < len) {
    const skipped = skipNonCodeToken(source, i, len)
    if (skipped !== -1) {
      i = skipped
      continue
    }

    if (!isKeywordAt(source, i, 'export')) {
      i++
      continue
    }

    i = consumeOneExportAt(source, i, len, exports)
  }

  return [...exports]
}

function skipBalancedBraceList(source: string, start: number, len: number): number {
  let i = start
  let depth = 0
  while (i < len) {
    const skipped = skipStringOrCommentAt(source, i, len)
    if (skipped != null) {
      i = skipped
      continue
    }
    const ch = source.charCodeAt(i)
    if (ch === CH_OPEN_BRACE) {
      depth++
      i++
      continue
    }
    if (ch === CH_CLOSE_BRACE) {
      depth--
      i++
      if (depth === 0) return i
      continue
    }
    i++
  }
  return i
}

function skipStringOrCommentAt(source: string, i: number, len: number): number | null {
  const ch = source.charCodeAt(i)
  if (ch === CH_SINGLE_QUOTE || ch === CH_DOUBLE_QUOTE || ch === CH_BACKTICK) {
    return skipString(source, i, len, ch)
  }
  if (ch === CH_SLASH && source.charCodeAt(i + 1) === CH_SLASH) {
    return skipSingleLineComment(source, i, len)
  }
  if (ch === CH_SLASH && source.charCodeAt(i + 1) === CH_STAR) {
    return skipMultiLineComment(source, i, len)
  }
  return null
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

function skipBackwardBlockComment(source: string, i: number): number {
  i -= 2
  while (i >= 1) {
    if (source.charCodeAt(i) === CH_STAR && source.charCodeAt(i - 1) === CH_SLASH) {
      return i - 2
    }
    i--
  }
  return -1
}

function skipBackwardLineCommentIfPresent(source: string, i: number): number | null {
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
    return afterNewline - 1
  }
  return null
}

function skipBackwardTriviaAt(source: string, i: number): number | null {
  const ch = source.charCodeAt(i)
  if (isWhitespaceCode(ch)) return i - 1

  if (i >= 1 && ch === CH_SLASH && source.charCodeAt(i - 1) === CH_STAR) {
    return skipBackwardBlockComment(source, i)
  }

  if (i >= 1 && ch === CH_SLASH && source.charCodeAt(i - 1) === CH_SLASH) {
    return i - 2
  }

  return skipBackwardLineCommentIfPresent(source, i)
}

function getPreviousToken(source: string, pos: number): string | undefined {
  let i = pos - 1

  while (i >= 0) {
    const skipped = skipBackwardTriviaAt(source, i)
    if (skipped !== null) {
      i = skipped
      if (i < 0) return undefined
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
    const skipped = skipBackwardTriviaAt(source, i)
    if (skipped !== null) {
      i = skipped
      if (i < 0) return -1
      continue
    }
    return source.charCodeAt(i)
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

const JSX_PRECEDE_KEYWORDS = new Set([
  'return',
  'throw',
  'case',
  'default',
  'else',
  'do',
  'typeof',
  'void',
  'yield',
  'await',
  'delete',
])

function canPrecedeJSX(source: string, pos: number): boolean {
  const prevCharCode = getPreviousNonTriviaCharCode(source, pos)
  if (prevCharCode === -1) return true

  if (isIdentifierPartCode(prevCharCode) || (prevCharCode >= CH_0 && prevCharCode <= CH_9)) {
    const prevToken = getPreviousToken(source, pos)
    return prevToken != null && prevToken !== '' && JSX_PRECEDE_KEYWORDS.has(prevToken)
  }

  if (
    prevCharCode === CH_CLOSE_PAREN ||
    prevCharCode === CH_CLOSE_BRACKET ||
    prevCharCode === CH_CLOSE_BRACE
  ) {
    return false
  }

  return true
}

/**
 * Try to skip a JSX tag/element at `i`. Returns the offset past a completed
 * tag (ending in `>`), or -1 when `<` is not JSX (comparison, generics, etc.).
 */
function trySkipJSX(source: string, i: number, len: number): number {
  if (!canPrecedeJSX(source, i)) return -1

  const nextCh = source.charCodeAt(i + 1)
  if (
    nextCh !== CH_SLASH &&
    nextCh !== CH_DOT &&
    nextCh !== CH_GT &&
    !isIdentifierStartCode(nextCh)
  ) {
    return -1
  }

  let end = skipJSX(source, i, len)
  // skipJSX may stop after a closing tag name before consuming `>`.
  if (end < len && source.charCodeAt(end) === CH_GT) end++
  if (end > i && source.charCodeAt(end - 1) === CH_GT) return end
  return -1
}

function advanceRegexChar(
  source: string,
  i: number,
  len: number,
  inCharClass: boolean,
): { i: number; inCharClass: boolean; done: boolean } {
  const ch = source.charCodeAt(i)
  if (ch === CH_BACKSLASH) return { i: i + 2, inCharClass, done: false }

  if (inCharClass) {
    return { i: i + 1, inCharClass: ch !== CH_CLOSE_BRACKET, done: false }
  }

  if (ch === CH_OPEN_BRACKET) return { i: i + 1, inCharClass: true, done: false }

  if (ch === CH_SLASH) {
    i++
    while (i < len && isIdentifierPartCode(source.charCodeAt(i))) i++
    return { i, inCharClass, done: true }
  }

  if (isLineTerminatorCode(ch)) return { i, inCharClass, done: true }

  return { i: i + 1, inCharClass, done: false }
}

function skipRegex(source: string, i: number, len: number): number {
  i++
  let inCharClass = false

  while (i < len) {
    const step = advanceRegexChar(source, i, len, inCharClass)
    i = step.i
    inCharClass = step.inCharClass
    if (step.done) return i
  }

  return i
}

/**
 * If `source[i]` starts whitespace, a comment, string, regex literal, or JSX,
 * return the offset past that token. Returns -1 when the position is code.
 */
export function skipNonCodeToken(source: string, i: number, len: number): number {
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
    const jsxEnd = trySkipJSX(source, i, len)
    if (jsxEnd !== -1) return jsxEnd
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
function readExportDefaultBindingName(
  source: string,
  valueStart: number,
  len: number,
): string | null {
  let pos = valueStart

  if (isKeywordAt(source, pos, 'async')) {
    const afterAsync = skipTrivia(source, pos + 5, len)
    if (isKeywordAt(source, afterAsync, 'function')) pos = afterAsync
  }

  if (!isKeywordAt(source, pos, 'function') && !isKeywordAt(source, pos, 'class')) {
    return null
  }

  const keywordLen = isKeywordAt(source, pos, 'function') ? 8 : 5
  let afterKeyword = skipTrivia(source, pos + keywordLen, len)
  if (source.charCodeAt(afterKeyword) === CH_STAR) {
    afterKeyword = skipTrivia(source, afterKeyword + 1, len)
  }
  const name = readIdentifier(source, afterKeyword, len)
  return name ? name.name : null
}

export function locateExportDefaultValue(source: string): ExportDefaultValueLocation | null {
  const len = source.length
  let i = 0

  while (i < len) {
    const skipped = skipNonCodeToken(source, i, len)
    if (skipped !== -1) {
      i = skipped
      continue
    }

    if (!isKeywordAt(source, i, 'export')) {
      i++
      continue
    }

    const afterExport = skipTrivia(source, i + 6, len)
    if (!isKeywordAt(source, afterExport, 'default')) {
      i++
      continue
    }

    const valueStart = skipTrivia(source, afterExport + 7, len)
    const bindingName = readExportDefaultBindingName(source, valueStart, len)
    const valueEnd = scanExportDefaultValueEnd(source, valueStart, len)

    return {
      exportStart: i,
      valueStart,
      valueEnd,
      statementEnd: consumeTrailingSemicolon(source, valueEnd, len),
      bindingName,
    }
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

interface ExprScanState {
  i: number
  paren: number
  brace: number
  bracket: number
  lastCanTerminate: boolean
}

function trySkipStringCommentOrRegex(source: string, state: ExprScanState, len: number): boolean {
  const ch = source.charCodeAt(state.i)
  if (ch === CH_SINGLE_QUOTE || ch === CH_DOUBLE_QUOTE || ch === CH_BACKTICK) {
    state.i = skipString(source, state.i, len, ch)
    state.lastCanTerminate = true
    return true
  }
  if (ch === CH_SLASH && source.charCodeAt(state.i + 1) === CH_SLASH) {
    state.i = skipSingleLineComment(source, state.i, len)
    return true
  }
  if (ch === CH_SLASH && source.charCodeAt(state.i + 1) === CH_STAR) {
    state.i = skipMultiLineComment(source, state.i, len)
    return true
  }
  if (
    ch === CH_SLASH &&
    source.charCodeAt(state.i + 1) !== CH_SLASH &&
    source.charCodeAt(state.i + 1) !== CH_STAR &&
    canPrecedeRegexWithKeywords(source, state.i)
  ) {
    state.i = skipRegex(source, state.i, len)
    state.lastCanTerminate = true
    return true
  }
  return false
}

function trySkipJsxInExpr(source: string, state: ExprScanState, len: number): boolean {
  if (source.charCodeAt(state.i) !== CH_LT) return false
  if (state.paren !== 0 || state.brace !== 0 || state.bracket !== 0) return false
  const jsxEnd = trySkipJSX(source, state.i, len)
  if (jsxEnd === -1) return false
  state.i = jsxEnd
  state.lastCanTerminate = true
  return true
}

function applyGroupingChar(ch: number, state: ExprScanState): boolean | null {
  if (ch === CH_OPEN_PAREN) {
    state.paren++
    state.lastCanTerminate = false
    state.i++
    return false
  }
  if (ch === CH_CLOSE_PAREN) {
    state.paren = Math.max(0, state.paren - 1)
    state.lastCanTerminate = true
    state.i++
    return false
  }
  if (ch === CH_OPEN_BRACE) {
    state.brace++
    state.lastCanTerminate = false
    state.i++
    return false
  }
  if (ch === CH_CLOSE_BRACE) {
    state.brace = Math.max(0, state.brace - 1)
    state.lastCanTerminate = true
    state.i++
    return state.paren === 0 && state.brace === 0 && state.bracket === 0
  }
  if (ch === CH_OPEN_BRACKET) {
    state.bracket++
    state.lastCanTerminate = false
    state.i++
    return false
  }
  if (ch === CH_CLOSE_BRACKET) {
    state.bracket = Math.max(0, state.bracket - 1)
    state.lastCanTerminate = true
    state.i++
    return false
  }
  return null
}

function tryTerminateAtTopLevel(
  source: string,
  state: ExprScanState,
  len: number,
  ch: number,
): number | null {
  if (state.paren !== 0 || state.brace !== 0 || state.bracket !== 0) return null
  if (ch === CH_SEMICOLON) return state.i
  if (!isLineTerminatorCode(ch)) return null
  const afterNl = skipTrivia(source, state.i + 1, len)
  if (!state.lastCanTerminate || isExpressionContinuationAt(source, afterNl, len)) {
    state.i++
    return -1 // sentinel: consumed, keep scanning
  }
  return state.i
}

function skipNumericLiteral(source: string, i: number, len: number): number {
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
  return i
}

function isBinaryOrUnaryOpCode(ch: number): boolean {
  return (
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
  )
}

function advanceExprAtom(source: string, state: ExprScanState, len: number): void {
  const ch = source.charCodeAt(state.i)

  if (isIdentifierStartCode(ch)) {
    const id = readIdentifier(source, state.i, len)
    if (id) {
      state.lastCanTerminate = !NON_TERMINATING_EXPR_KEYWORDS.has(id.name)
      state.i = id.end
      return
    }
  }

  if (ch >= CH_0 && ch <= CH_9) {
    state.lastCanTerminate = true
    state.i = skipNumericLiteral(source, state.i, len)
    return
  }

  if (ch === CH_EQUALS && source.charCodeAt(state.i + 1) === CH_GT) {
    state.lastCanTerminate = false
    state.i += 2
    return
  }

  if (isBinaryOrUnaryOpCode(ch)) {
    state.lastCanTerminate = false
    state.i++
    return
  }

  state.i++
}

function scanExportDefaultValueEnd(source: string, start: number, len: number): number {
  const state: ExprScanState = {
    i: start,
    paren: 0,
    brace: 0,
    bracket: 0,
    lastCanTerminate: false,
  }

  while (state.i < len) {
    if (trySkipStringCommentOrRegex(source, state, len)) continue
    if (trySkipJsxInExpr(source, state, len)) continue

    const ch = source.charCodeAt(state.i)
    const groupingDone = applyGroupingChar(ch, state)
    if (groupingDone === true) return state.i
    if (groupingDone === false) continue

    const terminated = tryTerminateAtTopLevel(source, state, len, ch)
    if (terminated === -1) continue
    if (terminated !== null) return terminated

    advanceExprAtom(source, state, len)
  }

  return state.i
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
