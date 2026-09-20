/* oxlint-disable typescript/prefer-readonly-parameter-types ViteBuilder embeds mutable environment config mutated during emit */
import type { InlineConfig, Plugin, ViteBuilder } from 'vite-plus'
import { Buffer } from 'node:buffer'
import path from 'node:path'
import { createBuilder } from 'vite-plus'
import { toPosixPath } from '@/shared/utils/path'
import { isRecord } from '@/shared/utils/type-guards'

const NODE_PROTOCOL_REGEX = /^node:/

const emitBuilderByRoot = new Map<string, Promise<ViteBuilder>>()

export type ViteEmitBuilderOptions = Readonly<{
  readonly root: string
  readonly configFile?: string | false
  readonly mode?: string
  readonly logLevel?: InlineConfig['logLevel']
}>

function emitBuilderKey(options: ViteEmitBuilderOptions): string {
  return path.resolve(options.root)
}

export async function getOrCreateViteEmitBuilder(
  projectRootOrOptions: string | ViteEmitBuilderOptions,
): Promise<ViteBuilder> {
  const options: ViteEmitBuilderOptions =
    typeof projectRootOrOptions === 'string' ? { root: projectRootOrOptions } : projectRootOrOptions
  const key = emitBuilderKey(options)
  let pending = emitBuilderByRoot.get(key)
  if (pending == null) {
    pending = createBuilder({
      root: key,
      ...(options.configFile !== undefined ? { configFile: options.configFile } : {}),
      ...(options.mode != null && options.mode !== '' ? { mode: options.mode } : {}),
      ...(options.logLevel != null ? { logLevel: options.logLevel } : {}),
    })
    emitBuilderByRoot.set(key, pending)
  }
  return pending
}

export function clearViteEmitBuilder(projectRoot: string): void {
  emitBuilderByRoot.delete(path.resolve(projectRoot))
}

export interface EnvViteBuildEntry {
  readonly entryName: string
  readonly filePath: string
}

export interface EnvViteBuiltEntry {
  readonly code: string
  readonly cssAssetSources: readonly string[]
}

export interface EnvViteBuildResult {
  readonly outputs: ReadonlyMap<string, EnvViteBuiltEntry>
  readonly extraFiles: ReadonlyArray<{ readonly fileName: string; readonly code: string }>
}

export interface RscViteBuildEntry {
  readonly componentId: string
  readonly filePath: string
}
export type RscViteBuiltEntry = EnvViteBuiltEntry
export type RscViteBuildResult = EnvViteBuildResult

interface BuildOutputChunk {
  readonly type: 'chunk'
  readonly isEntry?: boolean
  readonly name?: string
  readonly fileName: string
  readonly code: string
  readonly viteMetadata?: {
    readonly importedCss?: ReadonlySet<string> | readonly string[]
  }
}

interface BuildOutputAsset {
  readonly type: 'asset'
  readonly fileName: string
  readonly source: string | Uint8Array
}

type BuildOutputItem = BuildOutputChunk | BuildOutputAsset

function isBuildOutputItem(value: unknown): value is BuildOutputItem {
  return value != null && typeof value === 'object' && 'type' in value
}

function collectBuildOutputs(result: unknown): BuildOutputItem[] {
  const bags: unknown[] = Array.isArray(result) ? result : [result]
  const items: BuildOutputItem[] = []
  for (const bag of bags) {
    if (bag == null || typeof bag !== 'object') continue
    const output = (bag as { output?: unknown }).output
    if (!Array.isArray(output)) continue
    for (const item of output) {
      if (isBuildOutputItem(item)) items.push(item)
    }
  }
  return items
}

function assetSourceToString(source: string | Uint8Array): string {
  if (typeof source === 'string') return source
  return Buffer.from(source).toString('utf8')
}

function importedCssFileNames(importedCss: unknown): string[] {
  if (importedCss == null) return []
  if (importedCss instanceof Set) return [...importedCss].map(value => toPosixPath(String(value)))
  if (Array.isArray(importedCss)) return importedCss.map(value => toPosixPath(String(value)))
  return []
}

const RSC_EXTERNALS = [
  NODE_PROTOCOL_REGEX,
  'react',
  'react-dom',
  'react/jsx-runtime',
  'react/jsx-dev-runtime',
  'react/compiler-runtime',
  /^rari(?:\/|$)/,
  'react-server-dom-rari/server',
] as const

