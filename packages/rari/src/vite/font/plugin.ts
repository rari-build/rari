/* oxlint-disable typescript/prefer-readonly-parameter-types font option bags and transform params mutate during parse */
import type { Plugin } from 'vite-plus'
import type { FallbackFontName } from './metrics'
import type { JsonValue } from './parse-options'
import type {
  Font,
  FontDisplay,
  GoogleFontOptions,
  LocalFontOptions,
  ResolvedFontFace,
} from '@/font/types'
import fs from 'node:fs'
import path from 'node:path'
import { fromBuffer } from '@capsizecss/unpack'
import { buildFontFamilyStack, fontMimeType, serializeFontFaceRule } from './css'
import {
  fontPreloadMarker,
  googleExportNameToFamily,
  resolveGoogleFontFaces,
} from './google-loader'
import { classNameFromHash, contentHash, hashedFontFileName, publicFontUrl } from './hash'
import { resolveLocalFontFaces } from './local-resolve'
import { categoryFallback, computeFallbackOverrides, loadMetricsForFamily } from './metrics'
import { extractObjectLiteral } from './parse-options'

const LOCAL_ID = 'rari/font/local'
const GOOGLE_ID = 'rari/font/google'
const CSS_PREFIX = '\0rari-font-css:'
const FONT_EXT_RE = /\.(?:woff2?|ttf|otf)$/i

export interface FontTransformResult {
  readonly code: string
  readonly cssModules: ReadonlyArray<{ readonly id: string; readonly css: string }>
  readonly assets: ReadonlyArray<{ readonly fileName: string; readonly source: Buffer }>
  readonly preloadUrls: readonly string[]
}

interface PreparedFont {
  readonly font: Font
  readonly css: string
  readonly cssId: string
  readonly assets: ReadonlyArray<{ readonly fileName: string; readonly source: Buffer }>
  readonly preloadUrls: readonly string[]
}

function asStringArray(value: JsonValue | undefined): string[] | undefined {
  if (value == null) return undefined
  if (typeof value === 'string') return [value]
  if (!Array.isArray(value)) return undefined
  return value.filter((item): item is string => typeof item === 'string')
}

function asFontDisplay(value: JsonValue | undefined): FontDisplay | undefined {
  if (
    value === 'auto' ||
    value === 'block' ||
    value === 'swap' ||
    value === 'fallback' ||
    value === 'optional'
  ) {
    return value
  }
  return undefined
}

function asLocalOptions(value: Record<string, JsonValue>): LocalFontOptions {
  const srcValue = value.src
  let src: LocalFontOptions['src']
  if (typeof srcValue === 'string') {
    src = srcValue
  } else if (Array.isArray(srcValue)) {
    src = srcValue.map(entry => {
      if (typeof entry === 'string') return { path: entry }
      if (entry == null || typeof entry !== 'object' || Array.isArray(entry)) {
        throw new Error('rari/font/local: invalid src entry')
      }
      const pathValue = entry.path
      if (typeof pathValue !== 'string')
        throw new Error('rari/font/local: src.path must be a string')
      return {
        path: pathValue,
        weight:
          typeof entry.weight === 'string' || typeof entry.weight === 'number'
            ? entry.weight
            : undefined,
        style: typeof entry.style === 'string' ? entry.style : undefined,
      }
    })
  } else {
    throw new TypeError('rari/font/local: `src` is required')
  }

  return {
    src,
    display: asFontDisplay(value.display),
    weight:
      typeof value.weight === 'string' || typeof value.weight === 'number'
        ? value.weight
        : undefined,
    style: typeof value.style === 'string' ? value.style : undefined,
    variable: typeof value.variable === 'string' ? value.variable : undefined,
    preload: typeof value.preload === 'boolean' ? value.preload : undefined,
    fallback: asStringArray(value.fallback),
    adjustFontFallback:
      value.adjustFontFallback === false
        ? false
        : value.adjustFontFallback === 'Arial' || value.adjustFontFallback === 'Times New Roman'
          ? value.adjustFontFallback
          : undefined,
    declarations: asDeclarations(value.declarations),
  }
}

