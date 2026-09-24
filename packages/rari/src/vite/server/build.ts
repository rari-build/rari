// oxlint-disable typescript/prefer-readonly-parameter-types
import type { Plugin, ViteBuilder } from 'vite-plus'
import type { ModuleAnalysis } from '../analysis/directives'
import type { MdxPluginOptions } from '../mdx/registry'
import type {
  ServerActionConfig,
  ServerCacheConfig,
  ServerCacheControlConfig,
  ServerCacheLayerConfig,
  ServerConfig,
  ServerCSPConfig,
} from './config'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { buildProxyManifest } from '@/proxy/build/analyze'
import { copyAppIconsToOutDir, parseAppIconsFromManifest } from '@/router/metadata/app-icons'
import {
  EXPORTED_CONST_FUNCTION_REGEX,
  EXPORTED_DEFAULT_ARROW_REGEX,
  EXPORTED_FUNCTION_REGEX,
} from '@/shared/regex-constants'
import { resolveAlias } from '@/shared/utils/alias-resolver'
import { contentHash } from '@/shared/utils/content-hash'
import { resolveWithExtensionsAndIndex } from '@/shared/utils/file-resolver'
import { normalizeAssetsDir, toPosixPath } from '@/shared/utils/path'
import { errorMessage, getErrnoCode, isRecord, parseJsonRecord } from '@/shared/utils/type-guards'
import { readViteAliases } from '@/shared/utils/vite-aliases'
import {
  getReadableComponentId,
  getComponentId as getSharedComponentId,
  getProjectRelativePath as getSharedProjectRelativePath,
} from '../analysis/component-ids'
import { analyzeModuleSource } from '../analysis/directives'
import {
  filterExternalDependencies,
  filterRelativeImportSources,
  hasNodeImportsFromAnalysis,
  ModuleAnalysisCache,
  resolveModuleCachePath,
} from '../analysis/module-cache'
import { collectSourceFilePaths, normalizeScanDirs } from '../analysis/source-walker'
import { finalizeStaticImageSourceMapBuild } from '../image/static-import'
import {
  collectMdxContentDirs,
  copyMdxContentDirsToDest,
  resolveMdxPluginOptions,
  resolveMdxRegistryEntries,
} from '../mdx/registry'
import { collectExportNames } from '../transform/client-reference-stub'
import { collectComponentServerCssSources, resolveLayoutCssServerSkipSet } from './css-server-asset'
import {
  buildRscEntriesWithViteEnvironment,
  buildSsrEntriesWithViteEnvironment,
  getOrCreateViteEmitBuilder,
  rscBundlePathForComponent,
} from './rsc-vite-build'

const PROXY_FILE_REGEX = /^proxy\.(?:tsx?|jsx?|mts|mjs)$/
const PROXY_MANIFEST_FILE = 'proxy.json'
const SPECIAL_FILE_REGEX = /^(?:robots|sitemap|feed)\.(?:tsx?|jsx?)$/
const APP_ICON_FILE_REGEX = /^(?:favicon|icon\d*|apple-icon\d*)\.(?:ico|png|jpe?g|svg)$/i
const LAYOUT_FILENAME_REGEX = /^layout\.(?:tsx|ts|jsx|js)$/
export const RARI_CSS_MODULES_PATTERN = '[hash]_[local]'

const EXTERNAL_CLIENT_COMPONENT_MANIFESTS: Array<{
  componentId: string
  devSourceSegments: string[]
  publishedExport: string
  exports: string[]
}> = [
  {
    componentId: 'rari/image',
    devSourceSegments: ['src', 'image', 'image.tsx'],
    publishedExport: 'rari/image',
    exports: ['Image'],
  },
]

const RARI_DIST_DIR = path.dirname(fileURLToPath(import.meta.url))
const RARI_PACKAGE_ROOT = path.dirname(RARI_DIST_DIR)
function isRariInternalPath(filePath: string): boolean {
  return filePath.startsWith(RARI_PACKAGE_ROOT)
}

function resolveErrorBoundarySourcePath(): string | null {
  const devSource = path.join(
    RARI_PACKAGE_ROOT,
    'src',
    'runtime',
    'boundaries',
    'error-boundary-wrapper.tsx',
  )
  if (fs.existsSync(devSource)) return devSource

  try {
    const publishedPath = fileURLToPath(import.meta.resolve('rari/runtime/ErrorBoundaryWrapper'))
    if (fs.existsSync(publishedPath)) return publishedPath
  } catch {}

  return null
}

function isErrorBoundaryWrapperPath(filePath: string): boolean {
  const normalized = toPosixPath(filePath)
  return (
    normalized.includes('ErrorBoundaryWrapper') ||
    normalized.includes('/boundaries/error-boundary-wrapper') ||
    normalized.endsWith('/error-boundary-wrapper.tsx')
  )
}

interface ServerComponentManifest {
  components: Record<
    string,
    {
      id: string
      filePath: string
      relativePath: string
      bundlePath: string
      moduleSpecifier: string
      dependencies: string[]
      hasNodeImports: boolean
      css?: readonly string[]
    }
  >
  mdxRegistry?: Array<{
    name: string
    id: string
    client: boolean
  }>
  buildTime: string
  useCacheBuildId?: string
}

function isServerComponentManifestRecord(value: unknown): value is ServerComponentManifest {
  return isRecord(value) && isRecord(value.components)
}

function ssrClientComponentId(filePath: string, projectRoot: string): string {
  if (isErrorBoundaryWrapperPath(filePath)) return 'virtual:error-boundary-wrapper.tsx'

  const relativePath = toPosixPath(path.relative(projectRoot, filePath))
  if (relativePath.startsWith('..') || path.isAbsolute(relativePath)) return toPosixPath(filePath)

  return relativePath
}

function ssrClientBundleName(filePath: string, projectRoot: string): string {
  if (isErrorBoundaryWrapperPath(filePath))
    return `error_boundary_wrapper_${contentHash('virtual:error-boundary-wrapper.tsx')}`

  if (isRariInternalPath(filePath)) {
    const relative = toPosixPath(path.relative(RARI_PACKAGE_ROOT, filePath))
    return `${getReadableComponentId(relative)}_${contentHash(relative)}`
  }

  return getSharedComponentId(filePath, projectRoot)
}

