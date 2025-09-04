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
  extensions?: string[]
  outDir?: string
  generateTypes?: boolean
  validate?: boolean
  dev?: boolean
  transforms?: Array<(route: Route) => Route>
}

const DEFAULT_OPTIONS: Required<RariRouterPluginOptions> = {
  pagesDir: 'src/pages',
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
    const imports = routes
      .map((route, index) => {
        const componentName = `Page${index}`
        const importPath = path
          .relative(outDir, path.resolve(opts.pagesDir, route.filePath))
          .replace(/\\/g, '/')
          .replace(/\.(tsx?|jsx?)$/, '')

        return `import ${componentName} from '${importPath.startsWith('.') ? importPath : `./${importPath}`}'`
      })
      .join('\n')

    const routeDefinitions = routes
      .map((route, index) => {
        const componentName = `Page${index}`
        return `  {
    path: ${JSON.stringify(route.path)},
    filePath: ${JSON.stringify(route.filePath)},
    component: ${componentName},
    isDynamic: ${route.isDynamic},
    paramNames: ${JSON.stringify(route.paramNames)},
    meta: ${JSON.stringify(route.meta)},
  }`
      })
      .join(',\n')

    const content = `// This file is auto-generated. Do not edit manually.
/* eslint-disable perfectionist/sort-imports */
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
    }
    catch (error) {
      console.error('Failed to write generated files:', error)
    }
  }

  const setupWatcher = (root: string): void => {
    if (watcher) {
      watcher.close()
    }

    const pagesDir = path.resolve(root, opts.pagesDir)

    watcher = watch(pagesDir, {
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
      const isPageFile
        = file.startsWith(pagesDir)
          && opts.extensions.some(ext => file.endsWith(ext))

      if (isPageFile) {
        const newRoutes = await generateRoutes(server.config.root)
        routes = newRoutes

        await writeGeneratedFiles(server.config.root, routes)

        return []
      }
    },

    generateBundle() {
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