function asDeclarations(
  value: JsonValue | undefined,
): LocalFontOptions['declarations'] | undefined {
  if (!Array.isArray(value)) return undefined
  const declarations: Array<{ prop: string; value: string }> = []
  for (const entry of value) {
    if (entry == null || typeof entry !== 'object' || Array.isArray(entry)) continue
    const prop = entry.prop
    const declValue = entry.value
    if (typeof prop === 'string' && typeof declValue === 'string') {
      declarations.push({ prop, value: declValue })
    }
  }
  return declarations.length > 0 ? declarations : undefined
}

function asGoogleOptions(value: Record<string, JsonValue>): GoogleFontOptions {
  const weight = value.weight
  const style = value.style
  return {
    weight:
      typeof weight === 'string' || typeof weight === 'number'
        ? weight
        : Array.isArray(weight)
          ? weight.filter(
              (item): item is string | number =>
                typeof item === 'string' || typeof item === 'number',
            )
          : undefined,
    style:
      typeof style === 'string'
        ? style
        : Array.isArray(style)
          ? style.filter((item): item is string => typeof item === 'string')
          : undefined,
    subsets: asStringArray(value.subsets),
    display: asFontDisplay(value.display),
    variable: typeof value.variable === 'string' ? value.variable : undefined,
    preload: typeof value.preload === 'boolean' ? value.preload : undefined,
    fallback: asStringArray(value.fallback),
    adjustFontFallback:
      typeof value.adjustFontFallback === 'boolean' ? value.adjustFontFallback : undefined,
    axes: asStringArray(value.axes),
  }
}

function findClosingParen(source: string, openParenIndex: number): number {
  let depth = 0
  for (let i = openParenIndex; i < source.length; i += 1) {
    const ch = source[i]
    if (ch === '(') depth += 1
    else if (ch === ')') {
      depth -= 1
      if (depth === 0) return i
    }
  }
  return -1
}

