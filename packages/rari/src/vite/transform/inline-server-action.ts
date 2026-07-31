const USE_SERVER = 'use server'
const REGISTER_IMPORT = 'react-server-dom-rari/server'

const KNOWN_GLOBALS = new Set([
  'undefined',
  'NaN',
  'Infinity',
  'arguments',
  'Object',
  'Array',
  'String',
  'Number',
  'Boolean',
  'Symbol',
  'BigInt',
  'Math',
  'Date',
  'JSON',
  'Promise',
  'Error',
  'TypeError',
  'RangeError',
  'Map',
  'Set',
  'WeakMap',
  'WeakSet',
  'RegExp',
  'Proxy',
  'Reflect',
  'Function',
  'console',
  'fetch',
  'process',
  'Buffer',
  'globalThis',
  'global',
  'window',
  'document',
  'navigator',
  'location',
  'crypto',
  'TextEncoder',
  'TextDecoder',
  'URL',
  'URLSearchParams',
  'FormData',
  'Headers',
  'Request',
  'Response',
  'AbortController',
  'AbortSignal',
  'setTimeout',
  'clearTimeout',
  'setInterval',
  'clearInterval',
  'queueMicrotask',
  'structuredClone',
  'atob',
  'btoa',
  'React',
  'jsx',
  'jsxs',
  'Fragment',
])

const JS_KEYWORDS = new Set([
  'await',
  'return',
  'throw',
  'if',
  'else',
  'for',
  'while',
  'do',
  'switch',
  'case',
  'break',
  'continue',
  'new',
  'typeof',
  'instanceof',
  'void',
  'delete',
  'yield',
  'class',
  'extends',
  'super',
  'this',
  'try',
  'catch',
  'finally',
  'with',
  'debugger',
  'default',
  'of',
  'in',
  'from',
  'as',
  'import',
  'export',
  'async',
  'function',
  'const',
  'let',
  'var',
  'true',
  'false',
  'null',
  'static',
  'get',
  'set',
  'typeof',
])

export interface InlineServerActionTransformResult {
  readonly code: string
  readonly actionNames: readonly string[]
}

interface LocatedAction {
  readonly start: number
  readonly end: number
  readonly bodyOpen: number
  readonly bodyClose: number
  readonly paramsRaw: string
  readonly isAsync: boolean
  readonly name: string | null
  readonly kind: 'declaration' | 'expression' | 'arrow'
}

function isIdentStart(ch: number): boolean {
  return (ch >= 97 && ch <= 122) || (ch >= 65 && ch <= 90) || ch === 95 || ch === 36
}

function isIdentPart(ch: number): boolean {
  return isIdentStart(ch) || (ch >= 48 && ch <= 57)
}

function skipWhitespaceAndComments(source: string, i: number): number {
  const len = source.length
  while (i < len) {
    const ch = source.charCodeAt(i)
    if (ch === 32 || ch === 9 || ch === 10 || ch === 13 || ch === 0xFEFF) {
      i++
      continue
    }
    if (ch === 47 && source.charCodeAt(i + 1) === 47) {
      i += 2
      while (i < len && source.charCodeAt(i) !== 10 && source.charCodeAt(i) !== 13) i++
      continue
    }
    if (ch === 47 && source.charCodeAt(i + 1) === 42) {
      i += 2
      while (i < len - 1 && (source.charCodeAt(i) !== 42 || source.charCodeAt(i + 1) !== 47)) i++
      i += 2
      continue
    }
    break
  }
  return i
}

function skipString(source: string, i: number, quote: number): number {
  i++
  const len = source.length
  while (i < len) {
    const ch = source.charCodeAt(i)
    if (ch === 92) {
      i += 2
      continue
    }
    if (ch === quote) return i + 1
    i++
  }
  return i
}

