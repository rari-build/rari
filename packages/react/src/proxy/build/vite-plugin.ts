import type { Plugin } from 'vite-plus'
import type { RariPlugin } from '@/vite/plugin/types'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { toRariPlugin } from '@/vite/plugin/types'

export interface ProxyPluginOptions {
  readonly root?: string
  readonly srcDir?: string
  readonly proxyFileName?: string
  readonly extensions?: readonly string[]
  readonly verbose?: boolean
}

interface ProxyFileInfo {
  filePath: string
  exists: boolean
  relativePath: string
}

const DEFAULT_PROXY_EXTENSIONS = ['.ts', '.tsx', '.js', '.jsx', '.mts', '.mjs'] as const

export async function findProxyFilePath(
  root: string = process.cwd(),
  srcDir: string = 'src',
  options: {
    readonly proxyFileName?: string
    readonly extensions?: readonly string[]
  } = {},
): Promise<string | null> {
  const proxyFileName = options.proxyFileName ?? 'proxy'
  const extensions = options.extensions ?? DEFAULT_PROXY_EXTENSIONS

  for (const ext of extensions) {
    const filePath = path.join(root, `${proxyFileName}${ext}`)
    try {
      await fs.access(filePath)
      return filePath
    } catch {}
  }

  for (const ext of extensions) {
    const filePath = path.join(root, srcDir, `${proxyFileName}${ext}`)
    try {
      await fs.access(filePath)
      return filePath
    } catch {}
  }

  return null
}

export async function hasProxyFile(
  root: string = process.cwd(),
  srcDir: string = 'src',
): Promise<boolean> {
  return (await findProxyFilePath(root, srcDir)) != null
}

export function rariProxy(options: ProxyPluginOptions = {}): RariPlugin {
  const {
    root = process.cwd(),
    srcDir = 'src',
    proxyFileName = 'proxy',
    extensions = DEFAULT_PROXY_EXTENSIONS,
    verbose = false,
  } = options

  let proxyFile: ProxyFileInfo | null = null

  const log = (message: string) => {
    if (verbose) console.warn(`[rari] Proxy: ${message}`)
  }

  async function findProxyFile(): Promise<ProxyFileInfo | null> {
    const filePath = await findProxyFilePath(root, srcDir, { proxyFileName, extensions })
    if (filePath == null) return null

    const relativePath = path.relative(root, filePath)
    log(`Found proxy file: ${relativePath}`)
    return {
      filePath,
      exists: true,
      relativePath,
    }
  }

  const plugin = {
    name: 'rari:proxy',

    async buildStart() {
      proxyFile = await findProxyFile()

      if (proxyFile) log(`Proxy enabled: ${proxyFile.relativePath}`)
      else log('No proxy file found')
    },

    configureServer(server) {
      if (!proxyFile) return

      server.watcher.add(proxyFile.filePath)

      server.watcher.on('change', file => {
        if (file === proxyFile?.filePath) {
          log('Proxy file changed, reloading...')
          server.ws.send({
            type: 'custom',
            event: 'rari:proxy-reload',
          })
        }
      })
    },

    handleHotUpdate({ file, server }) {
      if (file === proxyFile?.filePath) {
        log('Hot reloading proxy...')

        server.ws.send({
          type: 'custom',
          event: 'rari:proxy-reload',
          data: {
            file: proxyFile.relativePath,
          },
        })

        return []
      }

      return undefined
    },
  } satisfies Plugin

  return toRariPlugin(plugin)
}