const SSR_EXTERNALS = [
  NODE_PROTOCOL_REGEX,
  'react',
  'react-dom',
  'react/jsx-runtime',
  'react/jsx-dev-runtime',
  'react/compiler-runtime',
  /^rari(?:\/|$)/,
  'react-server-dom-webpack/client',
  /^react-server-dom-webpack\//,
] as const

type EnvBuildSnapshot = Readonly<{
  write: unknown
  emptyOutDir: unknown
  copyPublicDir: unknown
  minify: unknown
  emitAssets: unknown
  rolldownOptions: unknown
}>

function snapshotEnvBuildConfig(buildConfig: {
  write: unknown
  emptyOutDir: unknown
  copyPublicDir: unknown
  minify: unknown
  emitAssets: unknown
  rolldownOptions: unknown
}): EnvBuildSnapshot {
  return {
    write: buildConfig.write,
    emptyOutDir: buildConfig.emptyOutDir,
    copyPublicDir: buildConfig.copyPublicDir,
    minify: buildConfig.minify,
    emitAssets: buildConfig.emitAssets,
    rolldownOptions: buildConfig.rolldownOptions,
  }
}

function restoreEnvBuildConfig(
  buildConfig: {
    write: unknown
    emptyOutDir: unknown
    copyPublicDir: unknown
    minify: unknown
    emitAssets: unknown
    rolldownOptions: unknown
  },
  previous: EnvBuildSnapshot,
): void {
  buildConfig.write = previous.write
  buildConfig.emptyOutDir = previous.emptyOutDir
  buildConfig.copyPublicDir = previous.copyPublicDir
  buildConfig.minify = previous.minify
  buildConfig.emitAssets = previous.emitAssets
  buildConfig.rolldownOptions = previous.rolldownOptions
}

export type BuildEntriesWithViteEnvironmentOptions = Readonly<{
  viteBuilder: ViteBuilder
  environmentName: 'rsc' | 'ssr'
  entries: readonly EnvViteBuildEntry[]
  minify?: boolean
  label?: string
}>

export async function buildEntriesWithViteEnvironment(
  options: BuildEntriesWithViteEnvironmentOptions,
): Promise<EnvViteBuildResult | null> {
  if (options.entries.length === 0) return { outputs: new Map(), extraFiles: [] }

  const environments = options.viteBuilder.environments as Partial<
    Record<'rsc' | 'ssr' | 'client', (typeof options.viteBuilder.environments)[string]>
  >
  const env = environments[options.environmentName]
  if (env == null) return null

  await env.init()

  const input: Record<string, string> = {}
  for (const entry of options.entries) {
    input[entry.entryName] = entry.filePath
  }

  const buildConfig = env.config.build
  const previous = snapshotEnvBuildConfig(buildConfig)
  const previousRolldown = isRecord(previous.rolldownOptions) ? { ...previous.rolldownOptions } : {}

  const label = options.label ?? options.environmentName.toUpperCase()

  try {
    buildConfig.write = false
    buildConfig.emptyOutDir = false
    buildConfig.copyPublicDir = false
    buildConfig.emitAssets = true
    buildConfig.minify = options.minify ?? false
    buildConfig.rolldownOptions = {
      ...previousRolldown,
      input,
      platform: 'node',
      output: {
        format: 'es',
        entryFileNames: '[name].js',
        chunkFileNames: 'chunks/[name]-[hash].js',
        assetFileNames: 'assets/[name]-[hash][extname]',
      },
      external: [...(options.environmentName === 'ssr' ? SSR_EXTERNALS : RSC_EXTERNALS)],
    }

    const buildResult = await options.viteBuilder.build(env)
    const outputs = collectBuildOutputs(buildResult)
    if (outputs.length === 0) return null

    const cssByFileName = new Map<string, string>()
    const extraFiles: Array<{ readonly fileName: string; readonly code: string }> = []
    const entryOutputs = new Map<string, EnvViteBuiltEntry>()
    const entryCssFiles = new Map<string, string[]>()

    for (const item of outputs) {
      if (item.type === 'asset') {
        const fileName = toPosixPath(item.fileName)
        if (fileName.endsWith('.css')) {
          cssByFileName.set(fileName, assetSourceToString(item.source))
        } else {
          extraFiles.push({ fileName, code: assetSourceToString(item.source) })
        }
        continue
      }

      const fileName = toPosixPath(item.fileName)
      if (item.isEntry) {
        const entryName = item.name
        if (entryName == null || entryName === '') continue
        entryOutputs.set(entryName, {
          code: item.code,
          cssAssetSources: [],
        })
        entryCssFiles.set(entryName, importedCssFileNames(item.viteMetadata?.importedCss))
        continue
      }

      extraFiles.push({ fileName, code: item.code })
    }

    for (const [entryName, built] of entryOutputs) {
      const cssFileNames = entryCssFiles.get(entryName) ?? []
      const cssAssetSources = cssFileNames
        .map(fileName => cssByFileName.get(fileName))
        .filter((source): source is string => source != null)

      if (cssAssetSources.length === 0 && entryOutputs.size === 1 && cssByFileName.size > 0) {
        entryOutputs.set(entryName, {
          ...built,
          cssAssetSources: [...cssByFileName.values()],
        })
        continue
      }

      entryOutputs.set(entryName, { ...built, cssAssetSources })
    }

    for (const entry of options.entries) {
      if (!entryOutputs.has(entry.entryName)) return null
    }

    return { outputs: entryOutputs, extraFiles }
  } catch (error) {
    console.warn(
      `[rari] Vite ${label} environment build failed:`,
      error instanceof Error ? error.message : error,
    )
    return null
  } finally {
    restoreEnvBuildConfig(buildConfig, previous)
  }
}

