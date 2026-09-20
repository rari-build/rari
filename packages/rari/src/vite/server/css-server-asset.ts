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

function extractImportQualifiers(suffix: string): CssImportQualifiers {
  let rest = suffix.trim().replace(/^\)\s*/, '')
  let layerName: string | null = null
  let supportsCondition: string | null = null

  const layerOpen = /^layer\s*\(/i.exec(rest)
  if (layerOpen) {
    const openParen = rest.indexOf('(')
    const closeParen = rest.indexOf(')', openParen)
    if (closeParen !== -1) {
      layerName = rest.slice(openParen + 1, closeParen).trim()
      rest = rest.slice(closeParen + 1).trim()
    }
  } else if (/^layer(?:\s|$)/i.test(rest)) {
    layerName = ''
    rest = rest.replace(/^layer\s*/i, '')
  }

  if (/^supports\s*\(/i.test(rest)) {
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
    if (end !== -1) {
      supportsCondition = rest.slice(openParen + 1, end).trim()
      rest = rest.slice(end + 1).trim()
    }
  }

  const mediaQuery = rest.trim() === '' ? null : rest.trim()
  return { layerName, supportsCondition, mediaQuery }
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

function resolveCssFilePath(
  source: string,
  fromFile: string,
  projectRoot: string,
  aliases: Readonly<Record<string, string>>,
): string | null {
  const bare = source.replace(/[?#].*$/, '')
  if (!CSS_IMPORT_SOURCE_RE.test(bare)) return null

  if (path.isAbsolute(bare)) {
    return fs.existsSync(bare) && fs.statSync(bare).isFile() ? bare : null
  }

  if (bare.startsWith('./') || bare.startsWith('../')) {
    const resolved = path.resolve(path.dirname(fromFile), bare)
    return fs.existsSync(resolved) && fs.statSync(resolved).isFile() ? resolved : null
  }

  const aliased = resolveAlias(bare, aliases, projectRoot)
  if (
    aliased != null &&
    aliased !== '' &&
    fs.existsSync(aliased) &&
    fs.statSync(aliased).isFile()
  ) {
    return aliased
  }

  const resolveFrom = [fromFile, path.join(projectRoot, 'package.json')]
  for (const from of resolveFrom) {
    try {
      const resolved = createRequire(from).resolve(bare)
      if (fs.existsSync(resolved) && fs.statSync(resolved).isFile()) return resolved
    } catch {
      try {
        const resolved = fileURLToPath(import.meta.resolve(bare, pathToFileURL(from).href))
        if (fs.existsSync(resolved) && fs.statSync(resolved).isFile()) return resolved
      } catch {
        // try next resolve root
      }
    }
  }

  return null
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
    const query = imp.source.includes('?') ? imp.source.slice(imp.source.indexOf('?')) : ''
    if (/(?:\?|&)raw(?:&|$)/.test(query) || /(?:\?|&)url(?:&|$)/.test(query)) continue

    const resolved = resolveCssFilePath(
      imp.source,
      options.filePath,
      options.projectRoot,
      options.aliases,
    )
    if (resolved == null) continue
    if (isCssModulePath(resolved)) continue
    if (options.layoutCssSkip.has(path.resolve(resolved))) continue

    try {
      const content = fs.readFileSync(resolved, 'utf-8')
      const prepared = preparePlainCssForServerAsset(resolved, content)
      if (prepared !== '') cssModules.push(prepared)
    } catch {
      // Skip unreadable CSS; emit path still owns the JS bundle.
    }
  }
  return cssModules
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
      const binding =
        imp.defaultBinding ?? `cssRaw${syntheticBindingCount === 0 ? '' : syntheticBindingCount}`
      if (imp.defaultBinding == null) syntheticBindingCount++
      replacements.push({
        start: imp.start,
        end: imp.end,
        code: `const ${binding} = ${JSON.stringify(content)};`,
      })
      continue
    }

    const content = fs.readFileSync(resolved)
    const ext = path.extname(resolved) || '.css'
    const base = path.basename(resolved, ext)
    const relativeSource = toPosixPath(path.relative(options.projectRoot, resolved))
    const hash = contentHash(`${relativeSource}:${content.toString('utf8')}`, 8)
    const fileName = `${base}-${hash}${ext}`
    const relativeFileName = `${assetsDirName}/${fileName}`
    extraFiles.push({ fileName: relativeFileName, code: content.toString('utf8') })
    const href = `/${relativeFileName}`
    const binding =
      imp.defaultBinding ?? `cssUrl${syntheticBindingCount === 0 ? '' : syntheticBindingCount}`
    if (imp.defaultBinding == null) syntheticBindingCount++
    replacements.push({
      start: imp.start,
      end: imp.end,
      code: `const ${binding} = ${JSON.stringify(href)};`,
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
