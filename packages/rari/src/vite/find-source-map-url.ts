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

function slash(value: string): string {
  return toPosixPath(value)
}

function rewriteModuleSourceUrl(source: string, moduleUrl: string, base: string): string {
  if (source.startsWith('//') || /^[a-z][a-z\d+.-]*:/i.test(source)) {
    return source
  }
  if (source === '' || source === moduleUrl) return `${base}${moduleUrl}`
  if (source.startsWith('/')) return `${base}${source}`
  return `${base}${path.posix.normalize(`${path.posix.dirname(moduleUrl)}/${source}`)}`
}

function readSources(map: object): readonly string[] | undefined {
  if (!('sources' in map)) return undefined
  const sources: unknown = Reflect.get(map, 'sources')
  if (!Array.isArray(sources)) return undefined
  return sources.every(entry => typeof entry === 'string') ? sources : undefined
}

function findSourceMapURL(
  server: ViteDevServer,
  filename: string,
  environmentName: string,
  projectRoot: string,
): object | undefined {
  if (filename.startsWith('file://')) {
    let filePath: string
    try {
      filePath = slash(fileURLToPath(filename))
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

  const base = server.config.base.endsWith('/')
    ? server.config.base.slice(0, -1)
    : server.config.base

  const isServer = environmentName === 'Server' || environmentName === 'rsc'
  const isClient = environmentName === 'Client' || environmentName === 'client'

  if (isServer) {
    const rscGraph = server.environments.rsc.moduleGraph
    const mod =
      rscGraph.getModuleById(filename) ??
      (filename.startsWith('/') ? rscGraph.urlToModuleMap.get(filename) : undefined)
    const map = mod?.transformResult?.map
    if (mod != null && map != null) {
      const mappings =
        typeof map.mappings === 'string' && map.mappings !== '' ? `;;${map.mappings}` : map.mappings
      const sources = readSources(map)
      return {
        ...map,
        mappings,
        ...(sources != null
          ? {
              sources: sources.map(source => rewriteModuleSourceUrl(source, mod.url, base)),
            }
          : {}),
      }
    }
  }

  if (isClient) {
    try {
      const pathname = new URL(filename, 'http://localhost').pathname
      const url = base !== '' && pathname.startsWith(base) ? pathname.slice(base.length) : pathname
      const mod = server.environments.client.moduleGraph.urlToModuleMap.get(url)
      const map = mod?.transformResult?.map
      if (mod != null && map != null) {
        const sources = readSources(map)
        return {
          ...map,
          ...(sources != null
            ? {
                sources: sources.map(source => rewriteModuleSourceUrl(source, mod.url, base)),
              }
            : {}),
        }
      }
    } catch {
      // ignore invalid URL filenames
    }
  }

  return undefined
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