async function prepareFaces(
  faces: ResolvedFontFace[],
  assetsDir: string,
  family: string,
): Promise<PreparedFont> {
  const assets: Array<{ fileName: string; source: Buffer }> = []
  const emittedFaces: ResolvedFontFace[] = []
  const hashParts: string[] = []

  for (const face of faces) {
    if (face.filePaths.length === 0) continue
    const filePath = face.filePaths[0]
    const source = fs.readFileSync(filePath)
    const hash = contentHash(source)
    hashParts.push(hash)
    const fileName = hashedFontFileName(filePath, hash, assetsDir)
    assets.push({ fileName, source })
    emittedFaces.push({
      ...face,
      family,
      src: [{ url: publicFontUrl(fileName), format: face.src[0]?.format ?? 'woff2' }],
    })
  }

  if (emittedFaces.length === 0) throw new Error('rari/font: no font files resolved')

  const primary = emittedFaces[0]
  const optionsIdentity = JSON.stringify({
    display: primary.display,
    weight: emittedFaces.map(face => face.weight),
    style: emittedFaces.map(face => face.style),
    fallback: primary.fallback ?? null,
    adjustFontFallback: primary.adjustFontFallback ?? null,
    declarations: primary.declarations ?? null,
    unicodeRange: emittedFaces.map(face => face.unicodeRange ?? null),
    variable: primary.variable ?? null,
  })
  const idHash = contentHash(`${hashParts.join('|')}|${family}|${optionsIdentity}`)
  const className = classNameFromHash('rari_font', idHash)
  const variableClassName =
    primary.variable != null ? classNameFromHash('rari_font_var', idHash) : null

  let overrides = null
  if (primary.adjustFontFallback !== false) {
    let fallbackName: FallbackFontName | null =
      primary.adjustFontFallback === 'Arial' || primary.adjustFontFallback === 'Times New Roman'
        ? primary.adjustFontFallback
        : null

    let metrics = await loadMetricsForFamily(family)
    if (metrics == null) {
      try {
        const unpacked = await fromBuffer(fs.readFileSync(primary.filePaths[0]))
        metrics = {
          ascent: unpacked.ascent,
          descent: unpacked.descent,
          lineGap: unpacked.lineGap,
          unitsPerEm: unpacked.unitsPerEm,
          xWidthAvg: unpacked.xWidthAvg,
        }
      } catch {
        metrics = null
      }
    }

    if (metrics != null) {
      fallbackName ??= categoryFallback(metrics.category)
      overrides = computeFallbackOverrides(metrics, fallbackName)
    }
  }

  let css = ''
  for (const face of emittedFaces) {
    css += serializeFontFaceRule(face)
  }

  if (overrides != null) {
    css +=
      `@font-face {\n` +
      `  font-family: "${family} Fallback";\n` +
      `  src: local("${overrides.fallbackFont}");\n` +
      `  ascent-override: ${overrides.ascentOverride};\n` +
      `  descent-override: ${overrides.descentOverride};\n` +
      `  line-gap-override: ${overrides.lineGapOverride};\n` +
      `  size-adjust: ${overrides.sizeAdjust};\n` +
      `}\n`
  }

  const stack = buildFontFamilyStack(family, primary.fallback, overrides != null)
  css += `.${className} {\n  font-family: ${stack};\n}\n`
  if (primary.variable != null && variableClassName != null) {
    css += `.${variableClassName} {\n  ${primary.variable}: ${stack};\n}\n`
  }

  const weightNumber = Number(primary.weight)
  const font: Font = {
    className,
    style: {
      fontFamily: stack,
      fontWeight: Number.isFinite(weightNumber) ? weightNumber : primary.weight,
      fontStyle: primary.style === 'normal' ? undefined : primary.style,
    },
    variable: variableClassName ?? undefined,
  }

  const preloadUrls = emittedFaces
    .filter(face => face.preload)
    .flatMap(face => face.src.map(entry => entry.url))
    .filter(url => url !== '')

  return {
    font,
    css,
    cssId: `${CSS_PREFIX}${idHash}.css`,
    assets,
    preloadUrls,
  }
}

function parseImportBindings(
  code: string,
  moduleId: string,
): {
  names: Array<{ imported: string; local: string }>
  defaultNames: string[]
  statements: string[]
} {
  const escaped = moduleId.replaceAll('/', '\\/')
  const mixedRe = new RegExp(
    `import\\s+(\\w+)\\s*,\\s*\\{([^}]+)\\}\\s*from\\s*['"]${escaped}['"]`,
    'g',
  )
  const namedRe = new RegExp(`import\\s*\\{([^}]+)\\}\\s*from\\s*['"]${escaped}['"]`, 'g')
  const defRe = new RegExp(`import\\s+(\\w+)\\s*from\\s*['"]${escaped}['"]`, 'g')

  const parseNamed = (body: string) =>
    body
      .split(',')
      .map(part => part.trim())
      .filter(Boolean)
      .map(part => {
        const pieces = part.split(/\s+as\s+/)
        const imported = pieces[0]?.trim() ?? ''
        const local = pieces[1]?.trim() ?? imported
        return { imported, local }
      })
      .filter(entry => entry.imported !== '' && entry.local !== '')

  const names: Array<{ imported: string; local: string }> = []
  const defaultNames: string[] = []
  const statements: string[] = []

  for (const match of code.matchAll(mixedRe)) {
    statements.push(match[0])
    defaultNames.push(match[1])
    names.push(...parseNamed(match[2]))
  }
  for (const match of code.matchAll(namedRe)) {
    if (statements.includes(match[0])) continue
    statements.push(match[0])
    names.push(...parseNamed(match[1]))
  }
  for (const match of code.matchAll(defRe)) {
    if (statements.some(statement => statement.includes(match[0]))) continue
    statements.push(match[0])
    defaultNames.push(match[1])
  }

  return {
    names: names.filter(entry => entry.imported !== ''),
    defaultNames: defaultNames.filter(name => name !== ''),
    statements,
  }
}

