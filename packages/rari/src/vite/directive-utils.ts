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
        const charAfter = source[end + 1]
        if (charAfter === undefined || charAfter === ';' || charAfter === '\n' || charAfter === '\r' || charAfter === ' ' || charAfter === '\t')
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
