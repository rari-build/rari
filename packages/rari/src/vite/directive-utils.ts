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

function skipJSX(source: string, i: number, len: number): number {
  i++

  const isClosingTag = source[i] === '/'
  if (isClosingTag) {
    i++
  }

  while (i < len && (isIdentifierPart(source[i]) || source[i] === '.' || source[i] === '-')) {
    i++
  }

  let depth = isClosingTag ? 0 : 1

  while (i < len && depth > 0) {
    const quote = source[i] === '\'' || source[i] === '"' || source[i] === '`' ? source[i] : null
    if (quote) {
      i = skipString(source, i, len, quote)
      continue
    }

    if (source[i] === '{') {
      i++
      let braceDepth = 1
      while (i < len && braceDepth > 0) {
        const quote = source[i] === '\'' || source[i] === '"' || source[i] === '`' ? source[i] : null
        if (quote) {
          i = skipString(source, i, len, quote)
          continue
        }
        if (source[i] === '{')
          braceDepth++
        if (source[i] === '}')
          braceDepth--
        i++
      }
      continue
    }

    if (source[i] === '/' && source[i + 1] === '>') {
      depth--
      i += 2
      continue
    }

    if (source[i] === '>') {
      i++
      if (isClosingTag) {
        depth--
      }
      continue
    }

    if (source[i] === '<') {
      const nextChar = source[i + 1]
      if (nextChar === '/' || nextChar === '.' || nextChar === '>' || isIdentifierStart(nextChar)) {
        if (nextChar === '/') {
          depth--
          i++
        }
        else if (nextChar !== '!') {
          depth++
        }
        i++
        while (i < len && (isIdentifierPart(source[i]) || source[i] === '.' || source[i] === '-')) {
          i++
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

        if (j < len && (isLineTerminator(source[j]) || source[j] === ';')) {
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

function isIdentifierStart(char: string | undefined): boolean {
  return !!char && (
    (char >= 'a' && char <= 'z')
    || (char >= 'A' && char <= 'Z')
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
      continue
    }

    let checkPos = i
    while (checkPos >= 0 && source[checkPos] !== '\n' && source[checkPos] !== '\r') {
      checkPos--
    }
    let afterNewline = checkPos + 1
    while (afterNewline < i && (source[afterNewline] === ' ' || source[afterNewline] === '\t')) {
      afterNewline++
    }
    if (afterNewline < i && source[afterNewline] === '/' && source[afterNewline + 1] === '/') {
      i = afterNewline - 1
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
    return REGEX_KEYWORDS.has(prevToken)
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
      continue
    }

    let checkPos = i
    while (checkPos >= 0 && source[checkPos] !== '\n' && source[checkPos] !== '\r') {
      checkPos--
    }
    let afterNewline = checkPos + 1
    while (afterNewline < i && (source[afterNewline] === ' ' || source[afterNewline] === '\t')) {
      afterNewline++
    }
    if (afterNewline < i && source[afterNewline] === '/' && source[afterNewline + 1] === '/') {
      i = afterNewline - 1
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

    if (source[i] === '<') {
      const nextChar = source[i + 1]
      if (nextChar === '/' || nextChar === '.' || nextChar === '>' || isIdentifierStart(nextChar)) {
        i = skipJSX(source, i, len)
        continue
      }
      i++
      continue
    }

    if (source.slice(i, i + 6) === 'export') {
      const afterExport = i + 6
      if (
        afterExport < len
        && (
          isWhitespace(source[afterExport])
          || source[afterExport] === '{'
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

        if (source[j] === '{') {
          let k = j + 1
          while (k < len) {
            k = skipTrivia(source, k, len)

            if (source[k] === '}') {
              break
            }

            const identStart = k
            while (k < len && isIdentifierPart(source[k])) {
              k++
            }
            const ident = source.slice(identStart, k)

            if (!ident) {
              break
            }

            k = skipTrivia(source, k, len)

            let hasAlias = false
            if (source.slice(k, k + 2) === 'as') {
              hasAlias = true
              const afterAs = k + 2
              if (afterAs < len && !isIdentifierPart(source[afterAs])) {
                k = skipTrivia(source, afterAs, len)
                const aliasStart = k
                while (k < len && isIdentifierPart(source[k])) {
                  k++
                }
                const alias = source.slice(aliasStart, k)
                if (alias === 'default')
                  return true
              }
            }

            if (!hasAlias && ident === 'default') {
              return true
            }

            if (source[k] === ',') {
              k++
              continue
            }

            if (source[k] === '}') {
              break
            }

            k++
          }
        }
      }
    }

    i++
  }

  return false
}
