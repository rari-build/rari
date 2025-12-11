import type { FSWatcher } from 'chokidar'
import type { Plugin, ViteDevServer } from 'rolldown-vite'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { watch } from 'chokidar'

interface RariRouterPluginOptions {
  appDir?: string
  extensions?: string[]
  outDir?: string
}

const DEFAULT_OPTIONS: Required<RariRouterPluginOptions> = {
  appDir: 'src/app',
  extensions: ['.tsx', '.jsx', '.ts', '.js'],
  outDir: 'dist',
}

type AppRouterFileType = 'page' | 'layout' | 'loading' | 'error' | 'not-found' | 'route' | 'server-action'

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
  const nameWithoutExt = fileName.replace(/\.(tsx?|jsx?)$/, '')

  switch (nameWithoutExt) {
    case 'page':
      return 'page'
    case 'layout':
      return 'layout'
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

function filePathToRoutePath(filePath: string, appDir: string): string {
  const relativePath = path.relative(appDir, path.dirname(filePath))

  if (!relativePath || relativePath === '.') {
    return '/'
  }

  const normalized = relativePath.replace(/\\/g, '/')
  const segments = normalized.split('/').filter(Boolean)

  return `/${segments.join('/')}`
}

function getAffectedRoutes(
  routePath: string,
  fileType: AppRouterFileType,
  allRoutes: string[],
): string[] {
  if (fileType === 'page') {
    return [routePath]
  }

  const affected = allRoutes.filter((route) => {
    return route === routePath || route.startsWith(`${routePath}/`)
  })

  return affected.length > 0 ? affected : [routePath]
}

function extractMetadata(fileContent: string): Record<string, any> | null {
  try {
    const metadataRegex = /export\s+const\s+metadata\s*(?::\s*\w+\s*)?=\s*(\{[\s\S]*?\n\})/
    const match = fileContent.match(metadataRegex)

    if (!match) {
      return null
    }

    const metadataString = match[1]

    const metadata: Record<string, any> = {}

    const titleMatch = metadataString.match(/title\s*:\s*['"]([^'"]+)['"]/)
    if (titleMatch) {
      metadata.title = titleMatch[1]
    }

    const descMatch = metadataString.match(/description\s*:\s*['"]([^'"]+)['"]/)
    if (descMatch) {
      metadata.description = descMatch[1]
    }

    const keywordsMatch = metadataString.match(/keywords\s*:\s*\[([\s\S]*?)\]/)
    if (keywordsMatch) {
      const keywordsStr = keywordsMatch[1]
      const keywords = keywordsStr
        .split(',')
        .map(k => k.trim().replace(/['"]/g, ''))
        .filter(Boolean)
      metadata.keywords = keywords
    }

    const fieldsToExtract = ['author', 'viewport', 'themeColor', 'robots', 'openGraph', 'twitter']
    for (const field of fieldsToExtract) {
      const fieldRegex = new RegExp(`${field}\\s*:\\s*['"]([^'"]+)['"]`, 'm')
      const fieldMatch = metadataString.match(fieldRegex)
      if (fieldMatch) {
        metadata[field] = fieldMatch[1]
      }
    }

    return Object.keys(metadata).length > 0 ? metadata : null
  }
  catch (error) {
    console.error('Failed to extract metadata:', error)
    return null
  }
}

function detectHttpMethods(fileContent: string): string[] {
  const methods: string[] = []
  const httpMethods = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS']

  for (const method of httpMethods) {
    const functionExportRegex = new RegExp(
      `export\\s+(?:async\\s+)?function\\s+${method}\\s*\\(`,
    )
    const constExportRegex = new RegExp(
      `export\\s+(?:async\\s+)?(?:const|let|var)\\s+${method}\\s*=`,
    )

    if (functionExportRegex.test(fileContent) || constExportRegex.test(fileContent)) {
      methods.push(method)
    }
  }

  return methods
}

async function notifyApiRouteInvalidation(filePath: string): Promise<void> {
  try {
    const response = await fetch('http://localhost:3000/api/rsc/hmr-invalidate-api-route', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        filePath,
      }),
    })

    if (!response.ok) {
      console.error(`Failed to invalidate API route cache: ${response.statusText}`)
      return
    }

    const result = await response.json()
    if (result.success) {
      console.warn(`[HMR] API route handler cache invalidated: ${filePath}`)
    }
    else {
      console.error(`[HMR] Failed to invalidate API route cache: ${result.error || 'Unknown error'}`)
    }
  }
  catch (error) {
    console.error('Failed to notify API route invalidation:', error)
  }
}

export function rariRouter(options: RariRouterPluginOptions = {}): Plugin {
  const opts = { ...DEFAULT_OPTIONS, ...options }

  let server: ViteDevServer | undefined
  let watcher: FSWatcher | undefined
  let cachedManifestContent: string | null = null

  const pendingHMRUpdates = new Map<string, NodeJS.Timeout>()
  const DEBOUNCE_DELAY = 200

  let routeStructureHash: string | null = null
  const routeFiles = new Set<string>()

  const computeRouteStructureHash = (files: Set<string>): string => {
    const sortedFiles = Array.from(files).sort()
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
          }
          else if (entry.isFile() && opts.extensions.some(ext => entry.name.endsWith(ext))) {
            const fileType = getAppRouterFileType(fullPath)
            if (fileType) {
              files.add(fullPath)
            }
          }
        }
      }
      catch {
        // Directory might not exist or be accessible
      }
    }

    await scanDir(appDir)
    return files
  }

  const generateAppRoutes = async (root: string, forceRegenerate: boolean = false): Promise<string | null> => {
    const appDir = path.resolve(root, opts.appDir)

    try {
      await fs.access(appDir)
    }
    catch {
      return null
    }

    try {
      const currentRouteFiles = await scanRouteFiles(appDir)
      const currentHash = computeRouteStructureHash(currentRouteFiles)

      if (!forceRegenerate && routeStructureHash === currentHash && cachedManifestContent) {
        console.warn('[Manifest] Route structure unchanged, using cached manifest')
        return cachedManifestContent
      }

      const { generateAppRouteManifest } = await import('./app-routes')

      const manifest = await generateAppRouteManifest(appDir, {
        extensions: opts.extensions,
      })

      const manifestContent = JSON.stringify(manifest, null, 2)

      const outDir = path.resolve(root, opts.outDir)
      await fs.mkdir(outDir, { recursive: true })
      await fs.writeFile(path.join(outDir, 'app-routes.json'), manifestContent, 'utf-8')

      const { generateLoadingComponentMap, getLoadingComponentMapPath } = await import('./loading-component-map')
      const loadingMapCode = generateLoadingComponentMap({
        appDir: opts.appDir,
        loadingComponents: manifest.loading,
      })
      const loadingMapPath = getLoadingComponentMapPath(outDir)
      await fs.writeFile(loadingMapPath, loadingMapCode, 'utf-8')

      routeStructureHash = currentHash
      routeFiles.clear()
      currentRouteFiles.forEach(file => routeFiles.add(file))

      return manifestContent
    }
    catch (error) {
      console.error('Failed to generate app routes:', error)
      return null
    }
  }

  const setupWatcher = (root: string): void => {
    if (watcher) {
      watcher.close()
    }

    const watchPaths = [path.resolve(root, opts.appDir)]

    watcher = watch(watchPaths, {
      ignored: /node_modules/,
      persistent: true,
      ignoreInitial: true,
    })

    watcher.on('all', async (event: string, filePath: string) => {
      if (opts.extensions.some(ext => filePath.endsWith(ext))) {
        try {
          const fileType = getAppRouterFileType(filePath)

          const isRouteFile = fileType !== null
          const isAddOrUnlink = event === 'add' || event === 'unlink'
          const isNewRouteFile = isRouteFile && !routeFiles.has(filePath)

          if (isAddOrUnlink || isNewRouteFile) {
            console.warn(`[Manifest] Route structure changed (${event}: ${path.relative(root, filePath)}), regenerating manifest`)
            await generateAppRoutes(root, true)

            if (server && filePath.includes(opts.appDir)) {
              server.ws.send({
                type: 'full-reload',
                path: '*',
              })
            }
          }
        }
        catch (error) {
          console.error('Failed to regenerate app routes:', error)
        }
      }
    })
  }

  return {
    name: 'rari-router',

    async buildStart() {
      cachedManifestContent = await generateAppRoutes(process.cwd())
    },

    configureServer(devServer: ViteDevServer) {
      server = devServer

      setupWatcher(devServer.config.root)
    },

    async handleHotUpdate(ctx: any) {
      const { file, server } = ctx

      const appDir = path.resolve(server.config.root, opts.appDir)

      const isAppFile
        = file.startsWith(appDir)
          && opts.extensions.some(ext => file.endsWith(ext))

      if (isAppFile) {
        const fileType = getAppRouterFileType(file)

        if (fileType) {
          const existingTimer = pendingHMRUpdates.get(file)
          if (existingTimer) {
            clearTimeout(existingTimer)
          }

          const timer = setTimeout(async () => {
            pendingHMRUpdates.delete(file)

            const isNewRouteFile = !routeFiles.has(file)

            const previousManifest = cachedManifestContent
            cachedManifestContent = await generateAppRoutes(server.config.root, isNewRouteFile)
            const manifestUpdated = previousManifest !== cachedManifestContent

            const routePath = filePathToRoutePath(file, appDir)

            let allRoutes: string[] = [routePath]
            if (cachedManifestContent) {
              try {
                const manifest = JSON.parse(cachedManifestContent)
                allRoutes = manifest.routes.map((r: any) => r.path)
              }
              catch (error) {
                console.error('Failed to parse manifest for affected routes:', error)
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
                  console.warn(`[HMR] Metadata detected in ${fileType}: ${JSON.stringify(metadata)}`)
                }
              }
              catch (error) {
                console.error('Failed to extract metadata:', error)
              }
            }

            if (fileType === 'route') {
              try {
                const fileContent = await fs.readFile(file, 'utf-8')
                methods = detectHttpMethods(fileContent)
                console.warn(`[HMR] API route methods detected: ${methods.join(', ')}`)

                await notifyApiRouteInvalidation(path.relative(appDir, file))
              }
              catch (error) {
                console.error('Failed to detect HTTP methods:', error)
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

            const metadataInfo = metadataChanged ? ' [metadata updated]' : ''
            const methodsInfo = methods ? ` [methods: ${methods.join(', ')}]` : ''
            console.warn(
              `[HMR] App router ${fileType} changed: ${hmrData.filePath} (affects ${affectedRoutes.length} route${affectedRoutes.length === 1 ? '' : 's'})${metadataInfo}${methodsInfo}`,
            )
          }, DEBOUNCE_DELAY)

          pendingHMRUpdates.set(file, timer)

          return []
        }

        cachedManifestContent = await generateAppRoutes(server.config.root)
        console.warn(`App router file changed: ${path.relative(server.config.root, file)}`)
        return []
      }
    },

    async generateBundle() {
      if (cachedManifestContent) {
        this.emitFile({
          type: 'asset',
          fileName: 'app-routes.json',
          source: cachedManifestContent,
        })
      }
      else {
        console.warn('App router manifest not generated, skipping emission')
      }
    },

    async closeBundle() {
      for (const timer of pendingHMRUpdates.values()) {
        clearTimeout(timer)
      }
      pendingHMRUpdates.clear()

      if (watcher) {
        await watcher.close()
      }
    },

  }
}
