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

export function rariRouter(options: RariRouterPluginOptions = {}): Plugin {
  const opts = { ...DEFAULT_OPTIONS, ...options }

  let server: ViteDevServer | undefined
  let watcher: FSWatcher | undefined
  let cachedManifestContent: string | null = null

  const generateAppRoutes = async (root: string): Promise<string | null> => {
    const appDir = path.resolve(root, opts.appDir)

    try {
      await fs.access(appDir)
    }
    catch {
      return null
    }

    try {
      const { generateAppRouteManifest } = await import('./app-routes')

      const manifest = await generateAppRouteManifest(appDir, {
        extensions: opts.extensions,
      })

      const manifestContent = JSON.stringify(manifest, null, 2)

      const outDir = path.resolve(root, opts.outDir)
      await fs.mkdir(outDir, { recursive: true })
      await fs.writeFile(path.join(outDir, 'app-routes.json'), manifestContent, 'utf-8')

      console.warn(`Generated app router manifest with ${manifest.routes.length} routes`)

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
          await generateAppRoutes(root)

          if (server && filePath.includes(opts.appDir)) {
            server.ws.send({
              type: 'full-reload',
              path: '*',
            })
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
      if (watcher) {
        await watcher.close()
      }
    },

  }
}
