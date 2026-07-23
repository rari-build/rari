import type { HmrContext, Plugin, ViteDevServer } from 'vite-plus'
import type { AppRouteManifest } from './types'
import type { RariPlugin } from '@/vite/plugin/types'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { BACKSLASH_REGEX, QUOTE_REGEX, TSX_EXT_REGEX } from '@/shared/regex-constants'
import {
  isRecord,
  isStaticParamsArray,
  parseJsonRecord,
  warnInvalidStaticParams,
} from '@/shared/utils/type-guards'
import { toRariPlugin } from '@/vite/plugin/types'
import { generateAppRouteManifest } from './routes'

const METADATA_EXPORT_REGEX = /export\s+const\s+metadata\s*(?::\s*\w+\s*)?=\s*(\{[\s\S]*?\n\})/
const TITLE_REGEX = /title\s*:\s*['"]([^'"]+)['"]/
const DESCRIPTION_REGEX = /description\s*:\s*['"]([^'"]+)['"]/
const KEYWORDS_REGEX = /keywords\s*:\s*\[([\s\S]*?)\]/

interface RariRouterPluginOptions {
  readonly appDir?: string
  readonly extensions?: readonly string[]
  readonly outDir?: string
}

const DEFAULT_OPTIONS: Required<RariRouterPluginOptions> = {
  appDir: 'src/app',
  extensions: ['.tsx', '.jsx', '.ts', '.js'],
  outDir: 'dist',
}

type AppRouterFileType =
  | 'page'
  | 'layout'
  | 'template'
  | 'loading'
  | 'error'
  | 'not-found'
  | 'route'
  | 'server-action'

interface AppRouterHMRData {
  fileType: AppRouterFileType
  filePath: string
  routePath: string
  affectedRoutes: string[]
  manifestUpdated: boolean
  timestamp: number
  metadata?: Record<string, any>
  metadataChanged?: boolean
  actionExports?: string[]
  methods?: string[]
}

function getAppRouterFileType(filePath: string): AppRouterFileType | null {
  const fileName = path.basename(filePath)
  const nameWithoutExt = fileName.replace(TSX_EXT_REGEX, '')

  switch (nameWithoutExt) {
    case 'page':
      return 'page'
    case 'layout':
      return 'layout'
    case 'template':
      return 'template'
    case 'loading':
      return 'loading'
    case 'error':
      return 'error'
    case 'not-found':
      return 'not-found'
    case 'route':
      return 'route'
    default:
      return null
  }
}

function isGroupSegment(segment: string): boolean {
  return /^\([^/]+\)$/.test(segment)
}

function stripRouteGroups(routePath: string): string {
  if (!routePath || routePath === '/') return '/'

  const segments = routePath
    .replace(BACKSLASH_REGEX, '/')
    .split('/')
    .filter(segment => Boolean(segment) && !isGroupSegment(segment))

  return segments.length > 0 ? `/${segments.join('/')}` : '/'
}

function filePathToRoutePath(filePath: string, appDir: string): string {
  const relativePath = path.relative(appDir, path.dirname(filePath))

  if (!relativePath || relativePath === '.') return '/'

  const normalized = relativePath.replace(BACKSLASH_REGEX, '/')
  const segments = normalized.split('/').filter(Boolean)

  return stripRouteGroups(`/${segments.join('/')}`)
}

function getAffectedRoutes(
  routePath: string,
  fileType: AppRouterFileType,
  allRoutes: readonly string[],
): string[] {
  if (fileType === 'page') return [routePath]

  const prefix = `${routePath}${routePath !== '/' ? '/' : ''}`
  const affected = allRoutes.filter(route => {
    return route === routePath || route.startsWith(prefix)
  })

  return affected.length > 0 ? affected : [routePath]
}