export interface ServerBuildOptions {
  readonly outDir?: string
  readonly rscDir?: string
  readonly manifestPath?: string
  readonly serverConfigPath?: string
  readonly minify?: boolean
  readonly alias?: Readonly<Record<string, string>>
  readonly assetsDir?: string
  readonly define?: Readonly<Record<string, string>>
  readonly csp?: ServerCSPConfig
  readonly cacheControl?: ServerCacheControlConfig
  readonly cache?: ServerCacheConfig
  readonly action?: ServerActionConfig
  readonly jsPoolSize?: number
  readonly origin?: string
  readonly htmlLimitedBots?: string
  readonly moduleAnalysisCache?: ModuleAnalysisCache
  readonly experimental?: {
    readonly useCache?: boolean
    readonly useCacheRemote?: ServerCacheLayerConfig
  }
  readonly mdx?: MdxPluginOptions
}

export interface ComponentRebuildResult {
  componentId: string
  bundlePath: string
  success: boolean
  error?: string
}

type ResolvedServerBuildOptions = Required<
  Omit<
    ServerBuildOptions,
    | 'csp'
    | 'cacheControl'
    | 'cache'
    | 'action'
    | 'jsPoolSize'
    | 'origin'
    | 'htmlLimitedBots'
    | 'define'
    | 'serverConfigPath'
    | 'experimental'
    | 'moduleAnalysisCache'
    | 'mdx'
  >
> & {
  serverConfigPath: string
  csp?: ServerBuildOptions['csp']
  cacheControl?: ServerBuildOptions['cacheControl']
  cache?: ServerBuildOptions['cache']
  action?: ServerBuildOptions['action']
  jsPoolSize?: ServerBuildOptions['jsPoolSize']
  origin?: ServerBuildOptions['origin']
  htmlLimitedBots?: ServerBuildOptions['htmlLimitedBots']
  define?: ServerBuildOptions['define']
  experimental?: ServerBuildOptions['experimental']
  moduleAnalysisCache?: ModuleAnalysisCache
  mdx?: ServerBuildOptions['mdx']
}

export function isServerComponentFromAnalysis(filePath: string, analysis: ModuleAnalysis): boolean {
  if (filePath.includes('node_modules')) return false

  return !analysis.directives.hasUseClient && !analysis.directives.hasUseServer
}

export class ServerComponentBuilder {
  private readonly serverComponents = new Map<
    string,
    {
      filePath: string
      originalCode: string
      dependencies: string[]
      hasNodeImports: boolean
    }
  >()

  private readonly serverActions = new Map<
    string,
    {
      filePath: string
      originalCode: string
      dependencies: string[]
      hasNodeImports: boolean
    }
  >()

  private readonly options: ResolvedServerBuildOptions
  private readonly projectRoot: string

  private readonly buildCache = new Map<
    string,
    {
      code: string
      css: string[]
      timestamp: number
      sourceDependencies: string[]
      bundledDependencies: string[]
    }
  >()

  private useCacheBuildId: string | null = null

  private readonly fileImporters = new Map<string, Set<string>>()
  private readonly moduleAnalysisCache: ModuleAnalysisCache
  private readonly discoveredExternalClientComponents = new Set<string>()
  private readonly clientComponentFiles = new Map<string, string>()
  private layoutCssSkipSet: Set<string> | null = null
  private viteBuilder: ViteBuilder | null = null
  private envEmitLock: Promise<void> = Promise.resolve()

  private getLayoutCssSkipSet(): Set<string> {
    this.layoutCssSkipSet ??= resolveLayoutCssServerSkipSet(this.projectRoot, this.options.alias)
    return this.layoutCssSkipSet
  }

  private resolveServerCssSources(filePath: string, code: string): string[] {
    return collectComponentServerCssSources({
      filePath,
      code,
      projectRoot: this.projectRoot,
      aliases: this.options.alias,
      layoutCssSkip: this.getLayoutCssSkipSet(),
    })
  }

  private resolveExtraFileOutPath(fileName: string): string {
    const normalized = toPosixPath(fileName)
    if (
      normalized === this.options.assetsDir ||
      normalized.startsWith(`${this.options.assetsDir}/`)
    ) {
      return path.join(this.options.outDir, normalized)
    }
    return path.join(this.options.outDir, this.options.rscDir, normalized)
  }

  setViteBuilder(viteBuilder: ViteBuilder | null): void {
    this.viteBuilder = viteBuilder
  }

  getProjectRoot(): string {
    return this.projectRoot
  }

  private async getEmitBuilder(): Promise<ViteBuilder> {
    if (this.viteBuilder != null) return this.viteBuilder
    this.viteBuilder = await getOrCreateViteEmitBuilder({
      root: this.projectRoot,
      logLevel: 'error',
    })
    return this.viteBuilder
  }

  private async withEnvEmitLock<T>(fn: () => Promise<T>): Promise<T> {
    const previous = this.envEmitLock
    let release!: () => void
    this.envEmitLock = new Promise(resolve => {
      release = resolve
    })
    await previous
    try {
      return await fn()
    } finally {
      release()
    }
  }

  private async emitRscEntries(
    entries: ReadonlyArray<{ readonly componentId: string; readonly filePath: string }>,
    options?: { readonly minify?: boolean; readonly codeSplitting?: boolean },
  ) {
    return this.withEnvEmitLock(async () => {
      const viteBuilder = await this.getEmitBuilder()
      const result = await buildRscEntriesWithViteEnvironment({
        viteBuilder,
        entries,
        minify: options?.minify ?? this.options.minify,
        codeSplitting: options?.codeSplitting,
      })
      if (result == null) {
        throw new Error('Vite RSC environment build failed')
      }
      return result
    })
  }

  recordClientComponent(filePath: string, code: string): void {
    this.clientComponentFiles.set(filePath, code)
  }

  getClientComponentFiles(): Array<{ filePath: string; code: string }> {
    return [...this.clientComponentFiles.entries()].map(([filePath, code]) => ({
      filePath,
      code,
    }))
  }

  getClientComponentPaths(): string[] {
    return [...this.clientComponentFiles.keys()]
  }

  getModuleAnalysisCache(): ModuleAnalysisCache {
    return this.moduleAnalysisCache
  }

  clearClientComponentFiles(): void {
    this.clientComponentFiles.clear()
  }

  getComponentCount(): number {
    return this.serverComponents.size + this.serverActions.size
  }

  hasComponent(filePath: string): boolean {
    return this.serverComponents.has(filePath) || this.serverActions.has(filePath)
  }

  removeComponent(filePath: string): void {
    this.serverComponents.delete(filePath)
    this.serverActions.delete(filePath)
    this.moduleAnalysisCache.invalidate(filePath)
  }

  getImportGraph(): ReadonlyMap<string, ReadonlySet<string>> {
    const copy = new Map<string, Set<string>>()
    for (const [key, value] of this.fileImporters) {
      copy.set(key, new Set(value))
    }

    return copy
  }