function skipBalanced(source: string, start: number, open: number, close: number): number {
  let i = start
  let depth = 0
  const len = source.length
  while (i < len) {
    const ch = source.charCodeAt(i)
    if (ch === 39 || ch === 34 || ch === 96) {
      i = skipString(source, i, ch)
      continue
    }
    if (ch === 47 && source.charCodeAt(i + 1) === 47) {
      i = skipWhitespaceAndComments(source, i)
      continue
    }
    if (ch === 47 && source.charCodeAt(i + 1) === 42) {
      i = skipWhitespaceAndComments(source, i)
      continue
    }
    if (ch === open) depth++
    else if (ch === close) {
      depth--
      if (depth === 0) return i + 1
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

function hasUseServerPrologue(source: string, bodyOpen: number, bodyClose: number): boolean {
  let i = skipWhitespaceAndComments(source, bodyOpen + 1)
  if (i >= bodyClose) return false

  const ch = source.charCodeAt(i)
  if (ch !== 39 && ch !== 34) return false

  const contentStart = i + 1
  const afterString = skipString(source, i, ch)
  const contentEnd = afterString - 1
  if (contentEnd - contentStart !== USE_SERVER.length) return false
  if (!regionEquals(source, contentStart, USE_SERVER)) return false

  i = afterString
  while (i < bodyClose) {
    const next = source.charCodeAt(i)
    if (next === 32 || next === 9) {
      i++
      continue
    }
    if (next === 59 || next === 10 || next === 13) return true
    if (next === 47 && source.charCodeAt(i + 1) === 47) return true
    if (next === 47 && source.charCodeAt(i + 1) === 42) return true
    return false
  }

  return true
}

function stripUseServerPrologue(source: string, bodyOpen: number, bodyClose: number): string {
  let i = skipWhitespaceAndComments(source, bodyOpen + 1)
  const ch = source.charCodeAt(i)
  if (ch !== 39 && ch !== 34) return source.slice(bodyOpen + 1, bodyClose)

  const afterString = skipString(source, i, ch)
  i = skipWhitespaceAndComments(source, afterString)
  if (source.charCodeAt(i) === 59) i++
  i = skipWhitespaceAndComments(source, i)
  return source.slice(i, bodyClose)
}

function readIdent(source: string, i: number): { name: string; end: number } | null {
  if (!isIdentStart(source.charCodeAt(i))) return null
  let j = i + 1
  while (j < source.length && isIdentPart(source.charCodeAt(j))) j++
  return { name: source.slice(i, j), end: j }
}

function isKeywordAt(source: string, i: number, keyword: string): boolean {
  if (!regionEquals(source, i, keyword)) return false
  const before = i > 0 ? source.charCodeAt(i - 1) : -1
  const after = i + keyword.length < source.length ? source.charCodeAt(i + keyword.length) : -1
  if (before !== -1 && isIdentPart(before)) return false
  if (after !== -1 && isIdentPart(after)) return false
  return true
}

function collectParamNames(paramsRaw: string): Set<string> {
  const names = new Set<string>()
  let i = 0
  const len = paramsRaw.length
  while (i < len) {
    i = skipWhitespaceAndComments(paramsRaw, i)
    if (i >= len) break
    const ch = paramsRaw.charCodeAt(i)
    if (ch === 123 || ch === 91) {
      const end = skipBalanced(paramsRaw, i, ch, ch === 123 ? 125 : 93)
      for (const m of paramsRaw.slice(i, end).matchAll(/[a-z_$][\w$]*/gi)) {
        if (m[0] !== 'as') names.add(m[0])
      }
      i = end
      continue
    }
    if (ch === 46 && paramsRaw.charCodeAt(i + 1) === 46 && paramsRaw.charCodeAt(i + 2) === 46) {
      i += 3
      i = skipWhitespaceAndComments(paramsRaw, i)
      const ident = readIdent(paramsRaw, i)
      if (ident) {
        names.add(ident.name)
        i = ident.end
      }
      continue
    }
    const ident = readIdent(paramsRaw, i)
    if (ident) {
      names.add(ident.name)
      i = ident.end
      continue
    }
    i++
  }
  return names
}

function collectLocalDeclarations(body: string): Set<string> {
  const names = new Set<string>()
  for (const m of body.matchAll(
    /\b(?:const|let|var|function|class|async\s+function)\s+([A-Za-z_$][\w$]*)/g,
  )) {
    names.add(m[1])
  }
  return names
}

function collectModuleBindings(source: string): Set<string> {
  const names = new Set<string>()
  for (const m of source.matchAll(
    /\b(?:import|export)\s+(?:type\s+)?(?:\{([^}]+)\}|(\*\s+as\s+[A-Za-z_$][\w$]*)|([A-Za-z_$][\w$]*))/g,
  )) {
    if (m[1]) {
      for (const part of m[1].split(',')) {
        const trimmed = part.trim()
        if (!trimmed || trimmed.startsWith('type ')) continue
        const asParts = trimmed.split(/\s+as\s+/)
        const local = (asParts.at(-1) ?? '').trim()
        if (local) names.add(local)
      }
    }
    if (m[2]) {
      const ns = /as\s+([A-Za-z_$][\w$]*)/.exec(m[2])
      if (ns) names.add(ns[1])
    }
    if (m[3] && m[3] !== 'type' && m[3] !== 'async' && m[3] !== 'function' && m[3] !== 'class') {
      names.add(m[3])
    }
  }
  for (const m of source.matchAll(
    /(?:^|\n)\s*(?:export\s+)?(?:async\s+)?(?:function|class|const|let|var)\s+([A-Za-z_$][\w$]*)/g,
  )) {
    names.add(m[1])
  }
  return names
}

function collectFreeVars(
  body: string,
  paramsRaw: string,
  moduleBindings: ReadonlySet<string>,
): string[] {
  const bound = new Set<string>([
    ...collectParamNames(paramsRaw),
    ...collectLocalDeclarations(body),
    ...moduleBindings,
    ...KNOWN_GLOBALS,
    ...JS_KEYWORDS,
  ])

  const used = new Set<string>()
  for (const m of body.matchAll(/\b([a-z_$][\w$]*)\b/gi)) {
    const name = m[1]
    if (bound.has(name)) continue
    const idx = m.index
    if (idx > 0 && body[idx - 1] === '.') continue
    used.add(name)
  }

  return [...used]
}

function locateInlineUseServerActions(source: string): LocatedAction[] {
  const actions: LocatedAction[] = []
  const len = source.length
  let i = 0

  while (i < len) {
    const ch = source.charCodeAt(i)

    if (ch === 39 || ch === 34 || ch === 96) {
      i = skipString(source, i, ch)
      continue
    }
    if (ch === 47 && (source.charCodeAt(i + 1) === 47 || source.charCodeAt(i + 1) === 42)) {
      i = skipWhitespaceAndComments(source, i)
      continue
    }

    if (isKeywordAt(source, i, 'async') || isKeywordAt(source, i, 'function')) {
      const start = i
      let pos = i
      let isAsync = false
      if (isKeywordAt(source, pos, 'async')) {
        isAsync = true
        pos = skipWhitespaceAndComments(source, pos + 5)
      }

      if (isKeywordAt(source, pos, 'function')) {
        pos = skipWhitespaceAndComments(source, pos + 8)
        if (source.charCodeAt(pos) === 42) pos = skipWhitespaceAndComments(source, pos + 1)

        let name: string | null = null
        const ident = readIdent(source, pos)
        if (ident) {
          name = ident.name
          pos = skipWhitespaceAndComments(source, ident.end)
        }

        if (source.charCodeAt(pos) !== 40) {
          i++
          continue
        }

        const paramsEnd = skipBalanced(source, pos, 40, 41)
        const paramsRaw = source.slice(pos + 1, paramsEnd - 1)
        pos = skipWhitespaceAndComments(source, paramsEnd)
        if (source.charCodeAt(pos) !== 123) {
          i++
          continue
        }

        const bodyClose = skipBalanced(source, pos, 123, 125) - 1
        if (hasUseServerPrologue(source, pos, bodyClose)) {
          actions.push({
            start,
            end: bodyClose + 1,
            bodyOpen: pos,
            bodyClose,
            paramsRaw,
            isAsync,
            name,
            kind: name != null ? 'declaration' : 'expression',
          })
          i = bodyClose + 1
        } else {
          i = pos + 1
        }
        continue
      }

      if (isAsync && source.charCodeAt(pos) === 40) {
        const paramsEnd = skipBalanced(source, pos, 40, 41)
        const paramsRaw = source.slice(pos + 1, paramsEnd - 1)
        let after = skipWhitespaceAndComments(source, paramsEnd)
        if (source.charCodeAt(after) === 61 && source.charCodeAt(after + 1) === 62) {
          after = skipWhitespaceAndComments(source, after + 2)
          if (source.charCodeAt(after) === 123) {
            const bodyClose = skipBalanced(source, after, 123, 125) - 1
            if (hasUseServerPrologue(source, after, bodyClose)) {
              actions.push({
                start,
                end: bodyClose + 1,
                bodyOpen: after,
                bodyClose,
                paramsRaw,
                isAsync: true,
                name: null,
                kind: 'arrow',
              })
              i = bodyClose + 1
            } else {
              i = after + 1
            }
            continue
          }
        }
      }
    }

    if (source.charCodeAt(i) === 40) {
      const paramsEnd = skipBalanced(source, i, 40, 41)
      let after = skipWhitespaceAndComments(source, paramsEnd)
      if (source.charCodeAt(after) === 61 && source.charCodeAt(after + 1) === 62) {
        after = skipWhitespaceAndComments(source, after + 2)
        if (source.charCodeAt(after) === 123) {
          const bodyClose = skipBalanced(source, after, 123, 125) - 1
          if (hasUseServerPrologue(source, after, bodyClose)) {
            actions.push({
              start: i,
              end: bodyClose + 1,
              bodyOpen: after,
              bodyClose,
              paramsRaw: source.slice(i + 1, paramsEnd - 1),
              isAsync: false,
              name: null,
              kind: 'arrow',
            })
            i = bodyClose + 1
          } else {
            i = after + 1
          }
          continue
        }
      }
    }

    i++
  }

  return actions
}

export function transformInlineServerActions(
  code: string,
  moduleId: string,
): InlineServerActionTransformResult | null {
  const actions = locateInlineUseServerActions(code)
  if (actions.length === 0) return null

  const moduleBindings = collectModuleBindings(code)
  const actionNames: string[] = []
  let result = code
  const hoisted: string[] = []
  let needsRegisterImport = false

  const ordered = [...actions].sort((a, b) => b.start - a.start)
  let actionIndex = actions.length - 1

  for (const action of ordered) {
    const originalName = action.name ?? 'anonymous_server_function'
    const hoistedName = `$$ACTION_${actionIndex}_${originalName.replace(/\W/g, '_')}`
    actionIndex--
    actionNames.unshift(hoistedName)

    const body = stripUseServerPrologue(result, action.bodyOpen, action.bodyClose)
    const freeVars = collectFreeVars(body, action.paramsRaw, moduleBindings)
    const params = [...freeVars, action.paramsRaw.trim()].filter(p => p !== '').join(', ')
    const asyncKw = action.isAsync ? 'async ' : ''

    hoisted.unshift(
      `${asyncKw}function ${hoistedName}(${params}) {\n${body}\n}`,
      `registerServerReference(${hoistedName}, ${JSON.stringify(moduleId)}, ${JSON.stringify(hoistedName)});`,
    )
    needsRegisterImport = true

    const bindExpr =
      freeVars.length > 0 ? `${hoistedName}.bind(null, ${freeVars.join(', ')})` : hoistedName

    let replacement = bindExpr
    if (action.kind === 'declaration' && action.name != null) {
      replacement = `const ${action.name} = ${bindExpr}`
    }

    result = result.slice(0, action.start) + replacement + result.slice(action.end)
  }

  const registerImport = needsRegisterImport
    ? `import { registerServerReference } from ${JSON.stringify(REGISTER_IMPORT)};\n`
    : ''

  const prefix =
    needsRegisterImport && !result.includes('registerServerReference')
      ? registerImport
      : needsRegisterImport && !/import\s*\{[^}]*registerServerReference/.test(result)
        ? registerImport
        : ''

  return {
    code: `${prefix}${result}\n\n${hoisted.join('\n')}\n`,
    actionNames,
  }
}
