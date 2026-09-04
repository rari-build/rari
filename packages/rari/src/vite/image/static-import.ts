import type { Plugin } from 'vite-plus'
import { createHash } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { resolveAlias } from '@/shared/utils/alias-resolver'
import { parseJsonRecord } from '@/shared/utils/type-guards'
import { readImageDimensions } from './dimensions'

const IMAGE_EXT_RE = /\.(?:avif|gif|jpe?g|png|webp)$/i
const QUERY_RE = /\?.*$/
const VIRTUAL_PREFIX = '\0rari-static-image:'
const BYPASS_QUERY = 'rari-static-bypass'
const VITE_NATIVE_IMAGE_QUERIES = new Set(['raw', 'url', 'inline', 'no-inline'])
const VIRTUAL_IMPORTER_PREFIXES = ['\0ssr-virtual:', '\0virtual:'] as const

export type StaticImageSourceMap = Record<string, string>

function stripQuery(id: string): string {
  return id.replace(QUERY_RE, '')
}

function queryParams(id: string): string[] {
  const queryIndex = id.indexOf('?')
  if (queryIndex === -1) return []
  return id
    .slice(queryIndex + 1)
    .split('&')
    .map(part => part.split('=')[0] ?? '')
    .filter(Boolean)
}

function hasBypassQuery(id: string): boolean {
  return queryParams(id).includes(BYPASS_QUERY)
}

function hasViteNativeImageQuery(id: string): boolean {
  return queryParams(id).some(param => VITE_NATIVE_IMAGE_QUERIES.has(param))
}

function contentHash(buffer: Buffer): string {
  return createHash('sha256').update(buffer).digest('hex').slice(0, 8)
}

function publicAssetPath(filePath: string, hash: string, assetsDir: string): string {
  const ext = path.extname(filePath)
  const base = path.basename(filePath, ext)
  const normalizedAssetsDir = assetsDir.replace(/^\/+|\/+$/g, '')
  return `/${normalizedAssetsDir}/${encodeURIComponent(base)}-${hash}${ext.toLowerCase()}`
}

function assetFileName(filePath: string, hash: string, assetsDir: string): string {
  const ext = path.extname(filePath)
  const base = path.basename(filePath, ext)
  const normalizedAssetsDir = assetsDir.replace(/^\/+|\/+$/g, '')
  return `${normalizedAssetsDir}/${base}-${hash}${ext.toLowerCase()}`
}

function generateModuleSource(publicPath: string, width: number, height: number): string {
  return (
    `const src = ${JSON.stringify(publicPath)};\n` +
    `export default {\n` +
    `  src,\n` +
    `  width: ${width},\n` +
    `  height: ${height},\n` +
    `};\n`
  )
}

export function normalizeStaticImageImporter(importer: string): string {
  let normalized = importer
  for (const prefix of VIRTUAL_IMPORTER_PREFIXES) {
    if (normalized.startsWith(prefix)) {
      normalized = normalized.slice(prefix.length)
      break
    }
  }
  return normalized
}

export function isStaticImageModuleId(id: string): boolean {
  return IMAGE_EXT_RE.test(stripQuery(id)) && !hasBypassQuery(id) && !hasViteNativeImageQuery(id)
}

export function resolveStaticImageFilePath(
  id: string,
  importer: string | undefined,
  projectRoot: string,
  aliases: Readonly<Record<string, string>> = {},
): string | null {
  if (importer == null || importer === '' || importer.includes('node_modules')) return null
  if (!isStaticImageModuleId(id)) return null
  if (id.startsWith('\0')) return null

  const importerPath = normalizeStaticImageImporter(importer)
  let resolved = id
  if (id.startsWith('.')) resolved = path.resolve(path.dirname(importerPath), id)
  else if (path.isAbsolute(id)) resolved = id
  else {
    const aliased = resolveAlias(id, aliases, projectRoot)
    if (aliased == null || aliased === '') return null
    resolved = aliased
  }

  const filePath = stripQuery(resolved)
  if (!fs.existsSync(filePath)) return null
  return filePath
}

export function buildStaticImageModule(
  filePath: string,
  assetsDir = 'assets',
): {
  readonly code: string
  readonly publicPath: string
  readonly source: Buffer
  readonly fileName: string
} {
  const source = fs.readFileSync(filePath)
  const hash = contentHash(source)
  const publicPath = publicAssetPath(filePath, hash, assetsDir)
  const fileName = assetFileName(filePath, hash, assetsDir)
  const dimensions = readImageDimensions(source)
  if (dimensions == null) {
    throw new Error(
      `[rari] Unable to read dimensions for static image import: ${filePath}. ` +
        `Pass explicit width/height to <Image>, or convert the file to png/jpeg/webp/gif/avif.`,
    )
  }

  return {
    code: generateModuleSource(publicPath, dimensions.width, dimensions.height),
    publicPath,
    source,
    fileName,
  }
}

