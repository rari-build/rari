import type { Plugin, ViteDevServer } from 'vite-plus'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { isFileLoadingAllowed } from 'vite-plus'
import { isPathInside, pathnameFromUrl, toPosixPath } from '@/shared/utils/path'
import { lineIdentitySourceMap } from './sourcemap'

export const FIND_SOURCE_MAP_URL_PATH = '/__rari_findSourceMapURL'

const SOURCE_FILE_EXT_RE = /\.(?:[cm]?[jt]sx?|vue|svelte|mdx?)$/i
const MAX_IDENTITY_SOURCE_BYTES = 1_048_576

function rewriteModuleSourceUrl(source: string, moduleUrl: string, base: string): string {
  if (source.startsWith('//') || /^[a-z][a-z\d+.-]*:/i.test(source)) {
    return source
  }
  if (source === '' || source === moduleUrl) return `${base}${moduleUrl}`
  if (source.startsWith('/')) return `${base}${source}`
  return `${base}${path.posix.normalize(`${path.posix.dirname(moduleUrl)}/${source}`)}`
}

function readSourceRoot(map: object): string | undefined {
  if (!('sourceRoot' in map)) return undefined
  const sourceRoot: unknown = Reflect.get(map, 'sourceRoot')
  return typeof sourceRoot === 'string' && sourceRoot !== '' ? sourceRoot : undefined
}

function applySourceRoot(source: string, sourceRoot: string | undefined): string {
  if (sourceRoot == null) return source
  if (source.startsWith('//') || /^[a-z][a-z\d+.-]*:/i.test(source) || source.startsWith('/')) {
    return source
  }
  const joined = sourceRoot.endsWith('/') ? `${sourceRoot}${source}` : `${sourceRoot}/${source}`
  return path.posix.normalize(joined)
}

function readSources(map: object): readonly (string | null)[] | undefined {
  if (!('sources' in map)) return undefined
  const sources: unknown = Reflect.get(map, 'sources')
  if (!Array.isArray(sources)) return undefined
  return sources.every(entry => entry === null || typeof entry === 'string') ? sources : undefined
}

function rewriteSourceEntry(
  source: string | null,
  moduleUrl: string,
  base: string,
  sourceRoot: string | undefined,
): string | null {
  if (source == null) return null
  return rewriteModuleSourceUrl(applySourceRoot(source, sourceRoot), moduleUrl, base)
}

export function adaptServerTransformSourceMap(
  map: object,
  moduleUrl: string,
  base: string,
): object {
  const sources = readSources(map)
  if (sources == null) return map

  const sourceRoot = readSourceRoot(map)
  const adapted: Record<string, unknown> = {
    ...map,
    sources: sources.map(source => rewriteSourceEntry(source, moduleUrl, base, sourceRoot)),
  }
  delete adapted.sourceRoot
  return adapted
}

function findSourceMapURL(
  server: ViteDevServer,
  filename: string,
  environmentName: string,
  projectRoot: string,
): object | undefined {
  if (filename.startsWith('file://')) {
    return identitySourceMapForFile(server, filename, projectRoot)
  }

  const base = server.config.base.endsWith('/')
    ? server.config.base.slice(0, -1)
    : server.config.base

  const isServer = environmentName === 'Server' || environmentName === 'rsc'
  const isClient = environmentName === 'Client' || environmentName === 'client'

  if (isServer) return serverEnvironmentSourceMap(server, filename, base)
  if (isClient) return clientEnvironmentSourceMap(server, filename, base)
  return undefined
}

function identitySourceMapForFile(
  server: ViteDevServer,
  filename: string,
  projectRoot: string,
): object | undefined {
  let filePath: string
  try {
    filePath = toPosixPath(fileURLToPath(filename))
  } catch {
    return undefined
  }
  if (!isPathInside(filePath, projectRoot)) return undefined
  if (!isFileLoadingAllowed(server.config, filePath)) return undefined
  if (!SOURCE_FILE_EXT_RE.test(filePath)) return undefined
  if (!fs.existsSync(filePath)) return undefined
  const stat = fs.statSync(filePath)
  if (!stat.isFile() || stat.size > MAX_IDENTITY_SOURCE_BYTES) return undefined
  const content = fs.readFileSync(filePath, 'utf-8')
  return lineIdentitySourceMap(filePath, content)
}

function serverEnvironmentSourceMap(
  server: ViteDevServer,
  filename: string,
  base: string,
): object | undefined {
  const rscGraph = server.environments.rsc.moduleGraph
  const mod =
    rscGraph.getModuleById(filename) ??
    (filename.startsWith('/') ? rscGraph.urlToModuleMap.get(filename) : undefined)
  const map = mod?.transformResult?.map
  if (mod == null || map == null) return undefined
  return adaptServerTransformSourceMap(map, mod.url, base)
}

function clientEnvironmentSourceMap(
  server: ViteDevServer,
  filename: string,
  base: string,
): object | undefined {
  try {
    const pathname = new URL(filename, 'http://localhost').pathname
    const url = base !== '' && pathname.startsWith(base) ? pathname.slice(base.length) : pathname
    const mod = server.environments.client.moduleGraph.urlToModuleMap.get(url)
    const map = mod?.transformResult?.map
    if (mod == null || map == null) return undefined
    return adaptServerTransformSourceMap(map, mod.url, base)
  } catch {
    return undefined
  }
}

export function createFindSourceMapURLPlugin(projectRoot: string): Plugin {
  const root = path.resolve(projectRoot)

  return {
    name: 'rari:find-source-map-url',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const pathname = pathnameFromUrl(req.url ?? '')
        if (pathname !== FIND_SOURCE_MAP_URL_PATH) {
          next()
          return
        }

        const url = new URL(req.url ?? '', 'http://localhost')
        const filename = url.searchParams.get('filename')
        const environmentName = url.searchParams.get('environmentName') ?? 'Server'
        if (filename == null || filename === '') {
          res.statusCode = 400
          res.end('{}')
          return
        }

        try {
          const map = findSourceMapURL(server, filename, environmentName, root)
          res.statusCode = map == null ? 404 : 200
          res.setHeader('Content-Type', 'application/json')
          res.setHeader('Cache-Control', 'no-store')
          res.end(JSON.stringify(map ?? {}))
        } catch (error) {
          next(error)
        }
      })
    },
  }
}
