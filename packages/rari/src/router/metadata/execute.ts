import type { MetadataVirtualPluginOptions } from './virtual-plugin'
import { randomUUID } from 'node:crypto'
import { promises as fs } from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'
import { isRecord } from '@/shared/utils/type-guards'
import { createMetadataVirtualPlugin } from './virtual-plugin'

export interface BuildAndImportMetadataModuleOptions {
  readonly virtualId: string
  readonly sourcePath: string
  readonly sourceCode: string
  readonly aliases?: Readonly<Record<string, string>>
  readonly projectRoot: string
  readonly kind: MetadataVirtualPluginOptions['kind']
  readonly pluginName: string
  readonly label: string
}

function isRariPackageId(id: string): boolean {
  return id === 'rari' || id.startsWith('rari/')
}

export async function buildAndImportMetadataModule(
  options: BuildAndImportMetadataModuleOptions,
): Promise<unknown> {
  const {
    virtualId,
    sourcePath,
    sourceCode,
    aliases = {},
    projectRoot,
    kind,
    pluginName,
    label,
  } = options

  const { build } = await import('rolldown')

  const result = await build({
    input: virtualId,
    external: isRariPackageId,
    platform: 'node',
    write: false,
    output: {
      format: 'esm',
      codeSplitting: false,
    },
    plugins: [
      createMetadataVirtualPlugin({
        name: pluginName,
        virtualId,
        sourcePath,
        sourceCode,
        aliases,
        projectRoot,
        kind,
      }),
    ],
  })

  if (result.output.length === 0) throw new Error(`Failed to build ${label} module`)

  const entryChunk =
    result.output.find(item => item.type === 'chunk' && item.isEntry) ??
    result.output.find(item => item.type === 'chunk')

  if (entryChunk?.type !== 'chunk')
    throw new Error(`No chunk output found in ${label} build result`)

  const cacheDir = path.join(projectRoot, 'node_modules', '.cache', 'rari-metadata')
  await fs.mkdir(cacheDir, { recursive: true })
  const tempFile = path.join(cacheDir, `${label}-${randomUUID()}.mjs`)

  await using stack = new AsyncDisposableStack()
  stack.defer(async () => {
    await fs.rm(tempFile, { force: true })
  })

  await fs.writeFile(tempFile, entryChunk.code, 'utf8')
  return await import(pathToFileURL(tempFile).href)
}

export function requireMetadataDefaultExport(module: unknown, label: string): unknown {
  if (!isRecord(module)) {
    throw new Error(`${label} file must export a default export (either an object or a function)`)
  }

  const defaultExport = module.default
  if (defaultExport == null) {
    throw new Error(`${label} file must export a default export (either an object or a function)`)
  }

  return defaultExport
}

export function metadataProjectRootFromAppDir(appDir: string): string {
  return path.resolve(appDir, '..', '..')
}