export type BuildRscEntriesWithViteEnvironmentOptions = Readonly<{
  viteBuilder: ViteBuilder
  entries: readonly RscViteBuildEntry[]
  minify?: boolean
}>

export async function buildRscEntriesWithViteEnvironment(
  options: BuildRscEntriesWithViteEnvironmentOptions,
): Promise<EnvViteBuildResult | null> {
  return buildEntriesWithViteEnvironment({
    viteBuilder: options.viteBuilder,
    environmentName: 'rsc',
    entries: options.entries.map(entry => ({
      entryName: entry.componentId,
      filePath: entry.filePath,
    })),
    minify: options.minify,
    label: 'RSC',
  })
}

export type BuildSsrEntriesWithViteEnvironmentOptions = Readonly<{
  viteBuilder: ViteBuilder
  entries: readonly EnvViteBuildEntry[]
  minify?: boolean
}>

export async function buildSsrEntriesWithViteEnvironment(
  options: BuildSsrEntriesWithViteEnvironmentOptions,
): Promise<EnvViteBuildResult | null> {
  return buildEntriesWithViteEnvironment({
    viteBuilder: options.viteBuilder,
    environmentName: 'ssr',
    entries: options.entries,
    minify: options.minify ?? false,
    label: 'SSR',
  })
}

export function rscBundlePathForComponent(rscDir: string, componentId: string): string {
  return toPosixPath(path.join(rscDir, `${componentId}.js`))
}

export type BuildMetadataModuleWithViteEnvironmentOptions = Readonly<{
  root: string
  virtualId: string
  plugins: readonly Plugin[]
  external?: (id: string) => boolean
}>

export async function buildMetadataModuleWithViteEnvironment(
  options: BuildMetadataModuleWithViteEnvironmentOptions,
): Promise<{ readonly code: string } | null> {
  const builder = await createBuilder({
    configFile: false,
    root: options.root,
    logLevel: 'error',
    plugins: [...options.plugins],
    define: {
      global: 'globalThis',
    },
    resolve: {
      conditions: ['import', 'module', 'default'],
      extensions: ['.mjs', '.js', '.ts', '.tsx', '.jsx'],
      mainFields: ['module', 'main'],
    },
    environments: {
      ssr: {
        consumer: 'server',
        build: {
          write: false,
          emptyOutDir: false,
          copyPublicDir: false,
          emitAssets: false,
          minify: false,
          cssMinify: false,
          target: 'esnext',
          rolldownOptions: {
            input: { entry: options.virtualId },
            platform: 'node',
            external: options.external,
            output: {
              format: 'es',
              codeSplitting: false,
              entryFileNames: '[name].js',
            },
          },
        },
      },
    },
    builder: {
      sharedPlugins: true,
    },
    css: {
      transformer: 'lightningcss',
    },
  })

  const env = builder.environments.ssr
  await env.init()
  const buildResult = await builder.build(env)
  const outputs = collectBuildOutputs(buildResult)
  const entryChunk =
    outputs.find(item => item.type === 'chunk' && item.isEntry) ??
    outputs.find(item => item.type === 'chunk')

  if (entryChunk?.type !== 'chunk') return null
  return { code: entryChunk.code }
}
