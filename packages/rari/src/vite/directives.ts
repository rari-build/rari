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
const CH_BOM = 0xFEFF
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
  return ch === CH_SPACE || ch === CH_TAB || ch === CH_CR || ch === CH_LF || ch === CH_LINE_SEP || ch === CH_PARA_SEP || ch === CH_BOM
}

function isLineTerminatorCode(ch: number): boolean {
  return ch === CH_CR || ch === CH_LF || ch === CH_LINE_SEP || ch === CH_PARA_SEP
}

function isIdentifierPartCode(ch: number): boolean {
  return (ch >= CH_LOWER_A && ch <= CH_LOWER_Z)
    || (ch >= CH_UPPER_A && ch <= CH_UPPER_Z)
    || (ch >= CH_0 && ch <= CH_9)
    || ch === CH_UNDERSCORE
    || ch === CH_DOLLAR
}

function isIdentifierStartCode(ch: number): boolean {
  return (ch >= CH_LOWER_A && ch <= CH_LOWER_Z)
    || (ch >= CH_UPPER_A && ch <= CH_UPPER_Z)
    || ch === CH_UNDERSCORE
    || ch === CH_DOLLAR
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
  while (i < len - 1 && (source.charCodeAt(i) !== CH_STAR || source.charCodeAt(i + 1) !== CH_SLASH)) {
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
    }
    else {
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
        if (bch === CH_OPEN_BRACE)
          braceDepth++
        if (bch === CH_CLOSE_BRACE)
          braceDepth--
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
      if (nextCh === CH_SLASH || nextCh === CH_DOT || nextCh === CH_GT || isIdentifierStartCode(nextCh)) {
        if (nextCh === CH_SLASH) {
          depth--
          i++
        }
        else if (nextCh !== CH_EXCL) {
          depth++
        }
        i++
        while (i < len) {
          const tch = source.charCodeAt(i)
          if (isIdentifierPartCode(tch) || tch === CH_DOT || tch === CH_MINUS) {
            i++
          }
          else {
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
    if (source.charCodeAt(offset + k) !== target.charCodeAt(k))
      return false
  }

  return true
}

export interface DirectiveResult {
  hasUseClient: boolean
  hasUseServer: boolean
}

function isKeywordAt(source: string, i: number, keyword: string): boolean {
  if (i + keyword.length > source.length)
    return false

  for (let k = 0; k < keyword.length; k++) {
    if (source.charCodeAt(i + k) !== keyword.charCodeAt(k))
      return false
  }

  const before = i > 0 ? source.charCodeAt(i - 1) : -1
  const after = source.charCodeAt(i + keyword.length)
  if (before !== -1 && isIdentifierPartCode(before))
    return false
  if (after !== undefined && isIdentifierPartCode(after))
    return false

  return true
}

function readImportModuleSpecifier(source: string, i: number, len: number): { source: string, end: number } | null {
  const pos = skipTrivia(source, i, len)
  if (pos >= len)
    return null

  const ch = source.charCodeAt(pos)
  if (ch !== CH_SINGLE_QUOTE && ch !== CH_DOUBLE_QUOTE)
    return null

  const strStart = pos + 1
  const strEnd = skipString(source, pos, len, ch)
  if (strEnd <= strStart)
    return null

  return {
    source: source.slice(strStart, strEnd - 1),
    end: strEnd,
  }
}

function collectImportSourcesAt(source: string, i: number, len: number): { sources: string[], end: number } {
  if (!isKeywordAt(source, i, 'import'))
    return { sources: [], end: i + 6 }

  let pos = i + 6
  pos = skipTrivia(source, pos, len)

  if (isKeywordAt(source, pos, 'type'))
    pos = skipTrivia(source, pos + 4, len)

  if (source.charCodeAt(pos) === CH_DOT) {
    pos++
    while (pos < len && isIdentifierPartCode(source.charCodeAt(pos)))
      pos++

    while (pos < len && source.charCodeAt(pos) === CH_DOT) {
      pos++
      while (pos < len && isIdentifierPartCode(source.charCodeAt(pos)))
        pos++
    }

    return { sources: [], end: pos }
  }

  if (source.charCodeAt(pos) === CH_OPEN_PAREN) {
    pos++
    const specifier = readImportModuleSpecifier(source, pos, len)
    if (specifier)
      return { sources: [specifier.source], end: specifier.end }

    return { sources: [], end: pos }
  }

  const sideEffect = readImportModuleSpecifier(source, pos, len)
  if (sideEffect)
    return { sources: [sideEffect.source], end: sideEffect.end }

  let depth = 0
  while (pos < len) {
    pos = skipTrivia(source, pos, len)
    if (pos >= len)
      break

    const ch = source.charCodeAt(pos)

    if (depth === 0 && isKeywordAt(source, pos, 'from')) {
      const specifier = readImportModuleSpecifier(source, pos + 4, len)
      if (specifier)
        return { sources: [specifier.source], end: specifier.end }

      break
    }

    if (ch === CH_SINGLE_QUOTE || ch === CH_DOUBLE_QUOTE || ch === CH_BACKTICK) {
      pos = skipString(source, pos, len, ch)
      continue
    }

    if (ch === CH_OPEN_BRACE || ch === CH_OPEN_PAREN || ch === CH_OPEN_BRACKET)
      depth++
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
  return source.charCodeAt(i) === EXPORT_E
    && source.charCodeAt(i + 1) === EXPORT_X
    && source.charCodeAt(i + 2) === EXPORT_P
    && source.charCodeAt(i + 3) === EXPORT_O
    && source.charCodeAt(i + 4) === EXPORT_R
    && source.charCodeAt(i + 5) === EXPORT_T
}

function isDefaultAt(source: string, i: number): boolean {
  return source.charCodeAt(i) === DEFAULT_D
    && source.charCodeAt(i + 1) === DEFAULT_E2
    && source.charCodeAt(i + 2) === DEFAULT_F
    && source.charCodeAt(i + 3) === DEFAULT_A
    && source.charCodeAt(i + 4) === DEFAULT_U
    && source.charCodeAt(i + 5) === DEFAULT_L
    && source.charCodeAt(i + 6) === DEFAULT_T2
}

export interface ModuleAnalysis {
  directives: DirectiveResult
  topLevelUseClient: boolean
  topLevelUseServer: boolean
  hasDefaultExport: boolean
  hasComponentExport: boolean
  importSources: string[]
}

export function analyzeModuleSource(source: string): ModuleAnalysis {
  const directives: DirectiveResult = { hasUseClient: false, hasUseServer: false }
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

    if (isWhitespaceCode(ch)) {
      i++
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

      if (isUseClient)
        directives.hasUseClient = true
      if (isUseServer)
        directives.hasUseServer = true

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
        if (j >= len)
          directivesPhase = false
        i = directivesPhase ? stringEnd : j
        continue
      }

      continue
    }

    directivesPhase = false

    if (ch === CH_SINGLE_QUOTE || ch === CH_DOUBLE_QUOTE || ch === CH_BACKTICK) {
      i = skipString(source, i, len, ch)
      continue
    }

    if (ch === CH_SLASH && source.charCodeAt(i + 1) !== CH_SLASH && source.charCodeAt(i + 1) !== CH_STAR) {
      if (canPrecedeRegexWithKeywords(source, i)) {
        i = skipRegex(source, i, len)
        continue
      }
    }

    if (ch === CH_LT) {
      const nextCh = source.charCodeAt(i + 1)
      if (nextCh === CH_SLASH || nextCh === CH_DOT || nextCh === CH_GT || isIdentifierStartCode(nextCh)) {
        i = skipJSX(source, i, len)
        continue
      }
      i++
      continue
    }

    if (isKeywordAt(source, i, 'import')) {
      const collected = collectImportSourcesAt(source, i, len)
      for (const importSource of collected.sources)
        importSources.push(importSource)
      i = collected.end
      continue
    }

    if (isExportAt(source, i)) {
      const afterExport = i + 6
      if (afterExport < len) {
        const afterCh = source.charCodeAt(afterExport)
        if (
          isWhitespaceCode(afterCh)
          || afterCh === CH_OPEN_BRACE
          || (afterCh === CH_SLASH && (source.charCodeAt(afterExport + 1) === CH_SLASH || source.charCodeAt(afterExport + 1) === CH_STAR))
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

              if (source.charCodeAt(k) === CH_CLOSE_BRACE)
                break

              const identStart = k
              while (k < len && isIdentifierPartCode(source.charCodeAt(k)))
                k++
              const identLen = k - identStart

              if (identLen === 0)
                break

              k = skipTrivia(source, k, len)

              let hasAlias = false
              if (source.charCodeAt(k) === AS_A && source.charCodeAt(k + 1) === AS_S) {
                hasAlias = true
                const afterAs = k + 2
                if (afterAs < len && !isIdentifierPartCode(source.charCodeAt(afterAs))) {
                  k = skipTrivia(source, afterAs, len)
                  const aliasStart = k
                  while (k < len && isIdentifierPartCode(source.charCodeAt(k)))
                    k++
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

              if (source.charCodeAt(k) === CH_CLOSE_BRACE)
                break

              k++
            }
          }
          else if (
            isKeywordAt(source, j, 'async')
            || isKeywordAt(source, j, 'function')
            || isKeywordAt(source, j, 'class')
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
  return ch === CH_OPEN_PAREN || ch === CH_OPEN_BRACKET || ch === CH_OPEN_BRACE || ch === CH_COMMA
    || ch === CH_SEMICOLON || ch === CH_EQUALS || ch === CH_COLON || ch === CH_QUESTION || ch === CH_EXCL
    || ch === CH_PLUS || ch === CH_MINUS || ch === CH_STAR || ch === CH_PERCENT || ch === CH_AMP
    || ch === CH_PIPE || ch === CH_CARET || ch === CH_TILDE || ch === CH_LT || ch === CH_GT
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
      if (i < 0)
        return undefined
      continue
    }

    if (i >= 1 && ch === CH_SLASH && source.charCodeAt(i - 1) === CH_SLASH) {
      i -= 2
      continue
    }

    let checkPos = i
    while (checkPos >= 0 && source.charCodeAt(checkPos) !== CH_LF && source.charCodeAt(checkPos) !== CH_CR) {
      checkPos--
    }
    let afterNewline = checkPos + 1
    while (afterNewline < i && (source.charCodeAt(afterNewline) === CH_SPACE || source.charCodeAt(afterNewline) === CH_TAB)) {
      afterNewline++
    }
    if (afterNewline < i && source.charCodeAt(afterNewline) === CH_SLASH && source.charCodeAt(afterNewline + 1) === CH_SLASH) {
      i = afterNewline - 1
      continue
    }

    break
  }

  if (i < 0)
    return undefined
  if (!isIdentifierPartCode(source.charCodeAt(i)))
    return undefined

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
      if (i < 0)
        return -1
      continue
    }

    if (i >= 1 && ch === CH_SLASH && source.charCodeAt(i - 1) === CH_SLASH) {
      i -= 2
      continue
    }

    let checkPos = i
    while (checkPos >= 0 && source.charCodeAt(checkPos) !== CH_LF && source.charCodeAt(checkPos) !== CH_CR) {
      checkPos--
    }
    let afterNewline = checkPos + 1
    while (afterNewline < i && (source.charCodeAt(afterNewline) === CH_SPACE || source.charCodeAt(afterNewline) === CH_TAB)) {
      afterNewline++
    }
    if (afterNewline < i && source.charCodeAt(afterNewline) === CH_SLASH && source.charCodeAt(afterNewline + 1) === CH_SLASH) {
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
  if (prevToken) {
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