function hasUnsupportedFontImport(code: string, moduleId: string): boolean {
  const escaped = moduleId.replaceAll('/', '\\/')
  return (
    new RegExp(`import\\s+\\*\\s+as\\s+\\w+\\s+from\\s*['"]${escaped}['"]`).test(code) ||
    new RegExp(`import\\s*['"]${escaped}['"]`).test(code)
  )
}

function serializeFont(font: Font): string {
  const styleEntries = [
    `fontFamily: ${JSON.stringify(font.style.fontFamily)}`,
    font.style.fontWeight != null ? `fontWeight: ${JSON.stringify(font.style.fontWeight)}` : null,
    font.style.fontStyle != null ? `fontStyle: ${JSON.stringify(font.style.fontStyle)}` : null,
  ].filter(Boolean)
  const variableLine =
    font.variable != null ? `  variable: ${JSON.stringify(font.variable)},\n` : ''
  return `{\n  className: ${JSON.stringify(font.className)},\n  style: { ${styleEntries.join(', ')} },\n${variableLine}}`
}

function isIdentStart(code: number): boolean {
  return (
    (code >= 65 && code <= 90) ||
    (code >= 97 && code <= 122) ||
    code === 36 ||
    code === 95 ||
    code >= 128
  )
}

function isIdentPart(code: number): boolean {
  return isIdentStart(code) || (code >= 48 && code <= 57)
}

function skipStringOrTemplate(code: string, start: number): number {
  const quote = code.charCodeAt(start)
  let i = start + 1
  if (quote === 96) {
    while (i < code.length) {
      const ch = code.charCodeAt(i)
      if (ch === 92) {
        i += 2
        continue
      }
      if (ch === 96) return i + 1
      if (ch === 36 && code.charCodeAt(i + 1) === 123) {
        i += 2
        let depth = 1
        while (i < code.length && depth > 0) {
          const inner = code.charCodeAt(i)
          if (inner === 34 || inner === 39 || inner === 96) {
            i = skipStringOrTemplate(code, i)
            continue
          }
          if (inner === 47 && code.charCodeAt(i + 1) === 47) {
            const nl = code.indexOf('\n', i + 2)
            i = nl === -1 ? code.length : nl + 1
            continue
          }
          if (inner === 47 && code.charCodeAt(i + 1) === 42) {
            const end = code.indexOf('*/', i + 2)
            i = end === -1 ? code.length : end + 2
            continue
          }
          if (inner === 123) depth += 1
          else if (inner === 125) depth -= 1
          i += 1
        }
        continue
      }
      i += 1
    }
    return i
  }

  while (i < code.length) {
    const ch = code.charCodeAt(i)
    if (ch === 92) {
      i += 2
      continue
    }
    if (ch === quote) return i + 1
    i += 1
  }
  return i
}

function findCalls(code: string, localName: string): Array<{ start: number; openParen: number }> {
  const sites: Array<{ start: number; openParen: number }> = []
  let i = 0
  while (i < code.length) {
    const ch = code.charCodeAt(i)
    if (ch === 47 && code.charCodeAt(i + 1) === 47) {
      const nl = code.indexOf('\n', i + 2)
      i = nl === -1 ? code.length : nl + 1
      continue
    }
    if (ch === 47 && code.charCodeAt(i + 1) === 42) {
      const end = code.indexOf('*/', i + 2)
      i = end === -1 ? code.length : end + 2
      continue
    }
    if (ch === 34 || ch === 39 || ch === 96) {
      i = skipStringOrTemplate(code, i)
      continue
    }

    if (isIdentStart(ch) && code.startsWith(localName, i)) {
      const end = i + localName.length
      const prev = i === 0 ? 0 : code.charCodeAt(i - 1)
      if (
        (i === 0 || !isIdentPart(prev)) &&
        (end >= code.length || !isIdentPart(code.charCodeAt(end)))
      ) {
        let j = end
        while (
          j < code.length &&
          (code[j] === ' ' || code[j] === '\t' || code[j] === '\n' || code[j] === '\r')
        ) {
          j += 1
        }
        if (code[j] === '(') {
          sites.push({ start: i, openParen: j })
          i = j + 1
          continue
        }
      }
    }
    i += 1
  }
  return sites
}

