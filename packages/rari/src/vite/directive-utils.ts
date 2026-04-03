function isWhitespace(char: string): boolean {
  return char === ' ' || char === '\t' || char === '\r' || char === '\n' || char === '\u2028' || char === '\u2029' || char === '\uFEFF'
}

function isLineTerminator(char: string): boolean {
  return char === '\r' || char === '\n' || char === '\u2028' || char === '\u2029'
}

function skipWhitespace(source: string, i: number, len: number): number {
  while (i < len && isWhitespace(source[i])) {
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
    if (source[i] === '/' && source[i + 1] === '/') {
      i = skipSingleLineComment(source, i, len)
      continue
    }
    if (source[i] === '/' && source[i + 1] === '*') {
      i = skipMultiLineComment(source, i, len)
      continue
    }
    break
  }

  return i
}

function skipSingleLineComment(source: string, i: number, len: number): number {
  while (i < len && !isLineTerminator(source[i])) {
    i++
  }

  return i
}

function skipMultiLineComment(source: string, i: number, len: number): number {
  i += 2
  while (i < len - 1 && (source[i] !== '*' || source[i + 1] !== '/')) {
    i++
  }

  return i + 2
}

function skipString(source: string, i: number, len: number, quote: string): number {
  i++
  while (i < len) {
    if (source[i] === '\\') {
      i += 2
      continue
    }
    if (source[i] === quote) {
      return i + 1
    }
    i++
  }

  return i
}

function hasDirective(source: string, targetDirective: string): boolean {
  let i = 0
  const len = source.length

  while (i < len) {
    if (isWhitespace(source[i])) {
      i++
      continue
    }

    if (source[i] === '/' && source[i + 1] === '/') {
      i = skipSingleLineComment(source, i, len)
      continue
    }

    if (source[i] === '/' && source[i + 1] === '*') {
      i = skipMultiLineComment(source, i, len)
      continue
    }

    const quote = source[i] === '\'' || source[i] === '"' ? source[i] : null
    if (quote) {
      const start = i + 1
      const end = skipString(source, i, len, quote)
      if (end > start) {
        const directive = source.slice(start, end - 1)
        let j = end

        while (j < len) {
          if (isWhitespace(source[j]) && !isLineTerminator(source[j])) {
            j++
            continue
          }
          if (isLineTerminator(source[j]) || source[j] === ';') {
            if (directive === targetDirective) {
              return true
            }
            i = j + 1
            break
          }
          if (source[j] === '/' && source[j + 1] === '/') {
            j = skipSingleLineComment(source, j, len)
            continue
          }
          if (source[j] === '/' && source[j + 1] === '*') {
            j = skipMultiLineComment(source, j, len)
            continue
          }

          return false
        }

        if (j >= len) {
          return directive === targetDirective
        }

        if (j < len && (isLineTerminator(source[j - 1]) || source[j - 1] === ';')) {
          continue
        }

        return false
      }
    }

    return false
  }

  return false
}

export function hasTopLevelUseServerDirective(source: string): boolean {
  return hasDirective(source, 'use server')
}

export function hasTopLevelUseClientDirective(source: string): boolean {
  return hasDirective(source, 'use client')
}

function isIdentifierPart(char: string | undefined): boolean {
  return !!char && (
    (char >= 'a' && char <= 'z')
    || (char >= 'A' && char <= 'Z')
    || (char >= '0' && char <= '9')
    || char === '_'
    || char === '$'
  )
}

function canPrecedeRegex(char: string | undefined): boolean {
  return !char || char === '(' || char === '[' || char === '{' || char === ','
    || char === ';' || char === '=' || char === ':' || char === '?' || char === '!'
    || char === '+' || char === '-' || char === '*' || char === '%' || char === '&'
    || char === '|' || char === '^' || char === '~' || char === '<' || char === '>'
}

