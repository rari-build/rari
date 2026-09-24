import { skipNonCodeToken } from '../analysis/directives'

const USE_SERVER = 'use server'
const REGISTER_IMPORT = 'react-server-dom-rari/server'

/** True when `code` already imports `registerServerReference` as a named binding. */
export const REGISTER_SERVER_REFERENCE_IMPORT_RE =
  /(?:^|[\n\r;])\s*import\s*\{[^}]*\bregisterServerReference\b[^}]*\}\s*from\s*["'][^"'\n]+["']/

export function hasRegisterServerReferenceImport(code: string): boolean {
  return REGISTER_SERVER_REFERENCE_IMPORT_RE.test(code)
}

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
  /** Export names already registered by the inline-action rewrite (skip in transformServerModule). */
  readonly rewrittenExportNames: readonly string[]
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
    const next = skipOneWhitespaceOrComment(source, i, len)
    if (next == null) break
    i = next
  }
  return i
}

function skipOneWhitespaceOrComment(source: string, i: number, len: number): number | null {
  const ch = source.charCodeAt(i)
  if (ch === 32 || ch === 9 || ch === 10 || ch === 13 || ch === 65279) return i + 1
  if (ch === 47 && source.charCodeAt(i + 1) === 47) {
    let j = i + 2
    while (j < len && source.charCodeAt(j) !== 10 && source.charCodeAt(j) !== 13) j++
    return j
  }
  if (ch === 47 && source.charCodeAt(i + 1) === 42) {
    let j = i + 2
    while (j < len - 1 && (source.charCodeAt(j) !== 42 || source.charCodeAt(j + 1) !== 47)) j++
    return j + 2
  }
  return null
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
    const skipped = skipNonCodeToken(source, i, len)
    if (skipped !== -1) {
      i = skipped
      continue
    }
    const ch = source.charCodeAt(i)
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
  const i = skipWhitespaceAndComments(source, bodyOpen + 1)
  if (i >= bodyClose) return false

  const ch = source.charCodeAt(i)
  if (ch !== 39 && ch !== 34) return false

  const contentStart = i + 1
  const afterString = skipString(source, i, ch)
  const contentEnd = afterString - 1
  if (contentEnd - contentStart !== USE_SERVER.length) return false
  if (!regionEquals(source, contentStart, USE_SERVER)) return false

  return isUseServerPrologueTerminator(source, afterString, bodyClose)
}

