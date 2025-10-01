import type { FSWatcher } from 'chokidar'
import type { Plugin, ResolvedConfig, ViteDevServer } from 'rolldown-vite'
import type { Route, RouteGenerationOptions } from './types'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { watch } from 'chokidar'
import {
  createRouteManifest,
  generateFileRoutes,
  validateRoutes,
} from './file-routes'

interface RariRouterPluginOptions {
  pagesDir?: string
  appDir?: string
  useAppRouter?: boolean
  extensions?: string[]
  outDir?: string
  generateTypes?: boolean
  validate?: boolean
  dev?: boolean
  transforms?: Array<(route: Route) => Route>
}

const DEFAULT_OPTIONS: Required<RariRouterPluginOptions> = {
  pagesDir: 'src/pages',
  appDir: 'src/app',
  useAppRouter: false,
  extensions: ['.tsx', '.jsx', '.ts', '.js'],
  outDir: '.rari',
  generateTypes: true,
  validate: true,
  dev: true,
  transforms: [],
}

export function rariRouter(options: RariRouterPluginOptions = {}): Plugin {
  const opts = { ...DEFAULT_OPTIONS, ...options }

  let server: ViteDevServer | undefined
  let routes: Route[] = []
  let isBuilding = false
  let watcher: FSWatcher | undefined

  const writeRoutesModule = async (
    outDir: string,
    routes: Route[],
  ): Promise<void> => {
    const isServerComponent = (filePath: string): boolean => {
      try {
        const fullPath = path.resolve(opts.pagesDir, filePath)
        if (!require('node:fs').existsSync(fullPath)) {
          return false
        }
        const code = require('node:fs').readFileSync(fullPath, 'utf-8')

        const hasClientDirective = code.includes('\'use client\'') || code.includes('"use client"')
        if (hasClientDirective) {
          return false
        }

        const hasClientPatterns
          = code.includes('react-dom/client')
            || code.includes('ReactDOM.createRoot')
            || code.includes('document.')
            || code.includes('window.')
            || code.includes('localStorage')
            || code.includes('sessionStorage')
            || code.includes('navigator.')
            || code.includes('history.')
            || (code.includes('rari/client') && (code.includes('useRouter') || code.includes('RouterProvider')))
            || /addEventListener\s*\(/.test(code)
            || /removeEventListener\s*\(/.test(code)

        if (hasClientPatterns) {
          return false
        }

        const hasNodeImports
          = /from\s+['"]node:/.test(code)
            || /from\s+['"]fs['"]/.test(code)
            || /from\s+['"]path['"]/.test(code)
            || /from\s+['"]crypto['"]/.test(code)
            || /from\s+['"]util['"]/.test(code)
            || /from\s+['"]os['"]/.test(code)
            || /from\s+['"]process['"]/.test(code)

        const hasAsyncDefaultExport = /export\s+default\s+async\s+function/.test(code)

        const hasServerOnlyPatterns
          = code.includes('readFileSync')
            || code.includes('writeFileSync')
            || code.includes('process.env')
            || code.includes('await fetch')

        const hasReactImport = code.includes('react') || code.includes('React')
        const hasJSX = /<[A-Z]/.test(code) || code.includes('jsx') || code.includes('tsx')

        const hasDefaultExport = /export\s+default/.test(code)
        const hasFunctionDeclaration = /function\s+\w+/.test(code) || /const\s+\w+\s*=\s*\([^)]*\)\s*=>/.test(code)
        const hasReactComponent = (hasReactImport || hasJSX) && (hasDefaultExport || hasFunctionDeclaration)

        const isServer
          = hasNodeImports
            || hasAsyncDefaultExport
            || hasServerOnlyPatterns
            || (hasReactComponent && !hasClientDirective)

        return isServer
      }
      catch {
        return false
      }
    }

    const imports = routes
      .map((route, index) => {
        const componentName = `Page${index}`
        const importPath = path
          .relative(outDir, path.resolve(opts.pagesDir, route.filePath))
          .replace(/\\/g, '/')
          .replace(/\.(tsx?|jsx?)$/, '')

        const fullImportPath = importPath.startsWith('.') ? importPath : `./${importPath}`

        if (isServerComponent(route.filePath)) {
          return `// Server component - using RSC wrapper for ${route.filePath}
const ${componentName} = null // Server component placeholder`
        }
        else {
          return `import ${componentName} from '${fullImportPath}'`
        }
      })
      .join('\n')

    const routeDefinitions = routes
      .map((route, index) => {
        const componentName = `Page${index}`
        const componentId = route.filePath
          .replace(/\.(tsx?|jsx?)$/, '')
          .replace(/[^\w/-]/g, '_')

        if (isServerComponent(route.filePath)) {
          return `  {
    path: ${JSON.stringify(route.path)},
    filePath: ${JSON.stringify(route.filePath)},
    component: createServerComponentWrapper('pages/${componentId}', '${route.filePath}'),
    isDynamic: ${route.isDynamic},
    paramNames: ${JSON.stringify(route.paramNames)},
    meta: ${JSON.stringify(route.meta)},
  }`
        }
        else {
          return `  {
    path: ${JSON.stringify(route.path)},
    filePath: ${JSON.stringify(route.filePath)},
    component: ${componentName},
    isDynamic: ${route.isDynamic},
    paramNames: ${JSON.stringify(route.paramNames)},
    meta: ${JSON.stringify(route.meta)},
  }`
        }
      })
      .join(',\n')

    const content = `// This file is auto-generated. Do not edit manually.
/* eslint-disable perfectionist/sort-imports */
import { createServerComponentWrapper } from 'virtual:rsc-integration'
${imports}

export const routes = [
${routeDefinitions}
]

export default routes
`

    await fs.writeFile(path.join(outDir, 'routes.ts'), content)
  }

  const writeTypeDefinitions = async (
    outDir: string,
    routes: Route[],
  ): Promise<void> => {
    const routePaths = routes.map(route => `  | '${route.path}'`).join('\n')
    const dynamicRoutes = routes.filter(route => route.isDynamic)

    const paramTypes = dynamicRoutes
      .map((route) => {
        const paramNames = route.paramNames || []
        const paramType = paramNames
          .map(name => `${name}: string`)
          .join('; ')
        return `  '${route.path}': { ${paramType} }`
      })
      .join('\n')

    const content = `// This file is auto-generated. Do not edit manually.
import type { Route } from 'rari/server'

export type RoutePaths =
${routePaths}

export type RouteParams<T extends RoutePaths> = T extends keyof RouteParamMap
  ? RouteParamMap[T]
  : Record<string, never>

export interface RouteParamMap {
${paramTypes}
}

export declare const routes: typeof Route[]
export default routes
`

    await fs.writeFile(path.join(outDir, 'routes.d.ts'), content)
  }

  const generateRoutes = async (root: string): Promise<Route[]> => {
    const pagesDir = path.resolve(root, opts.pagesDir)

    try {
      const generationOptions: RouteGenerationOptions = {
        pagesDir,
        extensions: opts.extensions,
        transforms: opts.transforms,
      }

      const generatedRoutes = await generateFileRoutes(generationOptions)

      let transformedRoutes = generatedRoutes
      for (const transform of opts.transforms) {
        transformedRoutes = transformedRoutes.map(transform)
      }

      if (opts.validate) {
        const validation = validateRoutes(transformedRoutes)
        if (!validation.valid) {
          console.warn('Route validation failed:', validation.errors)
          if (!opts.dev) {
            throw new Error(
              `Route validation failed: ${validation.errors.join(', ')}`,
            )
          }
        }
      }

      return transformedRoutes
    }
    catch (error) {
      console.error('Failed to generate routes:', error)
      return []
    }
  }

  const generateAppRoutes = async (root: string): Promise<void> => {
    if (!opts.useAppRouter) {
      return
    }

    const appDir = path.resolve(root, opts.appDir)

    try {
      await fs.access(appDir)
    } catch {
      return
    }

    try {
      const { generateAppRouteManifest, writeManifest } = await import('./app-routes')

      const manifest = await generateAppRouteManifest(appDir, {
        extensions: opts.extensions,
      })

      const outDir = path.resolve(root, opts.outDir)
      await fs.mkdir(outDir, { recursive: true })

      await writeManifest(manifest, path.join(outDir, 'app-routes.json'))

      console.warn(`Generated app router manifest with ${manifest.routes.length} routes`)
    }
    catch (error) {
      console.error('Failed to generate app routes:', error)
    }
  }

  const writeGeneratedFiles = async (
    root: string,
    routes: Route[],
  ): Promise<void> => {
    const outDir = path.resolve(root, opts.outDir)

    try {
      await fs.mkdir(outDir, { recursive: true })

      await createRouteManifest(routes, path.join(outDir, 'routes.json'))

      await writeRoutesModule(outDir, routes)

      if (opts.generateTypes) {
        await writeTypeDefinitions(outDir, routes)
      }

      await generateAppRoutes(root)
    }
    catch (error) {
      console.error('Failed to write generated files:', error)
    }
  }

  const setupWatcher = (root: string): void => {
    if (watcher) {
      watcher.close()
    }

    const watchPaths = [path.resolve(root, opts.pagesDir)]

    if (opts.useAppRouter) {
      watchPaths.push(path.resolve(root, opts.appDir))
    }

    watcher = watch(watchPaths, {
      ignored: /node_modules/,
      persistent: true,
      ignoreInitial: true,
    })

    watcher.on('all', async (event: string, filePath: string) => {
      if (opts.extensions.some(ext => filePath.endsWith(ext))) {
        try {
          const newRoutes = await generateRoutes(root)
          routes = newRoutes

          await writeGeneratedFiles(root, routes)

          if (server) {
            const module = server.moduleGraph.getModuleById(
              path.join(root, opts.outDir, 'routes.ts'),
            )
            if (module) {
              server.reloadModule(module)
            }

            if (opts.useAppRouter && filePath.includes(opts.appDir)) {
              server.ws.send({
                type: 'full-reload',
                path: '*'
              })
            }
          }
        }
        catch (error) {
          console.error('Failed to regenerate routes:', error)
        }
      }
    })
  }

  return {
    name: 'rari-router',

    configResolved(config: ResolvedConfig) {
      isBuilding = config.command === 'build'
    },

    async buildStart() {
      const root = process.cwd()
      routes = await generateRoutes(root)
      await writeGeneratedFiles(root, routes)
    },

    configureServer(devServer: ViteDevServer) {
      server = devServer

      if (opts.dev) {
        setupWatcher(devServer.config.root)
      }

      devServer.middlewares.use(
        '/api/routes',
        (req: any, res: any, next: any) => {
          if (req.method === 'GET') {
            res.setHeader('Content-Type', 'application/json')
            res.end(
              JSON.stringify({
                routes: routes.map(route => ({
                  path: route.path,
                  filePath: route.filePath,
                  isDynamic: route.isDynamic,
                  paramNames: route.paramNames,
                  meta: route.meta,
                })),
              }),
            )
          }
          else {
            next()
          }
        },
      )
    },

    async handleHotUpdate(ctx: any) {
      const { file, server } = ctx

      const pagesDir = path.resolve(server.config.root, opts.pagesDir)
      const appDir = path.resolve(server.config.root, opts.appDir)

      const isPageFile
        = file.startsWith(pagesDir)
          && opts.extensions.some(ext => file.endsWith(ext))

      const isAppFile
        = opts.useAppRouter
          && file.startsWith(appDir)
          && opts.extensions.some(ext => file.endsWith(ext))

      if (isPageFile || isAppFile) {
        const newRoutes = await generateRoutes(server.config.root)
        routes = newRoutes

        await writeGeneratedFiles(server.config.root, routes)

        if (isAppFile) {
          console.warn(`App router file changed: ${path.relative(server.config.root, file)}`)
          return []
        }

        return []
      }
    },

    async generateBundle() {
      if (isBuilding) {
        this.emitFile({
          type: 'asset',
          fileName: 'routes.json',
          source: JSON.stringify(
            {
              routes: routes.map(route => ({
                path: route.path,
                filePath: route.filePath,
                isDynamic: route.isDynamic,
                paramNames: route.paramNames,
                meta: route.meta,
              })),
            },
            null,
            2,
          ),
        })

        if (opts.useAppRouter) {
          try {
            const manifestPath = path.join(process.cwd(), opts.outDir, 'app-routes.json')
            const manifestContent = await fs.readFile(manifestPath, 'utf-8')

            this.emitFile({
              type: 'asset',
              fileName: 'app-routes.json',
              source: manifestContent,
            })
          } catch (error) {
            console.warn('App router manifest not found, skipping emission')
          }
        }
      }
    },

    async closeBundle() {
      if (watcher) {
        await watcher.close()
      }
    },

    resolveId(id: string) {
      if (id === 'virtual:rari-routes') {
        return id
      }
    },

    load(id: string) {
      if (id === 'virtual:rari-routes') {
        return `export { routes } from '${path.join(opts.outDir, 'routes.ts')}'`
      }
    },
  }
}