  private async writeComponentCssAsset(
    componentId: string,
    cssModules: string[],
  ): Promise<string[]> {
    if (cssModules.length === 0) return []

    const assetsDir = path.join(this.options.outDir, 'assets', 'server')
    await fs.promises.mkdir(assetsDir, { recursive: true })

    const cssContent = `${cssModules.join('\n')}\n`
    const cssFileName = `${contentHash(componentId + cssContent, 12)}.css`
    const cssPath = path.join(assetsDir, cssFileName)
    await fs.promises.writeFile(cssPath, cssContent, 'utf-8')

    return [`/assets/server/${cssFileName}`]
  }

  private getComponentIdFromRouteManifestPath(filePath: string): string {
    return this.getComponentId(path.join(this.projectRoot, 'src', 'app', filePath))
  }

  private getComponentReferenceId(filePath: string): string {
    return this.getReadableComponentId(this.getProjectRelativePath(filePath))
  }

  private async writeRouteCssEntries(manifest: ServerComponentManifest): Promise<void> {
    const routesPath = path.join(this.options.outDir, this.options.rscDir, 'routes.json')
    if (!fs.existsSync(routesPath)) return

    const content = await fs.promises.readFile(routesPath, 'utf-8')
    const parsed: unknown = JSON.parse(content)
    if (!isRecord(parsed)) return
    const routeManifestRecord = parsed

    const applyCss = (entries: unknown) => {
      if (!Array.isArray(entries)) return

      for (const entry of entries) {
        if (!isRecord(entry)) continue

        const filePath = entry.filePath
        if (typeof filePath !== 'string' || filePath === '') continue

        const componentId = this.getComponentIdFromRouteManifestPath(filePath)
        entry.componentId = componentId

        const css = manifest.components[componentId]?.css ?? []
        if (css.length) {
          entry.css = css
        } else {
          delete entry.css
        }
      }
    }

    applyCss(routeManifestRecord.routes)
    applyCss(routeManifestRecord.layouts)
    applyCss(routeManifestRecord.loading)
    applyCss(routeManifestRecord.errors)
    applyCss(routeManifestRecord.notFound)
    applyCss(routeManifestRecord.templates)

    const apiRoutes = routeManifestRecord.apiRoutes
    if (Array.isArray(apiRoutes)) {
      for (const entry of apiRoutes) {
        if (!isRecord(entry)) continue

        const filePath = entry.filePath
        if (typeof filePath === 'string' && filePath !== '') {
          entry.componentId = this.getComponentIdFromRouteManifestPath(filePath)
        }
      }
    }

    await fs.promises.writeFile(routesPath, JSON.stringify(routeManifestRecord), 'utf-8')
  }

  constructor(projectRoot: string, options: ServerBuildOptions = {}) {
    this.projectRoot = path.resolve(projectRoot)
    this.moduleAnalysisCache = options.moduleAnalysisCache ?? new ModuleAnalysisCache()
    const rscDir = options.rscDir != null && options.rscDir !== '' ? options.rscDir : 'server'
    const rawOutDir =
      options.outDir != null && options.outDir !== ''
        ? options.outDir
        : path.join(this.projectRoot, 'dist')
    this.options = {
      outDir: path.isAbsolute(rawOutDir) ? rawOutDir : path.resolve(this.projectRoot, rawOutDir),
      rscDir,
      manifestPath:
        options.manifestPath != null && options.manifestPath !== ''
          ? options.manifestPath
          : path.join(rscDir, 'manifest.json'),
      serverConfigPath:
        options.serverConfigPath != null && options.serverConfigPath !== ''
          ? options.serverConfigPath
          : path.join(rscDir, 'config.json'),
      minify: options.minify ?? process.env.NODE_ENV === 'production',
      alias: options.alias ?? {},
      assetsDir:
        options.assetsDir != null && options.assetsDir !== ''
          ? normalizeAssetsDir(options.assetsDir)
          : 'assets',
      define: options.define,
      csp: options.csp,
      cacheControl: options.cacheControl,
      cache: options.cache,
      action: options.action,
      jsPoolSize: options.jsPoolSize,
      origin: options.origin,
      htmlLimitedBots: options.htmlLimitedBots,
      experimental: options.experimental,
      mdx: options.mdx,
    }
  }

  getModuleAnalysis(filePath: string, source?: string): ModuleAnalysis {
    return this.moduleAnalysisCache.get(filePath, source)
  }

  isServerComponent(filePath: string, source?: string): boolean {
    try {
      const analysis = this.moduleAnalysisCache.get(filePath, source)
      return isServerComponentFromAnalysis(filePath, analysis)
    } catch {
      return false
    }
  }

  private isClientComponent(filePath: string, source?: string): boolean {
    try {
      return this.moduleAnalysisCache.get(filePath, source).directives.hasUseClient
    } catch {
      return false
    }
  }

  resolveImportedFilePath(importerPath: string, importPath: string): string | null {
    let resolvedPath: string | null = null

    if (importPath.startsWith('./') || importPath.startsWith('../')) {
      const importerDir = path.dirname(importerPath)
      resolvedPath = path.resolve(importerDir, importPath)
    } else {
      resolvedPath = resolveAlias(importPath, this.options.alias, this.projectRoot)
      if ((resolvedPath == null || resolvedPath === '') && importPath.startsWith('@/')) {
        const relativePath = importPath.slice(2)
        resolvedPath = path.join(this.projectRoot, 'src', relativePath)
      }
    }

    if (resolvedPath == null || resolvedPath === '') return null

    return resolveWithExtensionsAndIndex(resolvedPath, ['', '.ts', '.tsx', '.js', '.jsx'])
  }

  populateImportGraphFromFiles(
    files: ReadonlyArray<{ readonly filePath: string; readonly analysis: ModuleAnalysis }>,
  ): void {
    this.fileImporters.clear()

    for (const { filePath, analysis } of files) {
      for (const importPath of filterRelativeImportSources(analysis.importSources)) {
        const foundPath = this.resolveImportedFilePath(filePath, importPath)
        if (foundPath == null || foundPath === '') continue

        if (!this.fileImporters.has(foundPath)) this.fileImporters.set(foundPath, new Set())

        this.fileImporters.get(foundPath)!.add(filePath)
      }
    }
  }

  buildImportGraph(srcDir: string): void {
    const files = collectScannedFiles(this, [srcDir])
    this.populateImportGraphFromFiles(files)
  }