function getPreviousToken(source: string, pos: number): string | undefined {
  let i = pos - 1

  while (i >= 0) {
    if (isWhitespace(source[i])) {
      i--
      continue
    }

    if (i >= 1 && source[i] === '/' && source[i - 1] === '*') {
      i -= 2
      while (i >= 1) {
        if (source[i] === '*' && source[i - 1] === '/') {
          i -= 2
          break
        }
        i--
      }
      if (i < 0) {
        return undefined
      }
      continue
    }

    if (i >= 1 && source[i] === '/' && source[i - 1] === '/') {
      i -= 2
      while (i >= 0 && source[i] !== '\n') {
        i--
      }
      if (i < 0) {
        return undefined
      }
      continue
    }

    break
  }

  if (i < 0) {
    return undefined
  }

  if (!isIdentifierPart(source[i])) {
    return undefined
  }

  const end = i
  while (i >= 0 && isIdentifierPart(source[i])) {
    i--
  }

  return source.slice(i + 1, end + 1)
}

function canPrecedeRegexWithKeywords(source: string, pos: number): boolean {
  const prevChar = getPreviousNonTriviaChar(source, pos)

  if (canPrecedeRegex(prevChar)) {
    return true
  }

  const prevToken = getPreviousToken(source, pos)
  if (prevToken) {
    const regexKeywords = new Set([
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
    return regexKeywords.has(prevToken)
  }

  return false
}

function skipRegex(source: string, i: number, len: number): number {
  i++
  let inCharClass = false

  while (i < len) {
    if (source[i] === '\\') {
      i += 2
      continue
    }

    if (inCharClass) {
      if (source[i] === ']') {
        inCharClass = false
      }
      i++
      continue
    }

    if (source[i] === '[') {
      inCharClass = true
      i++
      continue
    }

    if (source[i] === '/') {
      i++
      while (i < len && isIdentifierPart(source[i])) {
        i++
      }

      return i
    }

    if (isLineTerminator(source[i])) {
      return i
    }

    i++
  }

  return i
}

function getPreviousNonTriviaChar(source: string, pos: number): string | undefined {
  let i = pos - 1
  while (i >= 0) {
    if (isWhitespace(source[i])) {
      i--
      continue
    }

    if (i >= 1 && source[i] === '/' && source[i - 1] === '*') {
      i -= 2
      while (i >= 1) {
        if (source[i] === '*' && source[i - 1] === '/') {
          i -= 2
          break
        }
        i--
      }
      if (i < 0) {
        return undefined
      }
      continue
    }

    if (i >= 1 && source[i] === '/' && source[i - 1] === '/') {
      i -= 2
      while (i >= 0 && source[i] !== '\n') {
        i--
      }
      if (i < 0) {
        return undefined
      }
      continue
    }

    return source[i]
  }

  return undefined
}

export function hasDefaultExport(source: string): boolean {
  let i = 0
  const len = source.length

  while (i < len) {
    if (isWhitespace(source[i])) {
      i++
      continue
    }

    if (source[i] === '/' && source[i + 1] === '/') {
      i = skipSingleLineComment(source, i, len)
      continue
    }

    if (source[i] === '/' && source[i + 1] === '*') {
      i = skipMultiLineComment(source, i, len)
      continue
    }

    const quote = source[i] === '\'' || source[i] === '"' || source[i] === '`' ? source[i] : null
    if (quote) {
      i = skipString(source, i, len, quote)
      continue
    }

    if (source[i] === '/' && source[i + 1] !== '/' && source[i + 1] !== '*') {
      if (canPrecedeRegexWithKeywords(source, i)) {
        i = skipRegex(source, i, len)
        continue
      }
    }

    if (source.slice(i, i + 6) === 'export') {
      const afterExport = i + 6
      if (
        afterExport < len
        && (
          isWhitespace(source[afterExport])
          || (source[afterExport] === '/' && (source[afterExport + 1] === '/' || source[afterExport + 1] === '*'))
        )
      ) {
        const j = skipTrivia(source, afterExport, len)
        if (source.slice(j, j + 7) === 'default') {
          const afterDefault = j + 7
          if (afterDefault >= len || !isIdentifierPart(source[afterDefault])) {
            return true
          }
        }
      }
    }

    i++
  }

  return false
}