const writeSourceMapsByOutDir = new Map<string, Map<string, string>>()
const buildEntriesByOutDir = new Map<string, Map<string, string>>()

export function resolveStaticImageOutDir(projectRoot: string, outDir?: string): string {
  const candidate = outDir != null && outDir !== '' ? outDir : path.join(projectRoot, 'dist')
  return path.isAbsolute(candidate) ? candidate : path.resolve(projectRoot, candidate)
}

function getSharedSourceMap(outDir: string, mapPath: string): Map<string, string> {
  const cached = writeSourceMapsByOutDir.get(outDir)
  if (cached != null) return cached

  const shared = new Map<string, string>()
  if (fs.existsSync(mapPath)) {
    try {
      const parsed = parseJsonRecord(fs.readFileSync(mapPath, 'utf8'))
      if (parsed != null) {
        for (const [key, value] of Object.entries(parsed)) {
          if (typeof value === 'string') shared.set(key, value)
        }
      }
    } catch {
      // Start fresh if the on-disk map is unreadable.
    }
  }

  writeSourceMapsByOutDir.set(outDir, shared)
  return shared
}

function setSourceMapEntry(map: Map<string, string>, publicPath: string, sourcePath: string): void {
  for (const [existingPublicPath, existingSourcePath] of map) {
    if (existingSourcePath === sourcePath && existingPublicPath !== publicPath) {
      map.delete(existingPublicPath)
    }
  }
  map.set(publicPath, sourcePath)
}

function pruneSharedSourceMap(
  shared: Map<string, string>,
  entries: ReadonlyMap<string, string>,
): void {
  const updatedSources = new Set(entries.values())

  for (const [publicPath, sourcePath] of shared) {
    if (!fs.existsSync(sourcePath)) {
      shared.delete(publicPath)
      continue
    }

    if (updatedSources.has(sourcePath) && entries.get(publicPath) !== sourcePath) {
      shared.delete(publicPath)
    }
  }
}

function sourceMapFilePath(outDir: string): string {
  return path.join(outDir, 'server', 'static-image-sources.json')
}

function persistSharedSourceMap(outDir: string, shared: Map<string, string>): void {
  const mapPath = sourceMapFilePath(outDir)
  fs.mkdirSync(path.dirname(mapPath), { recursive: true })
  const next: StaticImageSourceMap = Object.fromEntries(shared)
  fs.writeFileSync(mapPath, `${JSON.stringify(next, null, 2)}\n`)
}

export function beginStaticImageSourceMapBuild(outDir: string): void {
  if (buildEntriesByOutDir.has(outDir)) return
  buildEntriesByOutDir.set(outDir, new Map())
}

export function finalizeStaticImageSourceMapBuild(outDir: string): void {
  const seen = buildEntriesByOutDir.get(outDir)
  if (seen == null) return

  const mapPath = sourceMapFilePath(outDir)
  const shared = getSharedSourceMap(outDir, mapPath)
  shared.clear()
  for (const [publicPath, sourcePath] of seen) {
    if (fs.existsSync(sourcePath)) shared.set(publicPath, sourcePath)
  }
  persistSharedSourceMap(outDir, shared)
  buildEntriesByOutDir.delete(outDir)
}

function writeSourceMap(outDir: string, entries: ReadonlyMap<string, string>): void {
  const mapPath = sourceMapFilePath(outDir)
  const shared = getSharedSourceMap(outDir, mapPath)

  for (const [publicPath, sourcePath] of entries) shared.set(publicPath, sourcePath)

  const buildEntries = buildEntriesByOutDir.get(outDir)
  if (buildEntries != null) {
    for (const [publicPath, sourcePath] of entries) {
      setSourceMapEntry(buildEntries, publicPath, sourcePath)
    }
    return
  }

  pruneSharedSourceMap(shared, entries)
  persistSharedSourceMap(outDir, shared)
}

function forgetSourcePath(map: Map<string, string>, sourcePath: string): boolean {
  let removed = false
  for (const [publicPath, mappedPath] of map) {
    if (mappedPath === sourcePath) {
      map.delete(publicPath)
      removed = true
    }
  }
  return removed
}

