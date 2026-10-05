import { promises as fs } from 'node:fs'
import path from 'node:path'
import { resolveAlias } from '@/shared/utils/alias-resolver'
import { resolveWithExtensionsAndIndex } from '@/shared/utils/file-resolver'

export type MetadataModuleType = 'js' | 'jsx' | 'ts' | 'tsx' | 'json'

export function determineMetadataModuleType(
  ext: string,
  kind: 'robots' | 'feed' | 'sitemap',
): MetadataModuleType {
  switch (ext) {
    case 'ts':
      return 'ts'
    case 'tsx':
      return 'tsx'
    case 'js':
    case 'mjs':
      return 'js'
    case 'jsx':
      return 'jsx'
    case 'json':
      if (kind === 'sitemap') return 'json'
      break
    default:
      break
  }

  const allowed =
    kind === 'sitemap' ? '.ts, .tsx, .js, .jsx, .mjs, .json' : '.ts, .tsx, .js, .jsx, .mjs'
  throw new Error(
    `Unsupported ${kind} file extension: ".${ext}". Allowed extensions are: ${allowed}`,
  )
}

export interface MetadataVirtualPluginOptions {
  readonly name: string
  readonly virtualId: string
  readonly sourcePath: string
  readonly sourceCode: string
  readonly aliases?: Readonly<Record<string, string>>
  readonly projectRoot: string
  readonly kind: 'robots' | 'feed' | 'sitemap'
}

export function createMetadataVirtualPlugin(options: MetadataVirtualPluginOptions) {
  const { name, virtualId, sourcePath, sourceCode, aliases = {}, projectRoot, kind } = options

  return {
    name,
    resolveId(id: string, importer?: string) {
      if (id === virtualId) return id

      if (Object.keys(aliases).length > 0) {
        const resolved = resolveAlias(id, aliases, projectRoot)
        if (resolved != null && resolved !== '') {
          const found = resolveWithExtensionsAndIndex(resolved)
          if (found != null && found !== '') return found

          return resolved
        }
      }

      if (id.startsWith('.')) {
        const base =
          importer == null || importer === '' || importer.startsWith('\0') ? sourcePath : importer
        const resolved = path.resolve(path.dirname(base), id)
        const found = resolveWithExtensionsAndIndex(resolved)
        if (found != null && found !== '') return found

        return resolved
      }

      return null
    },
    async load(loadId: string) {
      if (loadId === virtualId) {
        const ext = path.extname(sourcePath).slice(1)
        return { code: sourceCode, moduleType: determineMetadataModuleType(ext, kind) }
      }

      if (loadId && !loadId.startsWith('\0')) {
        try {
          const code = await fs.readFile(loadId, 'utf-8')
          const ext = path.extname(loadId).slice(1)
          return { code, moduleType: determineMetadataModuleType(ext, kind) }
        } catch {
          return null
        }
      }

      return null
    },
  }
}