function extractMetadata(fileContent: string): Record<string, any> | null {
  try {
    const match = METADATA_EXPORT_REGEX.exec(fileContent)

    if (!match) return null

    const metadataString = match[1]

    const metadata: Record<string, any> = {}

    const titleMatch = TITLE_REGEX.exec(metadataString)
    if (titleMatch) metadata.title = titleMatch[1]

    const descMatch = DESCRIPTION_REGEX.exec(metadataString)
    if (descMatch) metadata.description = descMatch[1]

    const keywordsMatch = KEYWORDS_REGEX.exec(metadataString)
    if (keywordsMatch) {
      const keywordsStr = keywordsMatch[1]
      const keywords = keywordsStr
        .split(',')
        .map(k => k.trim().replace(QUOTE_REGEX, ''))
        .filter(Boolean)
      metadata.keywords = keywords
    }

    const fieldsToExtract = ['author', 'viewport', 'themeColor', 'robots', 'openGraph', 'twitter']
    for (const field of fieldsToExtract) {
      const fieldRegex = new RegExp(`${field}\\s*:\\s*['"]([^'"]+)['"]`, 'm')
      const fieldMatch = metadataString.match(fieldRegex)
      if (fieldMatch) metadata[field] = fieldMatch[1]
    }

    return Object.keys(metadata).length > 0 ? metadata : null
  } catch (error) {
    console.error('[rari] Router: Failed to extract metadata:', error)
    return null
  }
}

function detectHttpMethods(fileContent: string): string[] {
  const methods: string[] = []
  const httpMethods = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS']

  for (const method of httpMethods) {
    const functionExportRegex = new RegExp(`export\\s+(?:async\\s+)?function\\s+${method}\\s*\\(`)
    const constExportRegex = new RegExp(
      `export\\s+(?:async\\s+)?(?:const|let|var)\\s+${method}\\s*=`,
    )

    if (functionExportRegex.test(fileContent) || constExportRegex.test(fileContent))
      methods.push(method)
  }

  return methods
}

function isAppRouteManifest(value: unknown): value is AppRouteManifest {
  return isRecord(value) && Array.isArray(value.routes)
}

async function notifyApiRouteInvalidation(filePath: string): Promise<void> {
  try {
    const response = await fetch('http://localhost:3000/_rari/hmr', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        action: 'invalidate-api-route',
        filePath,
      }),
    })

    if (!response.ok) {
      console.error(`[rari] Router: Failed to invalidate API route cache: ${response.statusText}`)
      return
    }

    const result = parseJsonRecord(await response.text())
    if (result?.success !== true) {
      const error =
        typeof result?.error === 'string' && result.error !== '' ? result.error : 'Unknown error'
      console.error(`[rari] HMR: Failed to invalidate API route cache: ${error}`)
    }
  } catch (error) {
    console.error('[rari] Router: Failed to notify API route invalidation:', error)
  }
}

