import fs from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { resolveAlias } from '@/shared/utils/alias-resolver'
import { contentHash } from '@/shared/utils/content-hash'
import { normalizeAssetsDir, toPosixPath } from '@/shared/utils/path'
import { scanImportStatements } from '../analysis/directives'
import { collectLayoutCssImportPaths } from '../client-head'

const BARE_PACKAGE_CSS_IMPORT_RE =
  /@import\s+(?:url\(\s*)?['"](?![a-zA-Z][a-zA-Z0-9+.-]*:|\/\/|\.\/|\.\.\/|\/)[^'"]+['"][^;]*(?:;|$)/g

const LOCAL_RELATIVE_CSS_IMPORT_RE =
  /@import\s+(?:url\(\s*((?:\.\/|\.\.\/)[^)\s]+)\s*\)|url\(\s*['"]((?:\.\/|\.\.\/)[^'"]+)['"]\s*\)|['"]((?:\.\/|\.\.\/)[^'"]+)['"])([^;]*)(?:;|$)/g

const CSS_IMPORT_SOURCE_RE = /\.css(?:\?.*)?$/
const CSS_MODULE_PATH_RE = /\.module\.css(?:\?.*)?$/i

function isCssModulePath(sourceOrPath: string): boolean {
  return CSS_MODULE_PATH_RE.test(sourceOrPath.replace(/[?#].*$/, ''))
}

function stripBarePackageCssImports(css: string): string {
  return css.replace(BARE_PACKAGE_CSS_IMPORT_RE, '').trim()
}

interface CssImportQualifiers {
  readonly layerName: string | null
  readonly supportsCondition: string | null
  readonly mediaQuery: string | null
}

function extractSupportsCondition(rest: string): {
  supportsCondition: string | null
  rest: string
} {
  if (!/^supports\s*\(/i.test(rest)) return { supportsCondition: null, rest }

  const openParen = rest.indexOf('(')
  let depth = 0
  let end = -1
  for (let i = openParen; i < rest.length; i++) {
    const ch = rest[i]
    if (ch === '(') depth++
    else if (ch === ')') {
      depth--
      if (depth === 0) {
        end = i
        break
      }
    }
  }
  if (end === -1) return { supportsCondition: null, rest }

  return {
    supportsCondition: rest.slice(openParen + 1, end).trim(),
    rest: rest.slice(end + 1).trim(),
  }
}

function extractLayerName(rest: string): { layerName: string | null; rest: string } {
  const layerOpen = /^layer\s*\(/i.exec(rest)
  if (layerOpen) {
    const openParen = rest.indexOf('(')
    const closeParen = rest.indexOf(')', openParen)
    if (closeParen === -1) return { layerName: null, rest }
    return {
      layerName: rest.slice(openParen + 1, closeParen).trim(),
      rest: rest.slice(closeParen + 1).trim(),
    }
  }
  if (/^layer(?:\s|$)/i.test(rest)) {
    return { layerName: '', rest: rest.replace(/^layer\s*/i, '') }
  }
  return { layerName: null, rest }
}

function extractImportQualifiers(suffix: string): CssImportQualifiers {
  let rest = suffix.trim().replace(/^\)\s*/, '')

  const layer = extractLayerName(rest)
  rest = layer.rest

  const supports = extractSupportsCondition(rest)
  rest = supports.rest

  const mediaQuery = rest.trim() === '' ? null : rest.trim()
  return {
    layerName: layer.layerName,
    supportsCondition: supports.supportsCondition,
    mediaQuery,
  }
}

function applyImportQualifiers(css: string, qualifiers: CssImportQualifiers): string {
  let out = css.trim()
  if (out === '') return ''

  if (qualifiers.layerName !== null) {
    out =
      qualifiers.layerName === ''
        ? `@layer {\n${out}\n}`
        : `@layer ${qualifiers.layerName} {\n${out}\n}`
  }
  if (qualifiers.supportsCondition != null && qualifiers.supportsCondition !== '') {
    out = `@supports (${qualifiers.supportsCondition}) {\n${out}\n}`
  }
  if (qualifiers.mediaQuery != null && qualifiers.mediaQuery !== '') {
    out = `@media ${qualifiers.mediaQuery} {\n${out}\n}`
  }
  return out
}

function inlineLocalRelativeCssImports(
  filePath: string,
  css: string,
  seen: Set<string> = new Set(),
): string {
  const absPath = path.resolve(filePath)
  if (seen.has(absPath)) return ''
  seen.add(absPath)

  return css
    .replace(
      LOCAL_RELATIVE_CSS_IMPORT_RE,
      (
        fullMatch,
        urlUnquoted: string | undefined,
        urlQuoted: string | undefined,
        plainQuoted: string | undefined,
        qualifierSuffix: string,
      ) => {
        const relPath = urlUnquoted ?? urlQuoted ?? plainQuoted
        if (relPath == null || relPath === '') return fullMatch
        const nestedPath = path.resolve(path.dirname(absPath), relPath)
        try {
          if (!fs.existsSync(nestedPath) || !fs.statSync(nestedPath).isFile()) return fullMatch
          const nested = fs.readFileSync(nestedPath, 'utf-8')
          const prepared = preparePlainCssForServerAsset(nestedPath, nested, new Set(seen))
          return applyImportQualifiers(prepared, extractImportQualifiers(qualifierSuffix))
        } catch {
          return fullMatch
        }
      },
    )
    .trim()
}

export function preparePlainCssForServerAsset(
  filePath: string,
  css: string,
  seen: Set<string> = new Set(),
): string {
  return inlineLocalRelativeCssImports(filePath, stripBarePackageCssImports(css), seen)
}

export function resolveLayoutCssServerSkipSet(
  projectRoot: string,
  aliases: Readonly<Record<string, string>>,
): Set<string> {
  const skip = new Set<string>()
  for (const cssImport of collectLayoutCssImportPaths(projectRoot, aliases)) {
    if (path.isAbsolute(cssImport)) {
      skip.add(path.resolve(cssImport))
      continue
    }
    try {
      skip.add(createRequire(path.join(projectRoot, 'package.json')).resolve(cssImport))
    } catch {
      // Bare package not resolvable here; client head still owns the import.
    }
  }
  return skip
}

function existingFile(candidate: string): string | null {
  return fs.existsSync(candidate) && fs.statSync(candidate).isFile() ? candidate : null
}

function tryResolveBareCss(bare: string, fromFile: string, projectRoot: string): string | null {
  const resolveFrom = [fromFile, path.join(projectRoot, 'package.json')]
  for (const from of resolveFrom) {
    try {
      const resolved = existingFile(createRequire(from).resolve(bare))
      if (resolved != null) return resolved
    } catch {
      try {
        const resolved = existingFile(
          fileURLToPath(import.meta.resolve(bare, pathToFileURL(from).href)),
        )
        if (resolved != null) return resolved
      } catch {
        // try next resolve root
      }
    }
  }
  return null
}

function resolveCssFilePath(
  source: string,
  fromFile: string,
  projectRoot: string,
  aliases: Readonly<Record<string, string>>,
): string | null {
  const bare = source.replace(/[?#].*$/, '')
  if (!CSS_IMPORT_SOURCE_RE.test(bare)) return null

  if (path.isAbsolute(bare)) return existingFile(bare)

  if (bare.startsWith('./') || bare.startsWith('../')) {
    return existingFile(path.resolve(path.dirname(fromFile), bare))
  }

  const aliased = resolveAlias(bare, aliases, projectRoot)
  if (aliased != null && aliased !== '') {
    const resolved = existingFile(aliased)
    if (resolved != null) return resolved
  }

  return tryResolveBareCss(bare, fromFile, projectRoot)
}

function shouldSkipCssQueryImport(source: string): boolean {
  const query = source.includes('?') ? source.slice(source.indexOf('?')) : ''
  return /(?:\?|&)raw(?:&|$)/.test(query) || /(?:\?|&)url(?:&|$)/.test(query)
}

function readPreparedServerCss(
  resolved: string,
  layoutCssSkip: ReadonlySet<string>,
): string | null {
  if (isCssModulePath(resolved)) return null
  if (layoutCssSkip.has(path.resolve(resolved))) return null
  try {
    const content = fs.readFileSync(resolved, 'utf-8')
    const prepared = preparePlainCssForServerAsset(resolved, content)
    return prepared !== '' ? prepared : null
  } catch {
    return null
  }
}

export function collectComponentServerCssSources(options: {
  readonly filePath: string
  readonly code: string
  readonly projectRoot: string
  readonly aliases: Readonly<Record<string, string>>
  readonly layoutCssSkip: ReadonlySet<string>
}): string[] {
  const cssModules: string[] = []
  for (const imp of scanImportStatements(options.code)) {
    if (!CSS_IMPORT_SOURCE_RE.test(imp.source)) continue
    if (isCssModulePath(imp.source)) continue
    if (shouldSkipCssQueryImport(imp.source)) continue

    const resolved = resolveCssFilePath(
      imp.source,
      options.filePath,
      options.projectRoot,
      options.aliases,
    )
    if (resolved == null) continue

    const prepared = readPreparedServerCss(resolved, options.layoutCssSkip)
    if (prepared != null) cssModules.push(prepared)
  }
  return cssModules
}

function nextSyntheticBinding(
  defaultBinding: string | null | undefined,
  prefix: string,
  count: number,
): { binding: string; nextCount: number } {
  if (defaultBinding != null) return { binding: defaultBinding, nextCount: count }
  return {
    binding: `${prefix}${count === 0 ? '' : count}`,
    nextCount: count + 1,
  }
}

function emitCssUrlAsset(
  resolved: string,
  projectRoot: string,
  assetsDirName: string,
): { fileName: string; href: string; code: string } {
  const content = fs.readFileSync(resolved)
  const ext = path.extname(resolved) || '.css'
  const base = path.basename(resolved, ext)
  const relativeSource = toPosixPath(path.relative(projectRoot, resolved))
  const hash = contentHash(`${relativeSource}:${content.toString('utf8')}`, 8)
  const fileName = `${base}-${hash}${ext}`
  const relativeFileName = `${assetsDirName}/${fileName}`
  return {
    fileName: relativeFileName,
    href: `/${relativeFileName}`,
    code: content.toString('utf8'),
  }
}

export function transformCssQueryImportsForEmit(options: {
  readonly filePath: string
  readonly code: string
  readonly projectRoot: string
  readonly aliases: Readonly<Record<string, string>>
  readonly assetsDir?: string
}): {
  readonly code: string
  readonly extraFiles: Array<{ readonly fileName: string; readonly code: string }>
} {
  const assetsDirName = normalizeAssetsDir(options.assetsDir ?? 'assets')
  const extraFiles: Array<{ readonly fileName: string; readonly code: string }> = []
  const replacements: Array<{ start: number; end: number; code: string }> = []
  let syntheticBindingCount = 0

  for (const imp of scanImportStatements(options.code)) {
    if (!CSS_IMPORT_SOURCE_RE.test(imp.source)) continue
    const queryIndex = imp.source.indexOf('?')
    if (queryIndex === -1) continue
    const query = imp.source.slice(queryIndex)
    const isRaw = /(?:\?|&)raw(?:&|$)/.test(query)
    const isUrl = /(?:\?|&)url(?:&|$)/.test(query)
    if (!isRaw && !isUrl) continue

    const resolved = resolveCssFilePath(
      imp.source,
      options.filePath,
      options.projectRoot,
      options.aliases,
    )
    if (resolved == null) continue

    if (isRaw) {
      const content = fs.readFileSync(resolved, 'utf-8')
      const { binding, nextCount } = nextSyntheticBinding(
        imp.defaultBinding,
        'cssRaw',
        syntheticBindingCount,
      )
      syntheticBindingCount = nextCount
      replacements.push({
        start: imp.start,
        end: imp.end,
        code: `const ${binding} = ${JSON.stringify(content)};`,
      })
      continue
    }

    const asset = emitCssUrlAsset(resolved, options.projectRoot, assetsDirName)
    extraFiles.push({ fileName: asset.fileName, code: asset.code })
    const { binding, nextCount } = nextSyntheticBinding(
      imp.defaultBinding,
      'cssUrl',
      syntheticBindingCount,
    )
    syntheticBindingCount = nextCount
    replacements.push({
      start: imp.start,
      end: imp.end,
      code: `const ${binding} = ${JSON.stringify(asset.href)};`,
    })
  }

  if (replacements.length === 0) {
    return { code: options.code, extraFiles }
  }

  replacements.sort((a, b) => b.start - a.start)
  let code = options.code
  for (const replacement of replacements) {
    code = `${code.slice(0, replacement.start)}${replacement.code}${code.slice(replacement.end)}`
  }
  return { code, extraFiles }
}