  isOnlyImportedByClientComponents(filePath: string): boolean {
    const importers = this.fileImporters.get(filePath)

    if (!importers || importers.size === 0) return false

    for (const importer of importers) {
      if (this.isClientComponent(importer)) continue

      if (!this.isOnlyImportedByClientComponents(importer)) return false
    }

    return true
  }

  addServerComponent(filePath: string, source?: string, analysis?: ModuleAnalysis) {
    const code = source ?? fs.readFileSync(filePath, 'utf-8')
    const moduleAnalysis = analysis ?? this.moduleAnalysisCache.get(filePath, code)
    const dependencies = filterExternalDependencies(moduleAnalysis.importSources)
    const hasNodeImports = hasNodeImportsFromAnalysis(moduleAnalysis)

    if (moduleAnalysis.directives.hasUseServer) {
      this.serverActions.set(filePath, {
        filePath,
        originalCode: code,
        dependencies,
        hasNodeImports,
      })
      return
    }

    if (!isServerComponentFromAnalysis(filePath, moduleAnalysis)) return

    this.serverComponents.set(filePath, {
      filePath,
      originalCode: code,
      dependencies,
      hasNodeImports,
    })
  }

  private isServerAction(code: string, filePath?: string): boolean {
    if (filePath != null && filePath !== '')
      return this.moduleAnalysisCache.get(filePath, code).directives.hasUseServer

    return analyzeModuleSource(code).directives.hasUseServer
  }

  private extractDependencies(code: string, filePath?: string): string[] {
    const analysis =
      filePath != null && filePath !== ''
        ? this.moduleAnalysisCache.get(filePath, code)
        : analyzeModuleSource(code)

    return filterExternalDependencies(analysis.importSources)
  }

  private hasNodeImports(code: string, filePath?: string): boolean {
    const analysis =
      filePath != null && filePath !== ''
        ? this.moduleAnalysisCache.get(filePath, code)
        : analyzeModuleSource(code)

    return hasNodeImportsFromAnalysis(analysis)
  }

  async getTransformedComponentsForDevelopment(
    filter?: (filePath: string) => boolean,
  ): Promise<Array<{ id: string; code: string; isAction: boolean }>> {
    const entries: Array<{ componentId: string; filePath: string; isAction: boolean }> = []

    for (const [filePath] of this.serverComponents) {
      if (filter && !filter(filePath)) continue
      entries.push({
        componentId: this.getComponentId(filePath),
        filePath,
        isAction: false,
      })
    }

    for (const [filePath] of this.serverActions) {
      if (filter && !filter(filePath)) continue
      entries.push({
        componentId: this.getComponentId(filePath),
        filePath,
        isAction: true,
      })
    }

    if (entries.length === 0) return []

    const result = await this.emitRscEntries(
      entries.map(({ componentId, filePath }) => ({ componentId, filePath })),
      { minify: this.options.minify, codeSplitting: false },
    )

    for (const file of result.extraFiles) {
      const fullPath = this.resolveExtraFileOutPath(file.fileName)
      await fs.promises.mkdir(path.dirname(fullPath), { recursive: true })
      await fs.promises.writeFile(fullPath, file.code)
    }

    return entries.map(entry => {
      const output = result.outputs.get(entry.componentId)
      if (output == null) {
        throw new Error(`Vite RSC environment build missed entry: ${entry.componentId}`)
      }
      return {
        id: entry.componentId,
        code: output.code,
        isAction: entry.isAction,
      }
    })
  }

  private async emitProxyManifest(serverOutDir: string): Promise<void> {
    const proxyManifestPath = path.join(serverOutDir, PROXY_MANIFEST_FILE)
    const proxyEntries = [...this.serverComponents.entries()].filter(([filePath]) =>
      PROXY_FILE_REGEX.test(path.basename(filePath)),
    )

    if (proxyEntries.length === 0) {
      try {
        await fs.promises.unlink(proxyManifestPath)
      } catch (error: unknown) {
        if (getErrnoCode(error) !== 'ENOENT')
          console.warn(`Failed to remove proxy manifest file:`, error)
      }
      return
    }

    const [filePath, component] = proxyEntries[0]
    const relativePath = toPosixPath(this.getProjectRelativePath(filePath))
    const componentId = this.getComponentId(relativePath)
    const bundlePath = toPosixPath(path.join(this.options.rscDir, `${componentId}.js`))
    const proxyManifest = buildProxyManifest({
      proxyFile: relativePath,
      code: component.originalCode,
      bundlePath,
    })

    if (!proxyManifest.requiresRuntime) this.serverComponents.delete(filePath)

    await fs.promises.writeFile(proxyManifestPath, JSON.stringify(proxyManifest), 'utf-8')
  }