export async function transformFontSource(
  code: string,
  id: string,
  projectRoot: string,
  assetsDir: string,
): Promise<FontTransformResult | null> {
  if (!code.includes('rari/font/')) return null

  const localImport = parseImportBindings(code, LOCAL_ID)
  const googleImport = parseImportBindings(code, GOOGLE_ID)
  if (hasUnsupportedFontImport(code, LOCAL_ID) || hasUnsupportedFontImport(code, GOOGLE_ID)) {
    throw new Error(
      'rari/font only supports default imports from `rari/font/local` and named imports from `rari/font/google` (optionally with `as`). Namespace and side-effect imports are not supported.',
    )
  }
  if (localImport.statements.length === 0 && googleImport.statements.length === 0) return null

  let nextCode = code
  const cssModules: Array<{ id: string; css: string }> = []
  const assets: Array<{ fileName: string; source: Buffer }> = []
  const preloadUrls: string[] = []
  const cssImports = new Set<string>()
  const importerDir = path.dirname(id)
  const cacheDir = path.join(projectRoot, 'node_modules', '.cache', 'rari-fonts')

  const replaceCall = async (
    localName: string,
    resolvePrepared: (optionsLiteral: Record<string, JsonValue> | null) => Promise<PreparedFont>,
  ) => {
    for (const site of findCalls(nextCode, localName).sort((a, b) => b.start - a.start)) {
      const closeParen = findClosingParen(nextCode, site.openParen)
      if (closeParen === -1) continue

      const inside = nextCode.slice(site.openParen + 1, closeParen).trim()
      let optionsLiteral: Record<string, JsonValue> | null = null
      let spliceEnd = closeParen + 1
      if (inside.startsWith('{')) {
        const openBrace = nextCode.indexOf('{', site.openParen)
        const parsed = extractObjectLiteral(nextCode, openBrace)
        if (parsed == null) continue
        optionsLiteral = parsed.value
        let end = parsed.end
        while (end < nextCode.length && /\s/.test(nextCode[end] ?? '')) end += 1
        if (nextCode[end] !== ')') continue
        spliceEnd = end + 1
      } else if (inside !== '') {
        continue
      }

      const prepared = await resolvePrepared(optionsLiteral)
      cssModules.push({ id: prepared.cssId, css: prepared.css })
      cssImports.add(prepared.cssId)
      for (const asset of prepared.assets) assets.push(asset)
      for (const url of prepared.preloadUrls) {
        if (!preloadUrls.includes(url)) preloadUrls.push(url)
      }
      nextCode =
        nextCode.slice(0, site.start) + serializeFont(prepared.font) + nextCode.slice(spliceEnd)
    }
  }

  for (const defaultName of localImport.defaultNames) {
    await replaceCall(defaultName, async optionsLiteral => {
      if (optionsLiteral == null) throw new Error('rari/font/local: options object required')
      const options = asLocalOptions(optionsLiteral)
      const faces = resolveLocalFontFaces(options, importerDir, projectRoot)
      const family = `Rari ${path.basename(faces[0].filePaths[0], path.extname(faces[0].filePaths[0]))}`
      return prepareFaces(
        faces.map(face => Object.assign({}, face, { family })),
        assetsDir,
        family,
      )
    })
  }

  for (const binding of googleImport.names) {
    await replaceCall(binding.local, async optionsLiteral => {
      const options = optionsLiteral != null ? asGoogleOptions(optionsLiteral) : {}
      const family = googleExportNameToFamily(binding.imported)
      const faces = await resolveGoogleFontFaces(family, options, cacheDir)
      return prepareFaces(faces, assetsDir, family)
    })
  }

  for (const statement of [...localImport.statements, ...googleImport.statements]) {
    nextCode = nextCode.replace(statement, '')
  }

  const cssImportBlock = [...cssImports].map(cssId => `import ${JSON.stringify(cssId)};`).join('\n')
  if (cssImportBlock !== '') nextCode = insertAfterModulePrologue(nextCode, cssImportBlock)

  if (nextCode === code) return null
  return { code: nextCode, cssModules, assets, preloadUrls }
}

