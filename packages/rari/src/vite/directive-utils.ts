function isWhitespace(char: string): boolean {
  return char === ' ' || char === '\t' || char === '\r' || char === '\n' || char === '\uFEFF'
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
  while (i < len && source[i] !== '\n') {
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

export function getTopLevelDirective(source: string): string | null {
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
      const end = source.indexOf(quote, i + 1)
      if (end !== -1) {
        const directive = source.slice(i + 1, end)
        let j = end + 1
        while (j < len) {
          if (source[j] === ' ' || source[j] === '\t') {
            j++
            continue
          }
          if (source[j] === '\n' || source[j] === '\r' || source[j] === ';')
            return directive
          if (source[j] === '/' && source[j + 1] === '/') {
            j = skipSingleLineComment(source, j, len)
            continue
          }
          if (source[j] === '/' && source[j + 1] === '*') {
            j = skipMultiLineComment(source, j, len)
            continue
          }
          break
        }
        if (j >= len)
          return directive
      }
    }

    return null
  }

  return null
}

export function hasTopLevelUseServerDirective(source: string): boolean {
  return getTopLevelDirective(source) === 'use server'
}

export function hasTopLevelUseClientDirective(source: string): boolean {
  return getTopLevelDirective(source) === 'use client'
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
          if (
            afterDefault >= len
            || isWhitespace(source[afterDefault])
            || (source[afterDefault] === '/' && (source[afterDefault + 1] === '/' || source[afterDefault + 1] === '*'))
            || source[afterDefault] === '{'
            || source[afterDefault] === '('
          ) {
            return true
          }
        }
      }
    }

    i++
  }

  return false
}