  async buildServerComponents(): Promise<ServerComponentManifest> {
    const serverOutDir = path.join(this.options.outDir, this.options.rscDir)

    await fs.promises.mkdir(serverOutDir, { recursive: true })
    await this.emitProxyManifest(serverOutDir)

    const manifest: ServerComponentManifest = {
      components: {},
      buildTime: new Date().toISOString(),
    }

    const useCacheEnabled =
      this.options.experimental?.useCache === true ||
      this.options.experimental?.useCacheRemote != null

    const allRscEntries = [...this.serverComponents.entries(), ...this.serverActions.entries()]
    const viteBuilt = await this.buildRscEntriesWithVite(allRscEntries)

    for (const file of viteBuilt.extraFiles) {
      const fullPath = this.resolveExtraFileOutPath(file.fileName)
      await fs.promises.mkdir(path.dirname(fullPath), { recursive: true })
      await fs.promises.writeFile(fullPath, file.code)
    }

    for (const [filePath, built] of viteBuilt.entries) {
      const relativePath = path.relative(this.projectRoot, filePath)
      const componentId = this.getComponentId(filePath)
      const bundlePath = rscBundlePathForComponent(this.options.rscDir, componentId)
      const fullBundlePath = path.join(this.options.outDir, bundlePath)
      await fs.promises.mkdir(path.dirname(fullBundlePath), { recursive: true })
      await fs.promises.writeFile(fullBundlePath, built.code, 'utf-8')
      const component = this.serverComponents.get(filePath) ?? this.serverActions.get(filePath)
      const cssSources = this.resolveServerCssSources(
        filePath,
        component?.originalCode ?? (await fs.promises.readFile(filePath, 'utf-8')),
      )
      const css = [
        ...(await this.writeComponentCssAsset(
          componentId,
          built.cssAssetSources.length > 0 ? [...built.cssAssetSources] : cssSources,
        )),
      ]
      manifest.components[componentId] = {
        id: componentId,
        filePath,
        relativePath,
        bundlePath,
        moduleSpecifier: pathToFileURL(path.resolve(this.projectRoot, fullBundlePath)).href,
        dependencies: [...(component?.dependencies ?? [])],
        hasNodeImports: component?.hasNodeImports ?? false,
        css,
      }
    }

    if (useCacheEnabled) {
      this.useCacheBuildId = contentHash(JSON.stringify(manifest.components), 16)
      manifest.useCacheBuildId = this.useCacheBuildId
    }

    const manifestPath = path.join(this.options.outDir, this.options.manifestPath)
    await fs.promises.writeFile(manifestPath, JSON.stringify(manifest), 'utf-8')
    await this.writeRouteCssEntries(manifest)

    const serverConfig: ServerConfig = {}
    if (this.options.csp) serverConfig.csp = this.options.csp
    if (this.options.cacheControl) serverConfig.cacheControl = this.options.cacheControl
    if (this.options.cache) serverConfig.cache = this.options.cache
    if (this.options.action) serverConfig.action = this.options.action
    if (this.options.jsPoolSize != null) serverConfig.jsPoolSize = this.options.jsPoolSize
    const origin = this.options.origin?.trim().replace(/\/+$/, '')
    if (origin != null && origin !== '') serverConfig.origin = origin
    if (this.options.htmlLimitedBots != null)
      serverConfig.htmlLimitedBots = this.options.htmlLimitedBots
    if (
      this.options.experimental?.useCacheRemote != null ||
      (this.useCacheBuildId != null && this.useCacheBuildId !== '')
    ) {
      serverConfig.useCache = {
        ...(this.options.experimental?.useCacheRemote
          ? { remote: this.options.experimental.useCacheRemote }
          : {}),
        ...(this.useCacheBuildId != null && this.useCacheBuildId !== ''
          ? { buildId: this.useCacheBuildId }
          : {}),
      }
      if (!this.options.experimental?.useCache && this.options.experimental?.useCacheRemote) {
        console.warn(
          "[server-build] experimental.useCacheRemote is set without experimental.useCache; the 'use cache' transform will still run because useCacheRemote is configured.",
        )
      }
    }

    const serverConfigPath = path.join(this.options.outDir, this.options.serverConfigPath)

    if (Object.keys(serverConfig).length === 0) {
      try {
        await fs.promises.unlink(serverConfigPath)
      } catch (error: unknown) {
        if (getErrnoCode(error) !== 'ENOENT')
          console.warn(`Failed to remove server config file:`, error)
      }
    } else {
      await fs.promises.writeFile(serverConfigPath, JSON.stringify(serverConfig), 'utf-8')
    }

    return manifest
  }

  private async buildRscEntriesWithVite(
    entries: ReadonlyArray<
      readonly [
        string,
        {
          readonly filePath: string
          readonly dependencies: readonly string[]
          readonly hasNodeImports: boolean
        },
      ]
    >,
  ): Promise<{
    readonly entries: Map<
      string,
      {
        readonly code: string
        readonly cssAssetSources: readonly string[]
      }
    >
    readonly extraFiles: ReadonlyArray<{
      readonly fileName: string
      readonly code: string | Uint8Array
    }>
  }> {
    const empty = {
      entries: new Map<
        string,
        {
          readonly code: string
          readonly cssAssetSources: readonly string[]
        }
      >(),
      extraFiles: [] as Array<{ readonly fileName: string; readonly code: string | Uint8Array }>,
    }
    if (entries.length === 0) return empty
    if (this.viteBuilder == null) {
      throw new Error('Vite builder is required for RSC environment emit')
    }
    const viteBuilder = this.viteBuilder

    return this.withEnvEmitLock(async () => {
      const viteEntries = entries.map(([filePath]) => ({
        componentId: this.getComponentId(filePath),
        filePath,
      }))

      const result = await buildRscEntriesWithViteEnvironment({
        viteBuilder,
        entries: viteEntries,
        minify: this.options.minify,
      })
      if (result == null) {
        throw new Error('Vite RSC environment build failed')
      }

      const built = new Map<
        string,
        {
          readonly code: string
          readonly cssAssetSources: readonly string[]
        }
      >()
      for (const [filePath] of entries) {
        const componentId = this.getComponentId(filePath)
        const output = result.outputs.get(componentId)
        if (output == null) {
          throw new Error(`Vite RSC environment build missed entry: ${componentId}`)
        }
        built.set(filePath, output)
      }

      return { entries: built, extraFiles: [...result.extraFiles] }
    })
  }

  async buildMdxRegistry(mdxOptions?: MdxPluginOptions): Promise<void> {
    const entries = resolveMdxRegistryEntries({
      projectRoot: this.projectRoot,
      mdxOptions,
      alias: this.options.alias,
      cache: this.moduleAnalysisCache,
    })

    const manifestPath = path.join(this.options.outDir, this.options.manifestPath)
    let manifest: ServerComponentManifest

    if (fs.existsSync(manifestPath)) {
      const content = await fs.promises.readFile(manifestPath, 'utf-8')
      const parsed = parseJsonRecord(content)
      manifest =
        parsed && isServerComponentManifestRecord(parsed)
          ? parsed
          : {
              components: {},
              buildTime: new Date().toISOString(),
            }
    } else {
      manifest = {
        components: {},
        buildTime: new Date().toISOString(),
      }
    }

    manifest.mdxRegistry = entries.map(entry => ({
      name: entry.name,
      id: entry.moduleId,
      client: entry.client,
    }))

    await fs.promises.writeFile(manifestPath, JSON.stringify(manifest), 'utf-8')
  }