function insertAfterModulePrologue(code: string, insertion: string): string {
  let i = code.charCodeAt(0) === 0xfeff ? 1 : 0
  let insertAt = 0

  for (;;) {
    while (i < code.length) {
      const c = code[i]
      if (c === ' ' || c === '\t' || c === '\r' || c === '\n') {
        i += 1
        continue
      }
      if (c === '/' && code[i + 1] === '/') {
        const nl = code.indexOf('\n', i + 2)
        i = nl === -1 ? code.length : nl + 1
        continue
      }
      if (c === '/' && code[i + 1] === '*') {
        const end = code.indexOf('*/', i + 2)
        i = end === -1 ? code.length : end + 2
        continue
      }
      break
    }

    const quote = code[i]
    if (quote !== "'" && quote !== '"') break
    if (!code.startsWith('use client', i + 1) && !code.startsWith('use server', i + 1)) break
    if (code[i + 11] !== quote) break
    i += 12
    if (code[i] === ';') i += 1
    while (i < code.length) {
      const c = code[i]
      if (c === ' ' || c === '\t' || c === '\r' || c === '\n') {
        i += 1
        continue
      }
      break
    }
    insertAt = i
  }

  if (insertAt === 0) return `${insertion}\n${code}`
  return `${code.slice(0, insertAt)}${insertion}\n${code.slice(insertAt)}`
}

function writeFontAssets(
  outDir: string,
  assets: ReadonlyArray<{ readonly fileName: string; readonly source: Buffer }>,
): void {
  for (const asset of assets) {
    const outFile = path.join(outDir, asset.fileName)
    fs.mkdirSync(path.dirname(outFile), { recursive: true })
    fs.writeFileSync(outFile, asset.source)
  }
}

