import type { MetadataVirtualPluginOptions } from './virtual-plugin'
import { Buffer } from 'node:buffer'
import path from 'node:path'
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
    external: ['rari'],
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

  const dataUrl = `data:text/javascript;base64,${Buffer.from(entryChunk.code).toString('base64')}`
  return import(dataUrl)
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