  async buildSSRClientComponents(): Promise<void> {
    const ssrOutDir = path.join(this.options.outDir, 'ssr')
    await fs.promises.mkdir(ssrOutDir, { recursive: true })

    const clientFiles: Array<{ filePath: string; code: string }> = [
      ...this.getClientComponentFiles(),
    ]

    for (const extPath of this.discoveredExternalClientComponents) {
      if (clientFiles.some(f => f.filePath === extPath)) continue
      try {
        const code = fs.readFileSync(extPath, 'utf-8')
        clientFiles.push({ filePath: extPath, code })
      } catch {}
    }

    try {
      const errorBoundarySource = resolveErrorBoundarySourcePath()
      if (errorBoundarySource != null && errorBoundarySource !== '') {
        const code = fs.readFileSync(errorBoundarySource, 'utf-8')
        clientFiles.push({ filePath: errorBoundarySource, code })
      }
    } catch {}

    const externalSources: Array<{
      componentId: string
      filePath: string
      code: string
      exports: string[]
      bundleName: string
    }> = []
    for (const {
      componentId,
      devSourceSegments,
      publishedExport,
      exports,
    } of EXTERNAL_CLIENT_COMPONENT_MANIFESTS) {
      const sourcePath = this.resolveExternalClientSourcePath(devSourceSegments, publishedExport)
      if (sourcePath == null || sourcePath === '') continue
      try {
        const code = fs.readFileSync(sourcePath, 'utf-8')
        externalSources.push({
          componentId,
          filePath: sourcePath,
          code,
          exports: [...exports],
          bundleName: `external_${contentHash(componentId)}`,
        })
      } catch {}
    }

    const manifest: Record<
      string,
      { id: string; filePath: string; bundlePath: string; exports: string[] }
    > = {}

    const viteBuilt = await this.buildSsrEntriesWithVite(clientFiles, externalSources)
    for (const file of viteBuilt.extraFiles) {
      const normalized = toPosixPath(file.fileName)
      const fullPath =
        normalized === this.options.assetsDir || normalized.startsWith(`${this.options.assetsDir}/`)
          ? path.join(this.options.outDir, normalized)
          : path.join(ssrOutDir, normalized)
      await fs.promises.mkdir(path.dirname(fullPath), { recursive: true })
      await fs.promises.writeFile(fullPath, file.code)
    }

    for (const [filePath, built] of viteBuilt.entries) {
      const external = externalSources.find(entry => entry.filePath === filePath)
      const componentId = external?.componentId ?? ssrClientComponentId(filePath, this.projectRoot)
      const bundleName = external?.bundleName ?? ssrClientBundleName(filePath, this.projectRoot)
      const bundlePath = `ssr/${bundleName}.js`
      const fullBundlePath = path.join(this.options.outDir, bundlePath)
      await fs.promises.mkdir(path.dirname(fullBundlePath), { recursive: true })
      await fs.promises.writeFile(fullBundlePath, built.code, 'utf-8')
      const sourceCode =
        external?.code ??
        clientFiles.find(entry => entry.filePath === filePath)?.code ??
        fs.readFileSync(filePath, 'utf-8')
      manifest[componentId] = {
        id: componentId,
        filePath,
        bundlePath,
        exports: external?.exports ?? this.extractExportNames(sourceCode),
      }
    }

    await this.writeClientReferenceManifest(ssrOutDir, manifest)
  }

  private async buildSsrEntriesWithVite(
    clientFiles: ReadonlyArray<{ readonly filePath: string; readonly code: string }>,
    externalSources: ReadonlyArray<{
      readonly filePath: string
      readonly bundleName: string
    }> = [],
  ): Promise<{
    readonly entries: Map<string, { readonly code: string }>
    readonly extraFiles: ReadonlyArray<{
      readonly fileName: string
      readonly code: string | Uint8Array
    }>
  }> {
    const empty = {
      entries: new Map<string, { readonly code: string }>(),
      extraFiles: [] as Array<{ readonly fileName: string; readonly code: string | Uint8Array }>,
    }

    const viteEntries = [
      ...clientFiles.map(({ filePath }) => ({
        entryName: ssrClientBundleName(filePath, this.projectRoot),
        filePath,
      })),
      ...externalSources.map(entry => ({
        entryName: entry.bundleName,
        filePath: entry.filePath,
      })),
    ]
    if (viteEntries.length === 0) return empty
    if (this.viteBuilder == null) {
      throw new Error('Vite builder is required for SSR environment emit')
    }
    const viteBuilder = this.viteBuilder

    return this.withEnvEmitLock(async () => {
      const result = await buildSsrEntriesWithViteEnvironment({
        viteBuilder,
        entries: viteEntries,
        minify: this.options.minify,
      })
      if (result == null) {
        throw new Error('Vite SSR environment build failed')
      }

      const built = new Map<string, { readonly code: string }>()
      for (const entry of viteEntries) {
        const output = result.outputs.get(entry.entryName)
        if (output == null) {
          throw new Error(`Vite SSR environment build missed entry: ${entry.entryName}`)
        }
        built.set(entry.filePath, { code: output.code })
      }

      return { entries: built, extraFiles: [...result.extraFiles] }
    })
  }

  private async writeClientReferenceManifest(
    ssrOutDir: string,
    manifest: {
      [key: string]: { id: string; filePath: string; bundlePath: string; exports: string[] }
    },
  ): Promise<void> {
    const manifestPath = path.join(ssrOutDir, 'manifest.json')
    await fs.promises.writeFile(manifestPath, JSON.stringify(manifest), 'utf-8')

    const clientReferenceManifest: Record<string, { id: string; chunks: string; name: string }> = {}
    for (const [componentId, entry] of Object.entries(manifest)) {
      for (const exportName of entry.exports) {
        const fullId = `${componentId}#${exportName}`
        clientReferenceManifest[fullId] = {
          id: fullId,
          chunks: `/${entry.bundlePath}`,
          name: exportName,
        }
      }
    }

    const serverOutDir = path.join(this.options.outDir, 'server')
    await fs.promises.mkdir(serverOutDir, { recursive: true })
    const clientRefManifestPath = path.join(serverOutDir, 'client-reference-manifest.json')
    await fs.promises.writeFile(
      clientRefManifestPath,
      JSON.stringify(clientReferenceManifest),
      'utf-8',
    )
  }

  private resolveExternalClientSourcePath(
    devSourceSegments: readonly string[],
    publishedExport: string,
  ): string | null {
    const devPath = path.join(RARI_PACKAGE_ROOT, ...devSourceSegments)
    if (fs.existsSync(devPath)) return devPath

    try {
      const publishedPath = fileURLToPath(import.meta.resolve(publishedExport))
      if (fs.existsSync(publishedPath)) return publishedPath
    } catch {}

    return null
  }

  private extractExportNames(code: string): string[] {
    return collectExportNames(code)
  }

  private getProjectRelativePath(filePath: string): string {
    return getSharedProjectRelativePath(filePath, this.projectRoot)
  }

  private getReadableComponentId(projectRelativePath: string): string {
    return getReadableComponentId(projectRelativePath)
  }

  private getComponentId(filePath: string): string {
    return getSharedComponentId(filePath, this.projectRoot)
  }