export function rariRouter(options: RariRouterPluginOptions = {}): RariPlugin {
  const opts = { ...DEFAULT_OPTIONS, ...options }

  let cachedManifestContent: string | null = null

  const pendingHMRUpdates = new Map<string, NodeJS.Timeout>()
  const DEBOUNCE_DELAY = 200

  let routeStructureHash: string | null = null
  const routeFiles = new Set<string>()

  const computeRouteStructureHash = (files: Set<string>): string => {
    const sortedFiles = [...files].toSorted()
    return sortedFiles.join('|')
  }

  const scanRouteFiles = async (appDir: string): Promise<Set<string>> => {
    const files = new Set<string>()

    const scanDir = async (dir: string): Promise<void> => {
      try {
        const entries = await fs.readdir(dir, { withFileTypes: true })

        for (const entry of entries) {
          const fullPath = path.join(dir, entry.name)

          if (entry.isDirectory()) {
            await scanDir(fullPath)
          } else if (entry.isFile() && opts.extensions.some(ext => entry.name.endsWith(ext))) {
            const fileType = getAppRouterFileType(fullPath)
            if (fileType) files.add(fullPath)
          }
        }
      } catch {}
    }

    await scanDir(appDir)
    return files
  }

  const generateAppRoutes = async (
    root: string,
    forceRegenerate: boolean = false,
  ): Promise<string | null> => {
    const appDir = path.resolve(root, opts.appDir)

    try {
      await fs.access(appDir)
    } catch {
      return null
    }

    try {
      const currentRouteFiles = await scanRouteFiles(appDir)
      const currentHash = computeRouteStructureHash(currentRouteFiles)

      if (
        !forceRegenerate &&
        routeStructureHash === currentHash &&
        cachedManifestContent != null &&
        cachedManifestContent !== ''
      )
        return cachedManifestContent

      const manifest = await generateAppRouteManifest(appDir, {
        extensions: opts.extensions,
      })

      const manifestContent = JSON.stringify(manifest)

      const outDir = path.resolve(root, opts.outDir)
      await fs.mkdir(outDir, { recursive: true })
      const serverDir = path.join(outDir, 'server')
      await fs.mkdir(serverDir, { recursive: true })
      await fs.writeFile(path.join(serverDir, 'routes.json'), manifestContent, 'utf-8')

      routeStructureHash = currentHash
      routeFiles.clear()
      currentRouteFiles.forEach(file => {
        routeFiles.add(file)
      })

      return manifestContent
    } catch (error) {
      console.error('[rari] Router: Failed to generate app routes:', error)
      return null
    }
  }

  const setupWatcher = (devServer: ViteDevServer): void => {
    const appDir = path.resolve(devServer.config.root, opts.appDir)

    devServer.watcher.on('all', (event: string, filePath: string) => {
      void (async () => {
        if (!filePath.startsWith(appDir)) return

        if (opts.extensions.some(ext => filePath.endsWith(ext))) {
          try {
            const fileType = getAppRouterFileType(filePath)
            const isRouteFile = fileType !== null
            const isAddOrUnlink = event === 'add' || event === 'unlink'
            const isNewRouteFile = isRouteFile && !routeFiles.has(filePath)

            if (isAddOrUnlink || isNewRouteFile) {
              await generateAppRoutes(devServer.config.root, true)

              if (filePath.includes(opts.appDir)) {
                devServer.ws.send({
                  type: 'full-reload',
                  path: '*',
                })
              }
            }
          } catch (error) {
            console.error('[rari] Router: Failed to regenerate app routes:', error)
          }
        }
      })()
    })
  }

  let viteRoot: string

  const plugin = {
    name: 'rari-router',

    configResolved(config) {
      viteRoot = config.root

      // Suppress Vite warnings about dynamic imports in our dist files
      // These are intentional and use @vite-ignore comments that get stripped by minification
      const originalWarn = config.logger.warn.bind(config.logger)
      config.logger.warn = (msg, options) => {
        if (
          typeof msg === 'string' &&
          msg.includes('The above dynamic import cannot be analyzed') &&
          msg.includes('packages/rari/dist/')
        )
          return

        originalWarn(msg, options)
      }
    },

    async writeBundle() {
      const root = viteRoot || process.cwd()
      cachedManifestContent = await generateAppRoutes(root, true)
    },

    configureServer(devServer: ViteDevServer) {
      setupWatcher(devServer)
    },

    async handleHotUpdate(ctx: HmrContext) {
      const { file, server } = ctx

      const appDir = path.resolve(server.config.root, opts.appDir)

      const isAppFile = file.startsWith(appDir) && opts.extensions.some(ext => file.endsWith(ext))

      if (isAppFile) {
        const fileType = getAppRouterFileType(file)

        if (fileType) {
          const existingTimer = pendingHMRUpdates.get(file)
          if (existingTimer) clearTimeout(existingTimer)

          const timer = setTimeout(() => {
            void (async () => {
              pendingHMRUpdates.delete(file)

              const isNewRouteFile = !routeFiles.has(file)
              const previousManifest = cachedManifestContent
              cachedManifestContent = await generateAppRoutes(server.config.root, isNewRouteFile)
              const manifestUpdated = previousManifest !== cachedManifestContent
              const routePath = filePathToRoutePath(file, appDir)

              let allRoutes: string[] = [routePath]
              if (cachedManifestContent != null && cachedManifestContent !== '') {
                try {
                  const manifestRecord = parseJsonRecord(cachedManifestContent)
                  if (manifestRecord && isAppRouteManifest(manifestRecord))
                    allRoutes = manifestRecord.routes.map(r => r.path)
                } catch (error) {
                  console.error(
                    '[rari] Router: Failed to parse manifest for affected routes:',
                    error,
                  )
                }
              }

              const affectedRoutes = getAffectedRoutes(routePath, fileType, allRoutes)

              let metadata: Record<string, any> | undefined
              let metadataChanged = false
              let methods: string[] | undefined

              if (fileType === 'page' || fileType === 'layout') {
                try {
                  const fileContent = await fs.readFile(file, 'utf-8')
                  const extractedMetadata = extractMetadata(fileContent)

                  if (extractedMetadata) {
                    metadata = extractedMetadata
                    metadataChanged = true
                  }
                } catch (error) {
                  console.error('[rari] Router: Failed to extract metadata:', error)
                }
              }

              if (fileType === 'route') {
                try {
                  const fileContent = await fs.readFile(file, 'utf-8')
                  methods = detectHttpMethods(fileContent)

                  await notifyApiRouteInvalidation(path.relative(appDir, file))
                } catch (error) {
                  console.error('[rari] Router: Failed to detect HTTP methods:', error)
                }
              }

              const hmrData: AppRouterHMRData = {
                fileType,
                filePath: path.relative(server.config.root, file),
                routePath,
                affectedRoutes,
                manifestUpdated,
                timestamp: Date.now(),
                metadata,
                metadataChanged,
                methods,
              }

              server.ws.send({
                type: 'custom',
                event: 'rari:app-router-updated',
                data: hmrData,
              })
            })()
          }, DEBOUNCE_DELAY)

          pendingHMRUpdates.set(file, timer)

          return []
        }

        cachedManifestContent = await generateAppRoutes(server.config.root)
        return []
      }

      return undefined
    },

    async closeBundle() {
      for (const timer of pendingHMRUpdates.values()) clearTimeout(timer)

      pendingHMRUpdates.clear()

      const root = viteRoot || process.cwd()
      const serverDir = path.resolve(root, opts.outDir, 'server')
      const routesPath = path.join(serverDir, 'routes.json')

      try {
        const content = await fs.readFile(routesPath, 'utf-8')
        const manifestRecord = parseJsonRecord(content)
        if (!manifestRecord || !isAppRouteManifest(manifestRecord)) return

        const manifest = manifestRecord
        let updated = false

        for (const route of manifest.routes) {
          if (!route.isDynamic) continue

          const componentId = route.componentId
          if (componentId == null || componentId === '') continue

          const compiledPath = path.join(serverDir, `${componentId}.js`)

          try {
            const module: unknown = await import(/* @vite-ignore */ compiledPath)
            if (isRecord(module) && typeof module.generateStaticParams === 'function') {
              // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- dynamically imported route module
              const generateStaticParams = module.generateStaticParams as () => unknown
              const params = await generateStaticParams()
              if (isStaticParamsArray(params)) {
                if (params.length > 0) {
                  route.staticParams = params
                  updated = true
                }
              } else {
                warnInvalidStaticParams(componentId)
              }
            }
          } catch (error) {
            console.warn(
              `[rari] Failed to evaluate generateStaticParams for ${componentId}:`,
              error,
            )
          }
        }

        if (updated) await fs.writeFile(routesPath, JSON.stringify(manifest), 'utf-8')
      } catch (error) {
        console.warn('[rari] Failed to enrich routes manifest with static params:', error)
      }
    },
  } satisfies Plugin

  return toRariPlugin(plugin)
}
