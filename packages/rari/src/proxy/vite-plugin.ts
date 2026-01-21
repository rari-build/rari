import type { Plugin } from 'rolldown-vite'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import process from 'node:process'

export interface ProxyPluginOptions {
  root?: string
  srcDir?: string
  proxyFileName?: string
  extensions?: string[]
  verbose?: boolean
}

interface ProxyFileInfo {
  filePath: string
  exists: boolean
  relativePath: string
}

export function rariProxy(options: ProxyPluginOptions = {}): Plugin {
  const {
    root = process.cwd(),
    srcDir = 'src',
    proxyFileName = 'proxy',
    extensions = ['.ts', '.tsx', '.js', '.jsx', '.mts', '.mjs'],
    verbose = false,
  } = options

  let proxyFile: ProxyFileInfo | null = null

  const log = (message: string) => {
    if (verbose)
      console.warn(`[rari] Proxy: ${message}`)
  }

  async function findProxyFile(): Promise<ProxyFileInfo | null> {
    for (const ext of extensions) {
      const fileName = `${proxyFileName}${ext}`
      const filePath = path.join(root, fileName)

      try {
        await fs.access(filePath)
        log(`Found proxy file: ${fileName}`)
        return {
          filePath,
          exists: true,
          relativePath: fileName,
        }
      }
      catch {}
    }

    const srcPath = path.join(root, srcDir)
    try {
      await fs.access(srcPath)

      for (const ext of extensions) {
        const fileName = `${proxyFileName}${ext}`
        const filePath = path.join(srcPath, fileName)

        try {
          await fs.access(filePath)
          log(`Found proxy file: ${path.join(srcDir, fileName)}`)
          return {
            filePath,
            exists: true,
            relativePath: path.join(srcDir, fileName),
          }
        }
        catch {}
      }
    }
    catch {}

    return null
  }

  return {
    name: 'rari:proxy',

    async buildStart() {
      proxyFile = await findProxyFile()

      if (proxyFile) {
        log(`Proxy enabled: ${proxyFile.relativePath}`)
      }
      else {
        log('No proxy file found')
      }
    },

    configureServer(server) {
      if (!proxyFile)
        return

      server.watcher.add(proxyFile.filePath)

      server.watcher.on('change', (file) => {
        if (file === proxyFile?.filePath) {
          log('Proxy file changed, reloading...')
          server.ws.send({
            type: 'custom',
            event: 'rari:proxy-reload',
          })
        }
      })
    },

    async handleHotUpdate({ file, server }) {
      if (proxyFile && file === proxyFile.filePath) {
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
    },
  }
}

export async function hasProxyFile(
  root: string = process.cwd(),
  srcDir: string = 'src',
): Promise<boolean> {
  const extensions = ['.ts', '.tsx', '.js', '.jsx', '.mts', '.mjs']
  const proxyFileName = 'proxy'

  for (const ext of extensions) {
    const filePath = path.join(root, `${proxyFileName}${ext}`)
    try {
      await fs.access(filePath)
      return true
    }
    catch {}
  }

  for (const ext of extensions) {
    const filePath = path.join(root, srcDir, `${proxyFileName}${ext}`)
    try {
      await fs.access(filePath)
      return true
    }
    catch {}
  }

  return false
}