  async rebuildComponent(filePath: string): Promise<ComponentRebuildResult> {
    if (LAYOUT_FILENAME_REGEX.test(path.basename(filePath))) {
      this.layoutCssSkipSet = null
    }

    const componentId = this.getComponentId(filePath)

    const code = await fs.promises.readFile(filePath, 'utf-8')
    const sourceDependencies = this.extractDependencies(code, filePath)
    const hasNodeImports = this.hasNodeImports(code, filePath)

    const componentData = {
      filePath,
      originalCode: code,
      dependencies: sourceDependencies,
      hasNodeImports,
    }

    if (this.isServerAction(code, filePath)) {
      this.serverActions.set(filePath, componentData)
      this.serverComponents.delete(filePath)
    } else {
      this.serverComponents.set(filePath, componentData)
      this.serverActions.delete(filePath)
    }

    const relativeBundlePath = path.join(this.options.rscDir, `${componentId}.js`)
    const fullBundlePath = path.join(this.options.outDir, relativeBundlePath)

    const cached = this.buildCache.get(filePath)
    const fileStats = await fs.promises.stat(filePath)
    const fileTimestamp = fileStats.mtimeMs

    if (
      cached &&
      cached.timestamp >= fileTimestamp &&
      JSON.stringify(cached.sourceDependencies) === JSON.stringify(sourceDependencies)
    ) {
      const storedComponent =
        this.serverActions.get(filePath) ?? this.serverComponents.get(filePath)
      if (storedComponent && cached.bundledDependencies.length > 0)
        storedComponent.dependencies = cached.bundledDependencies

      await fs.promises.writeFile(fullBundlePath, cached.code, 'utf-8')
      await this.updateManifestForComponent(componentId, filePath, relativeBundlePath, cached.css)
      return {
        componentId,
        bundlePath: this.bundlePathForRuntime(fullBundlePath),
        success: true,
      }
    }

    const bundleDir = path.dirname(fullBundlePath)
    await fs.promises.mkdir(bundleDir, { recursive: true })

    const result = await this.emitRscEntries([{ componentId, filePath }], {
      minify: this.options.minify,
    })
    const built = result.outputs.get(componentId)
    if (built == null) {
      throw new Error(`Vite RSC environment build missed entry: ${componentId}`)
    }

    await fs.promises.writeFile(fullBundlePath, built.code, 'utf-8')

    const cssSources = this.resolveServerCssSources(filePath, code)
    const css = [
      ...(await this.writeComponentCssAsset(
        componentId,
        built.cssAssetSources.length > 0 ? [...built.cssAssetSources] : cssSources,
      )),
    ]

    for (const file of result.extraFiles) {
      const fullPath = this.resolveExtraFileOutPath(file.fileName)
      await fs.promises.mkdir(path.dirname(fullPath), { recursive: true })
      await fs.promises.writeFile(fullPath, file.code)
    }

    const storedComponent = this.serverActions.get(filePath) ?? this.serverComponents.get(filePath)
    this.buildCache.set(filePath, {
      code: built.code,
      css,
      timestamp: Date.now(),
      sourceDependencies,
      bundledDependencies: storedComponent?.dependencies ?? sourceDependencies,
    })

    await this.updateManifestForComponent(componentId, filePath, relativeBundlePath, css)

    return {
      componentId,
      bundlePath: this.bundlePathForRuntime(fullBundlePath),
      success: true,
    }
  }

  private bundlePathForRuntime(absoluteBundlePath: string): string {
    return toPosixPath(path.relative(this.projectRoot, absoluteBundlePath))
  }

  private manifestCache: ServerComponentManifest | null = null

  async updateManifestForComponent(
    componentId: string,
    filePath: string,
    bundlePath: string,
    css: readonly string[] = [],
  ): Promise<void> {
    const manifestPath = path.join(this.options.outDir, this.options.manifestPath)

    let manifest: ServerComponentManifest

    if (this.manifestCache) {
      manifest = this.manifestCache
    } else if (fs.existsSync(manifestPath)) {
      const content = await fs.promises.readFile(manifestPath, 'utf-8')
      const parsed = parseJsonRecord(content)
      if (parsed && isServerComponentManifestRecord(parsed)) {
        manifest = parsed
        this.manifestCache = manifest
      } else {
        manifest = {
          components: {},
          buildTime: new Date().toISOString(),
        }
        this.manifestCache = manifest
      }
    } else {
      manifest = {
        components: {},
        buildTime: new Date().toISOString(),
      }
      this.manifestCache = manifest
    }

    const componentData = this.serverComponents.get(filePath) ?? this.serverActions.get(filePath)
    const fullBundlePath = path.join(this.options.outDir, bundlePath)
    const moduleSpecifier = pathToFileURL(path.resolve(this.projectRoot, fullBundlePath)).href

    if (!componentData) {
      const code = await fs.promises.readFile(filePath, 'utf-8')
      manifest.components[componentId] = {
        id: componentId,
        filePath,
        relativePath: path.relative(this.projectRoot, filePath),
        bundlePath,
        moduleSpecifier,
        dependencies: this.extractDependencies(code, filePath),
        hasNodeImports: this.hasNodeImports(code, filePath),
        css,
      }
    } else {
      manifest.components[componentId] = {
        id: componentId,
        filePath,
        relativePath: path.relative(this.projectRoot, filePath),
        bundlePath,
        moduleSpecifier,
        dependencies: [...componentData.dependencies],
        hasNodeImports: componentData.hasNodeImports,
        css,
      }
    }

    manifest.buildTime = new Date().toISOString()

    await fs.promises.writeFile(manifestPath, JSON.stringify(manifest), 'utf-8')
    await this.writeRouteCssEntries(manifest)

    this.manifestCache = manifest
  }

  clearCache(): void {
    this.buildCache.clear()
    this.manifestCache = null
  }

  invalidateBuildCacheFor(filePath: string): void {
    this.buildCache.delete(filePath)
  }
}

export interface DirectoryScanResult {
  serverComponentPaths: string[]
  clientComponentPaths: string[]
}

interface ScannedFile {
  filePath: string
  cacheKey: string
  code: string
  analysis: ModuleAnalysis
}

function collectScannedFiles(
  builder: ServerComponentBuilder,
  dirs: readonly string[],
): ScannedFile[] {
  const files: ScannedFile[] = []

  for (const fullPath of collectSourceFilePaths(dirs)) {
    try {
      const cacheKey = resolveModuleCachePath(fullPath)
      const code = fs.readFileSync(fullPath, 'utf-8')
      const analysis = builder.getModuleAnalysis(fullPath, code)
      files.push({ filePath: fullPath, cacheKey, code, analysis })
    } catch (error) {
      console.warn(`[server-build] Error reading ${fullPath}:`, errorMessage(error, String(error)))
    }
  }

  return files
}