export function createFontPlugin(): Plugin {
  let projectRoot = process.cwd()
  let assetsDir = 'assets'
  let outDir = path.join(projectRoot, 'dist')
  const cssModules = new Map<string, string>()
  const pendingAssets = new Map<string, Buffer>()
  const pendingPreloads = new Set<string>()

  return {
    name: 'rari:font',
    enforce: 'pre',
    configResolved(config) {
      projectRoot = config.root
      assetsDir = config.build.assetsDir || 'assets'
      outDir = path.resolve(config.root, config.build.outDir)
    },
    resolveId(id) {
      if (id.startsWith(CSS_PREFIX)) return id
      return null
    },
    load(id) {
      if (!id.startsWith(CSS_PREFIX)) return null
      return cssModules.get(id) ?? null
    },
    async transform(code, id) {
      const result = await transformFontSource(code, id, projectRoot, assetsDir)
      if (result == null) return null

      for (const entry of result.cssModules) cssModules.set(entry.id, entry.css)
      for (const asset of result.assets) pendingAssets.set(asset.fileName, asset.source)
      for (const url of result.preloadUrls) pendingPreloads.add(url)
      return { code: result.code, map: null }
    },
    transformIndexHtml(html) {
      if (pendingPreloads.size === 0) return html
      const tags = [...pendingPreloads]
        .map(url => {
          const ext = path.extname(url).toLowerCase()
          const format =
            ext === '.woff'
              ? 'woff'
              : ext === '.ttf'
                ? 'truetype'
                : ext === '.otf'
                  ? 'opentype'
                  : 'woff2'
          return `<link rel="preload" href="${url}" as="font" type="${fontMimeType(format)}" crossorigin />`
        })
        .filter(tag => !html.includes(tag))
        .join('\n')
      if (tags === '') return html
      if (html.includes('</head>')) return html.replace('</head>', `${tags}\n</head>`)
      return `${tags}\n${html}`
    },
    generateBundle() {
      for (const [fileName, source] of pendingAssets) {
        this.emitFile({ type: 'asset', fileName, source })
      }
    },
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const urlPath = req.url?.split('?')[0]
        if (urlPath == null || urlPath === '') {
          next()
          return
        }
        let relative: string
        try {
          relative = decodeURIComponent(urlPath.replace(/^\//, ''))
        } catch {
          next()
          return
        }
        const source = pendingAssets.get(relative)
        if (source != null) {
          const ext = path.extname(relative).toLowerCase()
          res.setHeader(
            'Content-Type',
            ext === '.woff'
              ? 'font/woff'
              : ext === '.ttf'
                ? 'font/ttf'
                : ext === '.otf'
                  ? 'font/otf'
                  : 'font/woff2',
          )
          res.setHeader('Cache-Control', 'no-cache')
          res.end(source)
          return
        }

        const diskPath = path.resolve(outDir, relative)
        let realOutDir: string
        try {
          realOutDir = fs.realpathSync(outDir)
        } catch {
          realOutDir = path.resolve(outDir)
        }
        let realDiskPath: string
        try {
          realDiskPath = fs.realpathSync(diskPath)
        } catch {
          next()
          return
        }
        const contained = path.relative(realOutDir, realDiskPath)
        if (contained.startsWith('..') || path.isAbsolute(contained)) {
          next()
          return
        }
        try {
          const stat = fs.statSync(realDiskPath)
          if (!stat.isFile() || !FONT_EXT_RE.test(realDiskPath)) {
            next()
            return
          }
        } catch {
          next()
          return
        }
        const ext = path.extname(realDiskPath).toLowerCase()
        res.setHeader(
          'Content-Type',
          ext === '.woff'
            ? 'font/woff'
            : ext === '.ttf'
              ? 'font/ttf'
              : ext === '.otf'
                ? 'font/otf'
                : 'font/woff2',
        )
        res.setHeader('Cache-Control', 'no-cache')
        fs.createReadStream(realDiskPath).pipe(res)
      })
    },
  }
}

export function createFontRolldownPlugin(
  projectRoot: string,
  assetsDir = 'assets',
  outDir?: string,
  cssCollector?: string[],
  preloadCollector?: string[],
) {
  const resolvedOutDir = outDir != null && outDir !== '' ? outDir : path.join(projectRoot, 'dist')
  const cssModules = new Map<string, string>()

  return {
    name: 'rari:font-rolldown',
    resolveId(id: string) {
      if (id.startsWith(CSS_PREFIX)) return id
      return null
    },
    load(id: string) {
      if (!id.startsWith(CSS_PREFIX)) return null
      const css = cssModules.get(id)
      if (css == null) return null
      if (cssCollector != null) {
        cssCollector.push(css)
        return { code: 'export {}', moduleType: 'js' as const }
      }
      return { code: css, moduleType: 'css' as const }
    },
    async transform(code: string, id: string) {
      const result = await transformFontSource(code, id, projectRoot, assetsDir)
      if (result == null) return null

      for (const entry of result.cssModules) cssModules.set(entry.id, entry.css)
      writeFontAssets(resolvedOutDir, result.assets)
      if (preloadCollector != null) {
        for (const url of result.preloadUrls) {
          const marker = fontPreloadMarker(url)
          if (!preloadCollector.includes(marker)) preloadCollector.push(marker)
        }
      }
      return { code: result.code, moduleType: 'js' as const }
    },
  }
}