function isUseServerPrologueTerminator(source: string, start: number, bodyClose: number): boolean {
  let i = start
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

function collectDestructuredParamNames(paramsRaw: string, i: number, names: Set<string>): number {
  const ch = paramsRaw.charCodeAt(i)
  const end = skipBalanced(paramsRaw, i, ch, ch === 123 ? 125 : 93)
  for (const m of paramsRaw.slice(i, end).matchAll(/[a-z_$][\w$]*/gi)) {
    if (m[0] !== 'as') names.add(m[0])
  }
  return end
}

function collectRestParamName(paramsRaw: string, i: number, names: Set<string>): number {
  i += 3
  i = skipWhitespaceAndComments(paramsRaw, i)
  const ident = readIdent(paramsRaw, i)
  if (ident) {
    names.add(ident.name)
    return ident.end
  }
  return i
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
      i = collectDestructuredParamNames(paramsRaw, i, names)
      continue
    }
    if (ch === 46 && paramsRaw.charCodeAt(i + 1) === 46 && paramsRaw.charCodeAt(i + 2) === 46) {
      i = collectRestParamName(paramsRaw, i, names)
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

function addNamedImportLocals(specList: string, names: Set<string>): void {
  for (const part of specList.split(',')) {
    const trimmed = part.trim()
    if (!trimmed || trimmed.startsWith('type ')) continue
    const asParts = trimmed.split(/\s+as\s+/)
    const local = (asParts.at(-1) ?? '').trim()
    if (local) names.add(local)
  }
}

function collectModuleBindings(source: string): Set<string> {
  const names = new Set<string>()
  for (const m of source.matchAll(
    /\b(?:import|export)\s+(?:type\s+)?(?:\{([^}]+)\}|(\*\s+as\s+[A-Za-z_$][\w$]*)|([A-Za-z_$][\w$]*))/g,
  )) {
    if (m[1]) addNamedImportLocals(m[1], names)
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
  let i = 0
  const len = body.length

  while (i < len) {
    const skipped = skipNonCodeToken(body, i, len)
    if (skipped !== -1) {
      i = skipped
      continue
    }

    if (!isIdentStart(body.charCodeAt(i))) {
      i++
      continue
    }

    const ident = readIdent(body, i)
    if (ident == null) {
      i++
      continue
    }

    // Member access: skip `obj.prop` / `obj?.prop` property names.
    if (i > 0 && body.charCodeAt(i - 1) === 46) {
      i = ident.end
      continue
    }

    // Object-literal keys in `key: value` pairs (not ternary / labels alone).
    if (isObjectLiteralKey(body, i, ident.end)) {
      i = ident.end
      continue
    }

    if (!bound.has(ident.name)) used.add(ident.name)
    i = ident.end
  }

  return [...used]
}

function skipWsBack(source: string, i: number): number {
  while (i > 0) {
    const ch = source.charCodeAt(i - 1)
    if (ch === 32 || ch === 9 || ch === 10 || ch === 13 || ch === 65279) i--
    else break
  }
  return i
}

function keywordEndsAt(source: string, end: number, keyword: string): number | null {
  const start = end - keyword.length
  if (start < 0) return null
  if (!isKeywordAt(source, start, keyword)) return null
  return start
}

/** Object key when prev significant is `{` or `,` and next significant is `:`. */
function isObjectLiteralKey(body: string, identStart: number, identEnd: number): boolean {
  const after = skipWhitespaceAndComments(body, identEnd)
  if (body.charCodeAt(after) !== 58) return false

  const before = skipWsBack(body, identStart)
  if (before === 0) return false
  const prev = body.charCodeAt(before - 1)
  return prev === 123 || prev === 44
}

function tryResolveAssignedBindingRange(
  source: string,
  cursor: number,
  actionStart: number,
): {
  start: number
  exportKind: 'default' | 'named' | null
  bindingName: string | null
} | null {
  if (cursor <= 0 || source.charCodeAt(cursor - 1) !== 61 /* = */) return null

  const beforeEq = skipWsBack(source, cursor - 1)
  const nameEnd = beforeEq
  let nameStart = nameEnd
  while (nameStart > 0 && isIdentPart(source.charCodeAt(nameStart - 1))) nameStart--
  if (nameStart >= nameEnd || !isIdentStart(source.charCodeAt(nameStart))) return null

  const bindingName = source.slice(nameStart, nameEnd)
  const beforeName = skipWsBack(source, nameStart)
  const declStart =
    keywordEndsAt(source, beforeName, 'const') ??
    keywordEndsAt(source, beforeName, 'let') ??
    keywordEndsAt(source, beforeName, 'var')
  if (declStart == null) return null

  const beforeDecl = skipWsBack(source, declStart)
  const exportStart = keywordEndsAt(source, beforeDecl, 'export')
  if (exportStart != null) {
    return { start: actionStart, exportKind: 'named', bindingName }
  }
  return { start: actionStart, exportKind: null, bindingName }
}

function resolveActionReplaceRange(
  source: string,
  actionStart: number,
): {
  start: number
  exportKind: 'default' | 'named' | null
  bindingName: string | null
} {
  let cursor = skipWsBack(source, actionStart)

  const defaultStart = keywordEndsAt(source, cursor, 'default')
  if (defaultStart != null) {
    cursor = skipWsBack(source, defaultStart)
    const exportStart = keywordEndsAt(source, cursor, 'export')
    if (exportStart != null) return { start: exportStart, exportKind: 'default', bindingName: null }
  }

  const assigned = tryResolveAssignedBindingRange(source, cursor, actionStart)
  if (assigned != null) return assigned

  const exportStart = keywordEndsAt(source, cursor, 'export')
  if (exportStart != null) return { start: exportStart, exportKind: 'named', bindingName: null }

  return { start: actionStart, exportKind: null, bindingName: null }
}

function tryLocateFunctionAction(
  source: string,
  start: number,
  pos: number,
  isAsync: boolean,
): { action: LocatedAction | null; nextI: number } | null {
  if (!isKeywordAt(source, pos, 'function')) return null

  pos = skipWhitespaceAndComments(source, pos + 8)
  if (source.charCodeAt(pos) === 42) pos = skipWhitespaceAndComments(source, pos + 1)

  let name: string | null = null
  const ident = readIdent(source, pos)
  if (ident) {
    name = ident.name
    pos = skipWhitespaceAndComments(source, ident.end)
  }

  if (source.charCodeAt(pos) !== 40) return { action: null, nextI: start + 1 }

  const paramsEnd = skipBalanced(source, pos, 40, 41)
  const paramsRaw = source.slice(pos + 1, paramsEnd - 1)
  pos = skipWhitespaceAndComments(source, paramsEnd)
  if (source.charCodeAt(pos) !== 123) return { action: null, nextI: start + 1 }

  const bodyClose = skipBalanced(source, pos, 123, 125) - 1
  if (!hasUseServerPrologue(source, pos, bodyClose)) {
    return { action: null, nextI: pos + 1 }
  }

  return {
    action: {
      start,
      end: bodyClose + 1,
      bodyOpen: pos,
      bodyClose,
      paramsRaw,
      isAsync,
      name,
      kind: name != null ? 'declaration' : 'expression',
    },
    nextI: bodyClose + 1,
  }
}

function tryLocateArrowAction(
  source: string,
  start: number,
  paramsOpen: number,
  isAsync: boolean,
): { action: LocatedAction | null; nextI: number } | null {
  if (source.charCodeAt(paramsOpen) !== 40) return null

  const paramsEnd = skipBalanced(source, paramsOpen, 40, 41)
  const paramsRaw = source.slice(paramsOpen + 1, paramsEnd - 1)
  let after = skipWhitespaceAndComments(source, paramsEnd)
  if (source.charCodeAt(after) !== 61 || source.charCodeAt(after + 1) !== 62) return null

  after = skipWhitespaceAndComments(source, after + 2)
  if (source.charCodeAt(after) !== 123) return null

  const bodyClose = skipBalanced(source, after, 123, 125) - 1
  if (!hasUseServerPrologue(source, after, bodyClose)) {
    return { action: null, nextI: after + 1 }
  }

  return {
    action: {
      start,
      end: bodyClose + 1,
      bodyOpen: after,
      bodyClose,
      paramsRaw,
      isAsync,
      name: null,
      kind: 'arrow',
    },
    nextI: bodyClose + 1,
  }
}

// oxlint-disable typescript/prefer-readonly-parameter-types
function tryScanAsyncOrFunctionAction(
  source: string,
  i: number,
  actions: LocatedAction[],
): number | null {
  if (!isKeywordAt(source, i, 'async') && !isKeywordAt(source, i, 'function')) return null

  const start = i
  let pos = i
  let isAsync = false
  if (isKeywordAt(source, pos, 'async')) {
    isAsync = true
    pos = skipWhitespaceAndComments(source, pos + 5)
  }

  const fnResult = tryLocateFunctionAction(source, start, pos, isAsync)
  if (fnResult != null) {
    if (fnResult.action != null) actions.push(fnResult.action)
    return fnResult.nextI
  }

  if (!isAsync) return null

  const arrow = tryLocateArrowAction(source, start, pos, true)
  if (arrow == null) return null
  if (arrow.action != null) actions.push(arrow.action)
  return arrow.nextI
}

function advanceLocateActionsAt(source: string, i: number, actions: LocatedAction[]): number {
  const ch = source.charCodeAt(i)

  if (ch === 39 || ch === 34 || ch === 96) return skipString(source, i, ch)
  if (ch === 47 && (source.charCodeAt(i + 1) === 47 || source.charCodeAt(i + 1) === 42)) {
    return skipWhitespaceAndComments(source, i)
  }

  const asyncOrFn = tryScanAsyncOrFunctionAction(source, i, actions)
  if (asyncOrFn != null) return asyncOrFn

  const parenArrow = tryLocateArrowAction(source, i, i, false)
  if (parenArrow != null) {
    if (parenArrow.action != null) actions.push(parenArrow.action)
    return parenArrow.nextI
  }

  return i + 1
}

function locateInlineUseServerActions(source: string): LocatedAction[] {
  const actions: LocatedAction[] = []
  const len = source.length
  let i = 0

  while (i < len) {
    i = advanceLocateActionsAt(source, i, actions)
  }

  return actions
}

function buildActionReplacement(
  action: LocatedAction,
  bindExpr: string,
  moduleId: string,
  exportKind: 'default' | 'named' | null,
  bindingName: string | null,
): { replacement: string; rewrittenExport: string | null } {
  if (exportKind === 'default') {
    return {
      rewrittenExport: 'default',
      replacement: `export default registerServerReference(${bindExpr}, ${JSON.stringify(moduleId)}, "default")`,
    }
  }
  if (exportKind === 'named' && action.name != null) {
    return {
      rewrittenExport: action.name,
      replacement: `export const ${action.name} = registerServerReference(${bindExpr}, ${JSON.stringify(moduleId)}, ${JSON.stringify(action.name)})`,
    }
  }
  if (exportKind === 'named' && bindingName != null) {
    return {
      rewrittenExport: bindingName,
      replacement: `registerServerReference(${bindExpr}, ${JSON.stringify(moduleId)}, ${JSON.stringify(bindingName)})`,
    }
  }
  if (action.kind === 'declaration' && action.name != null) {
    return { replacement: `const ${action.name} = ${bindExpr}`, rewrittenExport: null }
  }
  return { replacement: bindExpr, rewrittenExport: null }
}

function hoistActionDeclaration(
  hoisted: string[],
  rewrittenExportNames: string[],
  asyncKw: string,
  hoistedName: string,
  params: string,
  body: string,
  moduleId: string,
  rewrittenExport: string | null,
): void {
  if (rewrittenExport == null) {
    hoisted.unshift(
      `${asyncKw}function ${hoistedName}(${params}) {\n${body}\n}`,
      `registerServerReference(${hoistedName}, ${JSON.stringify(moduleId)}, ${JSON.stringify(hoistedName)});`,
    )
    return
  }
  hoisted.unshift(`${asyncKw}function ${hoistedName}(${params}) {\n${body}\n}`)
  rewrittenExportNames.unshift(rewrittenExport)
}
// oxlint-enable typescript/prefer-readonly-parameter-types

export function transformInlineServerActions(
  code: string,
  moduleId: string,
): InlineServerActionTransformResult | null {
  const actions = locateInlineUseServerActions(code)
  if (actions.length === 0) return null

  const moduleBindings = collectModuleBindings(code)
  const actionNames: string[] = []
  const rewrittenExportNames: string[] = []
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
    const bindExpr =
      freeVars.length > 0 ? `${hoistedName}.bind(null, ${freeVars.join(', ')})` : hoistedName

    const {
      start: replaceStart,
      exportKind,
      bindingName,
    } = resolveActionReplaceRange(result, action.start)

    const { replacement, rewrittenExport } = buildActionReplacement(
      action,
      bindExpr,
      moduleId,
      exportKind,
      bindingName,
    )

    hoistActionDeclaration(
      hoisted,
      rewrittenExportNames,
      asyncKw,
      hoistedName,
      params,
      body,
      moduleId,
      rewrittenExport,
    )
    needsRegisterImport = true

    result = result.slice(0, replaceStart) + replacement + result.slice(action.end)
  }

  const registerImport = needsRegisterImport
    ? `import { registerServerReference } from ${JSON.stringify(REGISTER_IMPORT)};\n`
    : ''

  const prefix =
    needsRegisterImport && !hasRegisterServerReferenceImport(result) ? registerImport : ''

  return {
    code: `${prefix}${result}\n\n${hoisted.join('\n')}\n`,
    actionNames,
    rewrittenExportNames,
  }
}