export function hasComponentExport(code: string, analysis?: ModuleAnalysis): boolean {
  const moduleAnalysis = analysis ?? analyzeModuleSource(code)
  return (
    moduleAnalysis.hasComponentExport ||
    EXPORTED_FUNCTION_REGEX.test(code) ||
    EXPORTED_DEFAULT_ARROW_REGEX.test(code) ||
    EXPORTED_CONST_FUNCTION_REGEX.test(code)
  )
}

export function isEligibleServerComponent(
  filePath: string,
  code: string,
  builder: ServerComponentBuilder,
  analysis?: ModuleAnalysis,
): boolean {
  const fileName = path.basename(filePath)
  if (
    SPECIAL_FILE_REGEX.test(fileName) ||
    APP_ICON_FILE_REGEX.test(fileName) ||
    fileName.endsWith('.d.ts')
  ) {
    return false
  }

  const moduleAnalysis = analysis ?? builder.getModuleAnalysis(filePath, code)

  if (moduleAnalysis.directives.hasUseClient) return false

  if (moduleAnalysis.directives.hasUseServer) return true

  if (builder.isOnlyImportedByClientComponents(filePath)) return false

  return (
    isServerComponentFromAnalysis(filePath, moduleAnalysis) &&
    hasComponentExport(code, moduleAnalysis)
  )
}

export function scanDirectory(
  dir: string,
  builder: ServerComponentBuilder,
  additionalDirs: readonly string[] = [],
): DirectoryScanResult {
  const dirs = normalizeScanDirs(dir, additionalDirs)

  const files = collectScannedFiles(builder, dirs)
  builder.clearClientComponentFiles()
  builder.populateImportGraphFromFiles(files)

  const serverComponentPaths: string[] = []
  const clientComponentPaths: string[] = []

  for (const { filePath, code, analysis } of files) {
    if (analysis.directives.hasUseClient) {
      clientComponentPaths.push(filePath)
      builder.recordClientComponent(filePath, code)
    }

    if (isEligibleServerComponent(filePath, code, builder, analysis)) {
      builder.addServerComponent(filePath, code, analysis)
      serverComponentPaths.push(filePath)
    }
  }

  return { serverComponentPaths, clientComponentPaths }
}

export function createServerBuildPlugin(options: ServerBuildOptions = {}): Plugin {
  let builder: ServerComponentBuilder | null = null
  let projectRoot: string
  let resolvedViteOutDir: string
  let resolvedAliases: Record<string, string> = {}
  let serverArtifactsEmitted = false

  async function emitPostBuildArtifacts(): Promise<void> {
    if (!builder) return

    try {
      await builder.buildMdxRegistry(options.mdx)
    } catch (error) {
      console.warn('[rari] Failed to build MDX component registry:', error)
    }

    try {
      const { generateRobotsFile } = await import('@/router/metadata/robots')
      await generateRobotsFile({
        appDir: path.join(projectRoot, 'src', 'app'),
        outDir: resolvedViteOutDir,
        aliases: resolvedAliases,
      })
    } catch (error) {
      console.warn('[rari] Failed to generate robots.txt:', error)
    }

    try {
      const { generateSitemapFiles } = await import('@/router/metadata/sitemap')
      await generateSitemapFiles({
        appDir: path.join(projectRoot, 'src', 'app'),
        outDir: resolvedViteOutDir,
        aliases: resolvedAliases,
      })
    } catch (error) {
      console.warn('[rari] Failed to generate sitemap:', error)
    }

    try {
      const { generateFeedFile } = await import('@/router/metadata/feed')
      await generateFeedFile({
        appDir: path.join(projectRoot, 'src', 'app'),
        outDir: resolvedViteOutDir,
        aliases: resolvedAliases,
      })
    } catch (error) {
      console.warn('[rari] Failed to generate feed:', error)
    }

    try {
      const routesPath = path.join(resolvedViteOutDir, 'server', 'routes.json')
      if (fs.existsSync(routesPath)) {
        const icons = parseAppIconsFromManifest(fs.readFileSync(routesPath, 'utf-8'))
        if (icons.length > 0) {
          await copyAppIconsToOutDir({
            appDir: path.join(projectRoot, 'src', 'app'),
            outDir: resolvedViteOutDir,
            icons,
          })
        }
      }
    } catch (error) {
      console.warn('[rari] Failed to copy app icons:', error)
    }

    try {
      const mdxOpts = resolveMdxPluginOptions(projectRoot, options.mdx)
      const contentDirs = collectMdxContentDirs(projectRoot, mdxOpts.contentDirs).filter(dir => {
        const rel = toPosixPath(path.relative(projectRoot, dir))
        if (rel === 'public/content' || rel.startsWith('public/content/')) return false
        if (rel === 'dist/content' || rel.startsWith('dist/content/')) return false
        return true
      })
      if (contentDirs.length > 0) {
        copyMdxContentDirsToDest(contentDirs, path.join(resolvedViteOutDir, 'content'))
      }
    } catch (error) {
      console.warn('[rari] Failed to copy MDX content:', error)
    }

    finalizeStaticImageSourceMapBuild(resolvedViteOutDir)
  }

  return {
    name: 'rari-server-build',
    sharedDuringBuild: true,

    configResolved(config) {
      projectRoot = config.root
      resolvedViteOutDir = path.resolve(config.root, config.build.outDir)
      serverArtifactsEmitted = false

      const alias = readViteAliases(config)
      resolvedAliases = alias
      const assetsDir =
        typeof config.build.assetsDir === 'string' && config.build.assetsDir !== ''
          ? normalizeAssetsDir(config.build.assetsDir)
          : 'assets'
      builder = new ServerComponentBuilder(projectRoot, {
        ...options,
        alias,
        assetsDir,
        outDir: resolvedViteOutDir,
        minify: options.minify ?? config.mode === 'production',
      })
    },

    buildStart() {
      if (!builder) return

      const srcDir = path.join(projectRoot, 'src')
      if (fs.existsSync(srcDir)) scanDirectory(srcDir, builder, Object.values(resolvedAliases))
    },

    async buildApp(viteBuilder) {
      if (!builder || serverArtifactsEmitted) return

      const clientEnv = viteBuilder.environments.client
      if (!clientEnv.isBuilt) await viteBuilder.build(clientEnv)

      const rscEnv = viteBuilder.environments.rsc
      const ssrEnv = viteBuilder.environments.ssr
      await rscEnv.init()
      await ssrEnv.init()

      builder.setViteBuilder(viteBuilder)
      await builder.buildServerComponents()

      await builder.buildSSRClientComponents()

      serverArtifactsEmitted = true
      await emitPostBuildArtifacts()
    },
  }
}
