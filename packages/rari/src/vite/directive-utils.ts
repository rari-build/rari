const DEFAULT_EXPORT_REGEX = /^\s*export\s+default\s+/m

export function getTopLevelDirective(source: string): string | null {
  let i = 0
  const len = source.length

  while (i < len) {
    if (
      source[i] === ' '
      || source[i] === '\t'
      || source[i] === '\r'
      || source[i] === '\n'
      || source[i] === '\uFEFF'
    ) {
      i++
      continue
    }

    if (source[i] === '/' && source[i + 1] === '/') {
      while (i < len && source[i] !== '\n')
        i++
      continue
    }

    if (source[i] === '/' && source[i + 1] === '*') {
      i += 2
      while (i < len - 1 && (source[i] !== '*' || source[i + 1] !== '/'))
        i++
      i += 2
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
            while (j < len && source[j] !== '\n')
              j++
            continue
          }
          if (source[j] === '/' && source[j + 1] === '*') {
            j += 2
            while (j < len - 1 && (source[j] !== '*' || source[j + 1] !== '/'))
              j++
            j += 2
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
  return DEFAULT_EXPORT_REGEX.test(source)
}