export function createStaticImagePlugin(): Plugin {
  let projectRoot = process.cwd()
  let outDir = path.join(projectRoot, 'dist')
  let assetsDir = 'assets'
  let isBuildCommand = false
  const sourceByPublicPath = new Map<string, string>()

  return {
    name: 'rari:static-image',
    enforce: 'pre',
    configResolved(config) {
      projectRoot = config.root
      outDir = path.resolve(config.root, config.build.outDir)
      assetsDir = config.build.assetsDir || 'assets'
      isBuildCommand = config.command === 'build'
    },
    buildStart() {
      if (!isBuildCommand) return
      beginStaticImageSourceMapBuild(outDir)
    },
    async resolveId(id, importer) {
      if (importer == null || importer === '') return null
      if (!isStaticImageModuleId(id)) return null

      const resolved = await this.resolve(id, importer, { skipSelf: true })
      if (resolved == null || resolved.id === '') return null

      const filePath = stripQuery(resolved.id)
      if (!fs.existsSync(filePath)) return null
      return VIRTUAL_PREFIX + filePath
    },
    load(id) {
      if (!id.startsWith(VIRTUAL_PREFIX)) return null

      const filePath = id.slice(VIRTUAL_PREFIX.length)
      const built = buildStaticImageModule(filePath, assetsDir)
      setSourceMapEntry(sourceByPublicPath, built.publicPath, filePath)
      writeSourceMap(outDir, sourceByPublicPath)

      this.emitFile({
        type: 'asset',
        fileName: built.fileName,
        source: built.source,
      })

      return built.code
    },
    watchChange(id, change) {
      if (change.event !== 'delete') return
      const filePath = stripQuery(id)
      if (!forgetSourcePath(sourceByPublicPath, filePath)) return
      const buildEntries = buildEntriesByOutDir.get(outDir)
      if (buildEntries != null) forgetSourcePath(buildEntries, filePath)
      writeSourceMap(outDir, sourceByPublicPath)
    },
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const urlPath = req.url?.split('?')[0]
        if (urlPath == null || urlPath === '') {
          next()
          return
        }

        let decodedPath = urlPath
        try {
          decodedPath = decodeURIComponent(urlPath)
        } catch {
          decodedPath = urlPath
        }

        const sourcePath = sourceByPublicPath.get(urlPath) ?? sourceByPublicPath.get(decodedPath)
        if (sourcePath == null || sourcePath === '') {
          next()
          return
        }

        const ext = path.extname(sourcePath).toLowerCase()
        const contentType =
          ext === '.png'
            ? 'image/png'
            : ext === '.gif'
              ? 'image/gif'
              : ext === '.webp'
                ? 'image/webp'
                : ext === '.avif'
                  ? 'image/avif'
                  : ext === '.svg'
                    ? 'image/svg+xml'
                    : 'image/jpeg'

        res.setHeader('Content-Type', contentType)
        res.setHeader('Cache-Control', 'no-cache')
        fs.createReadStream(sourcePath)
          .on('error', () => {
            if (!res.headersSent) res.statusCode = 404
            res.end()
          })
          .pipe(res)
      })
    },
  }
}

export function createStaticImageRolldownPlugin(
  projectRoot: string,
  assetsDir = 'assets',
  aliases: Readonly<Record<string, string>> = {},
  outDir?: string,
) {
  const resolvedOutDir = resolveStaticImageOutDir(projectRoot, outDir)
  const sourceByPublicPath = new Map<string, string>()

  return {
    name: 'rari:static-image-rolldown',
    resolveId(id: string, importer: string | undefined) {
      const filePath = resolveStaticImageFilePath(id, importer, projectRoot, aliases)
      if (filePath == null) return null
      return VIRTUAL_PREFIX + filePath
    },
    load(id: string) {
      if (!id.startsWith(VIRTUAL_PREFIX)) return null

      const filePath = id.slice(VIRTUAL_PREFIX.length)
      const built = buildStaticImageModule(filePath, assetsDir)
      setSourceMapEntry(sourceByPublicPath, built.publicPath, filePath)
      writeSourceMap(resolvedOutDir, sourceByPublicPath)

      const outFile = path.join(resolvedOutDir, built.fileName)
      fs.mkdirSync(path.dirname(outFile), { recursive: true })
      fs.writeFileSync(outFile, built.source)

      return { code: built.code, moduleType: 'js' as const }
    },
  }
}
