/* oxlint-disable typescript/prefer-readonly-parameter-types -- SSR build mutates manifest and css module buffers */
import type { Plugin } from 'vite-plus'
import type { ModuleAnalysis } from '../analysis/directives'
import type { MdxPluginOptions } from '../mdx/registry'
import type {
  ServerCacheConfig,
  ServerCacheControlConfig,
  ServerCacheLayerConfig,
  ServerConfig,
  ServerCSPConfig,
} from './config'
import fs from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { build } from 'rolldown'
import {
  BACKSLASH_REGEX,
  EXPORTED_CONST_FUNCTION_REGEX,
  EXPORTED_DEFAULT_ARROW_REGEX,
  EXPORTED_FUNCTION_REGEX,
  FILE_PROTOCOL_REGEX,
  TSX_EXT_REGEX,
} from '@/shared/regex-constants'
import { resolveAlias } from '@/shared/utils/alias-resolver'
import {
  resolveIndexFile,
  resolveWithExtensions,
  resolveWithExtensionsAndIndex,
} from '@/shared/utils/file-resolver'
import { getErrnoCode, isRecord, parseJsonRecord } from '@/shared/utils/type-guards'
import {
  getReadableComponentId,
  getComponentId as getSharedComponentId,
  getProjectRelativePath as getSharedProjectRelativePath,
  hashString as sharedHashString,
} from '../analysis/component-ids'
import { analyzeModuleSource } from '../analysis/directives'
import {
  filterExternalDependencies,
  filterRelativeImportSources,
  hasNodeImportsFromAnalysis,
  isNodeBuiltinModule,
  ModuleAnalysisCache,
  resolveModuleCachePath,
} from '../analysis/module-cache'
import { collectSourceFilePaths, normalizeScanDirs } from '../analysis/source-walker'
import { resolveMdxRegistryEntries } from '../mdx/registry'
import { parseHtmlEntryImports } from '../transform/html-entry'
import { getUseCacheTransform } from '../transform/use-cache'

const COMPONENT_IMPORT_REGEX = /import\s+(\w+)\s+from\s+['"]([^'"]+)['"]/g
const CLIENT_IMPORT_REGEX = /import\s+(?:(\w+)|\{([^}]+)\})\s+from\s+['"]([^'"]+)['"];?\s*$/gm
const PROXY_FILE_REGEX = /^proxy\.(?:tsx?|jsx?|mts|mjs)$/
const COMPONENTS_PATH_REGEX = /\/components\/(\w+)(?:\.tsx?|\.jsx?)?$/
const COMPONENTS_PATH_ALT_REGEX = /[/\\]components[/\\](\w+)(?:\.tsx?|\.jsx?)?$/
const SPECIAL_FILE_REGEX = /^(?:robots|sitemap|feed)\.(?:tsx?|jsx?)$/
const RSC_REFERENCES_IMPORT = 'react-server-dom-rari/server'
const NODE_PROTOCOL_REGEX = /^node:/
export const RARI_CSS_MODULES_PATTERN = '[hash]_[local]'

/** Bare package name including scope (e.g. `markdown-it`, `@scope/pkg`). */
function barePackageName(source: string): string {
  if (source.startsWith('@')) {
    const parts = source.split('/')
    return parts.length >= 2 ? `${parts[0]}/${parts[1]}` : source
  }

  return source.split('/')[0] ?? source
}

/**
 * True when BYONM/Node would find the package by walking `node_modules` from
 * the app root. Avoid createRequire here -- pnpm bin shims set NODE_PATH, which
 * is baked into Module.globalPaths and makes transitive workspace deps look
 * like app installs (then incorrectly get externalized).
 */
function isInstalledFromAppRoot(projectRoot: string, source: string): boolean {
  const packageName = barePackageName(source)
  let dir = projectRoot
  for (;;) {
    const candidate = path.join(dir, 'node_modules', ...packageName.split('/'))
    if (fs.existsSync(path.join(candidate, 'package.json'))) return true

    const parent = path.dirname(dir)
    if (parent === dir) return false
    dir = parent
  }
}

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

function aliasRootForPath(filePath: string, projectRoot: string): string {
  if (isRariInternalPath(filePath)) return path.join(RARI_PACKAGE_ROOT, 'src')

  return path.join(projectRoot, 'src')
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
  const normalized = filePath.replace(BACKSLASH_REGEX, '/')
  return (
    normalized.includes('ErrorBoundaryWrapper') ||
    normalized.includes('/boundaries/error-boundary-wrapper') ||
    normalized.endsWith('/error-boundary-wrapper.tsx')
  )
}

function ssrClientComponentId(filePath: string, projectRoot: string): string {
  if (isErrorBoundaryWrapperPath(filePath)) return 'virtual:error-boundary-wrapper.tsx'

  const relativePath = path.relative(projectRoot, filePath).replace(BACKSLASH_REGEX, '/')
  if (relativePath.startsWith('..') || path.isAbsolute(relativePath))
    return filePath.replace(BACKSLASH_REGEX, '/')

  return relativePath
}

function ssrClientBundleName(filePath: string, projectRoot: string): string {
  if (isErrorBoundaryWrapperPath(filePath))
    return `error_boundary_wrapper_${sharedHashString('virtual:error-boundary-wrapper.tsx')}`

  if (isRariInternalPath(filePath)) {
    const relative = path.relative(RARI_PACKAGE_ROOT, filePath).replace(BACKSLASH_REGEX, '/')
    return `${getReadableComponentId(relative)}_${sharedHashString(relative)}`
  }

  return getSharedComponentId(filePath, projectRoot)
}

let lightningcssTransform: typeof import('lightningcss').transform | null = null

async function getLightningcssTransform() {
  if (!lightningcssTransform) {
    const mod = await import('lightningcss')
    lightningcssTransform = mod.transform
  }

  return lightningcssTransform
}

interface BuiltComponent {
  code: string
  css: string[]
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

export interface ServerBuildOptions {
  readonly outDir?: string
  readonly rscDir?: string
  readonly manifestPath?: string
  readonly serverConfigPath?: string
  readonly minify?: boolean
  readonly alias?: Readonly<Record<string, string>>
  readonly define?: Readonly<Record<string, string>>
  readonly csp?: ServerCSPConfig
  readonly cacheControl?: ServerCacheControlConfig
  readonly cache?: ServerCacheConfig
  readonly jsPoolSize?: number
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
    | 'jsPoolSize'
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
  jsPoolSize?: ServerBuildOptions['jsPoolSize']
  htmlLimitedBots?: ServerBuildOptions['htmlLimitedBots']
  define?: ServerBuildOptions['define']
  experimental?: ServerBuildOptions['experimental']
  moduleAnalysisCache?: ModuleAnalysisCache
  mdx?: ServerBuildOptions['mdx']
}

export function isServerComponentFromAnalysis(
  filePath: string,
  analysis: ModuleAnalysis,
  htmlOnlyImports: ReadonlySet<string>,
  cacheKey?: string,
): boolean {
  if (filePath.includes('node_modules')) return false

  if (htmlOnlyImports.has(cacheKey ?? resolveModuleCachePath(filePath))) return false

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

  private readonly htmlOnlyImports = new Set<string>()
  private readonly fileImporters = new Map<string, Set<string>>()
  private readonly moduleAnalysisCache: ModuleAnalysisCache
  private readonly discoveredExternalClientComponents = new Set<string>()
  private readonly clientComponentFiles = new Map<string, string>()

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

  getHtmlOnlyImports(): ReadonlySet<string> {
    return new Set(this.htmlOnlyImports)
  }

  private async writeComponentCssAsset(
    componentId: string,
    cssModules: string[],
  ): Promise<string[]> {
    if (cssModules.length === 0) return []

    const assetsDir = path.join(this.options.outDir, 'assets', 'server')
    await fs.promises.mkdir(assetsDir, { recursive: true })

    const cssContent = `${cssModules.join('\n')}\n`
    const cssFileName = `${sharedHashString(componentId + cssContent, 12)}.css`
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
    this.projectRoot = projectRoot
    this.moduleAnalysisCache = options.moduleAnalysisCache ?? new ModuleAnalysisCache()
    const rscDir = options.rscDir != null && options.rscDir !== '' ? options.rscDir : 'server'
    this.options = {
      outDir:
        options.outDir != null && options.outDir !== ''
          ? options.outDir
          : path.join(projectRoot, 'dist'),
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
      define: options.define,
      csp: options.csp,
      cacheControl: options.cacheControl,
      cache: options.cache,
      jsPoolSize: options.jsPoolSize,
      htmlLimitedBots: options.htmlLimitedBots,
      experimental: options.experimental,
      mdx: options.mdx,
    }

    this.parseHtmlImports()
  }

  private parseHtmlImports() {
    for (const importPath of parseHtmlEntryImports(this.projectRoot))
      this.htmlOnlyImports.add(importPath)
  }

  getModuleAnalysis(filePath: string, source?: string): ModuleAnalysis {
    return this.moduleAnalysisCache.get(filePath, source)
  }

  isServerComponent(filePath: string, source?: string): boolean {
    try {
      const analysis = this.moduleAnalysisCache.get(filePath, source)
      return isServerComponentFromAnalysis(filePath, analysis, this.htmlOnlyImports)
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

    if (!isServerComponentFromAnalysis(filePath, moduleAnalysis, this.htmlOnlyImports)) return

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

  async getTransformedComponentsForDevelopment(): Promise<
    Array<{ id: string; code: string; isAction: boolean }>
  > {
    const components: Array<{ id: string; code: string; isAction: boolean }> = []

    for (const [filePath] of this.serverComponents) {
      const relativePath = path.relative(this.projectRoot, filePath)
      const componentId = this.getComponentId(relativePath)

      const transformedCode = await this.buildComponentCodeOnly(filePath)

      components.push({
        id: componentId,
        code: transformedCode,
        isAction: false,
      })
    }

    for (const [filePath] of this.serverActions) {
      const relativePath = path.relative(this.projectRoot, filePath)
      const actionId = this.getComponentId(relativePath)

      const transformedCode = await this.buildComponentCodeOnly(filePath)

      components.push({
        id: actionId,
        code: transformedCode,
        isAction: true,
      })
    }

    return components
  }

  private transformComponentImportsToGlobal(code: string): string {
    const replacements: Array<{ original: string; replacement: string }> = []

    for (const match of code.matchAll(COMPONENT_IMPORT_REGEX)) {
      const [fullMatch, importName, importPath] = match

      if (
        !importPath.startsWith('.') &&
        !importPath.startsWith('@') &&
        !importPath.startsWith('~') &&
        !importPath.startsWith('#')
      )
        continue

      let resolvedPath: string | null = null

      if (importPath.startsWith('.')) {
        if (importPath.includes('/components/')) {
          const componentMatch = COMPONENTS_PATH_REGEX.exec(importPath)
          if (componentMatch) {
            const componentName = componentMatch[1]

            const possiblePaths = [
              path.resolve(this.projectRoot, 'src', 'components', `${componentName}.tsx`),
              path.resolve(this.projectRoot, 'src', 'components', `${componentName}.ts`),
              path.resolve(this.projectRoot, 'src', 'components', `${componentName}.jsx`),
              path.resolve(this.projectRoot, 'src', 'components', `${componentName}.js`),
            ]

            let isClient = false
            for (const possiblePath of possiblePaths) {
              if (fs.existsSync(possiblePath) && this.isClientComponent(possiblePath)) {
                isClient = true
                break
              }
            }

            if (!isClient) continue

            const replacement = `// Component reference: ${componentName}
const ${importName} = (props) => {
  let Component = globalThis['~clientComponents']?.['components/${componentName}']?.component
    || globalThis['components/${componentName}'];

  if (Component && typeof Component === 'object' && Component.default) {
    Component = Component.default;
  }

  if (!Component) {
    throw new Error('Component components/${componentName} not loaded');
  }

  if (typeof Component !== 'function') {
    throw new Error('Component components/${componentName} is not a function, got: ' + typeof Component);
  }

  return Component(props);
}`
            replacements.push({ original: fullMatch, replacement })
          }
        }
        continue
      }

      const aliases = this.options.alias
      for (const [alias, replacement] of Object.entries(aliases)) {
        if (importPath.startsWith(`${alias}/`) || importPath === alias) {
          const relativePath = importPath.slice(alias.length).replace(/^\/+/, '')
          resolvedPath = path.join(replacement, relativePath)
          break
        }
      }

      if (resolvedPath != null && resolvedPath !== '') {
        const componentMatch = COMPONENTS_PATH_ALT_REGEX.exec(resolvedPath)
        if (componentMatch) {
          const componentName = componentMatch[1]

          const absolutePath = path.isAbsolute(resolvedPath)
            ? resolvedPath
            : path.resolve(this.projectRoot, resolvedPath)

          const possiblePaths = [
            absolutePath,
            `${absolutePath}.tsx`,
            `${absolutePath}.ts`,
            `${absolutePath}.jsx`,
            `${absolutePath}.js`,
          ]

          let isClient = false
          let actualPath = absolutePath
          for (const possiblePath of possiblePaths) {
            if (fs.existsSync(possiblePath)) {
              actualPath = possiblePath
              if (this.isClientComponent(possiblePath)) isClient = true
              break
            }
          }

          if (!isClient) continue

          const componentId = this.getComponentReferenceId(actualPath)

          const replacement = `// Component reference: ${componentName}
const ${importName} = (props) => {
  let Component = globalThis['~clientComponents']?.['${componentId}']?.component
    || globalThis['${componentId}'];

  if (Component && typeof Component === 'object' && Component.default) {
    Component = Component.default;
  }

  if (!Component) {
    throw new Error('Component ${componentId} not loaded');
  }

  if (typeof Component !== 'function') {
    throw new Error('Component ${componentId} is not a function, got: ' + typeof Component);
  }

  return Component(props);
}`
          replacements.push({ original: fullMatch, replacement })
        }
      }
    }

    let transformedCode = code
    for (const { original, replacement } of replacements)
      transformedCode = transformedCode.replace(original, replacement)

    return transformedCode
  }

  private isPageComponent(inputPath: string): boolean {
    return inputPath.includes('/app/') || inputPath.includes('\\app\\')
  }

  private createRolldownModuleInfoPlugin(filePath: string): Plugin {
    const externalDeps = new Set<string>()

    return {
      name: 'rari-rolldown-module-info',
      moduleParsed(moduleInfo) {
        for (const id of moduleInfo.importedIds) {
          if (
            !id.startsWith('.') &&
            !id.startsWith('/') &&
            !id.startsWith('node:') &&
            !isNodeBuiltinModule(id)
          )
            externalDeps.add(id)
        }

        for (const id of moduleInfo.dynamicallyImportedIds) {
          if (
            !id.startsWith('.') &&
            !id.startsWith('/') &&
            !id.startsWith('node:') &&
            !isNodeBuiltinModule(id)
          )
            externalDeps.add(id)
        }
      },
      buildEnd: () => {
        if (externalDeps.size === 0) return

        const dependencies = [...externalDeps].sort()
        const component = this.serverComponents.get(filePath) ?? this.serverActions.get(filePath)
        if (component) component.dependencies = dependencies

        const cached = this.buildCache.get(filePath)
        if (cached) cached.bundledDependencies = dependencies
      },
    }
  }

  private createBuildPlugins(
    virtualModuleId: string,
    transformedCode: string,
    loader: 'tsx' | 'jsx' | 'ts' | 'js',
    inputPath: string,
    isPage = false,
    cssModules?: string[],
  ) {
    const resolveDir = path.dirname(inputPath)
    const isProxyFile = PROXY_FILE_REGEX.test(path.basename(inputPath))

    const clientComponentRefs = new Map<string, string>()
    const serverActionRefs = new Map<string, { actionId: string; hasDefaultExport: boolean }>()

    return [
      {
        name: 'virtual-module',
        resolveId(id: string, importer: string | undefined) {
          if (id === virtualModuleId) return id

          if (importer === virtualModuleId && (id.startsWith('./') || id.startsWith('../'))) {
            if (id.endsWith('.module.css')) {
              return null
            }

            const resolved = path.resolve(resolveDir, id)
            const extensions = ['.ts', '.tsx', '.js', '.jsx', '']
            for (const ext of extensions) {
              const pathWithExt = resolved + ext
              if (fs.existsSync(pathWithExt) && fs.statSync(pathWithExt).isFile())
                return pathWithExt
            }
            for (const ext of ['.ts', '.tsx', '.js', '.jsx']) {
              const indexPath = path.join(resolved, `index${ext}`)
              if (fs.existsSync(indexPath)) return indexPath
            }

            return resolved
          }

          return null
        },
        load(id: string) {
          if (id === virtualModuleId) {
            return {
              code: transformedCode,
              moduleType: loader,
            }
          }

          return null
        },
      },
      {
        name: 'resolve-client-server-boundaries',
        enforce: 'pre' as const,
        resolveId: (source: string, importer: string | undefined) => {
          if (
            importer == null ||
            importer === '' ||
            importer.includes('node_modules') ||
            isRariInternalPath(importer)
          )
            return null

          if (
            source.startsWith('node:') ||
            isNodeBuiltinModule(source) ||
            source === 'react' ||
            source === 'react-dom' ||
            source === 'react/jsx-runtime' ||
            source === 'react/jsx-dev-runtime'
          ) {
            return null
          }

          let resolvedPath: string | null = null
          const aliases = this.options.alias

          resolvedPath = resolveAlias(source, aliases, this.projectRoot)

          if (
            (resolvedPath == null || resolvedPath === '') &&
            (source.startsWith('./') || source.startsWith('../'))
          ) {
            const importerDir = importer === virtualModuleId ? resolveDir : path.dirname(importer)
            resolvedPath = path.resolve(importerDir, source)
          }

          if ((resolvedPath == null || resolvedPath === '') && path.isAbsolute(source))
            resolvedPath = source

          if (resolvedPath !== '') {
            const extensions = ['', '.ts', '.tsx', '.js', '.jsx']
            for (const ext of extensions) {
              const pathWithExt = resolvedPath + ext
              if (fs.existsSync(pathWithExt) && fs.statSync(pathWithExt).isFile()) {
                if (this.isClientComponent(pathWithExt)) {
                  const relativePath = path.relative(this.projectRoot, pathWithExt)
                  const componentId = (
                    relativePath.startsWith('..') ? pathWithExt : relativePath
                  ).replace(BACKSLASH_REGEX, '/')
                  clientComponentRefs.set(pathWithExt, componentId)

                  if (relativePath.startsWith('..'))
                    this.discoveredExternalClientComponents.add(pathWithExt)

                  return { id: `\0client-ref:${pathWithExt}` }
                }

                try {
                  const analysis = this.moduleAnalysisCache.get(pathWithExt)
                  if (analysis.directives.hasUseServer) {
                    const actionId = this.getComponentId(pathWithExt)
                    serverActionRefs.set(pathWithExt, {
                      actionId,
                      hasDefaultExport: analysis.hasDefaultExport,
                    })
                    return { id: `\0server-action:${pathWithExt}` }
                  }
                } catch (error) {
                  console.error(
                    `[rari] Failed to read file for server action detection: ${pathWithExt}`,
                    error,
                  )
                }
                break
              }
            }
          }

          return null
        },
        load: (id: string) => {
          if (id.startsWith('\0client-ref:')) {
            const filePath = id.slice('\0client-ref:'.length)
            const relativePath = path.relative(this.projectRoot, filePath)
            const componentId = (
              clientComponentRefs.get(filePath) ??
              (relativePath.startsWith('..') ? filePath : relativePath)
            ).replace(BACKSLASH_REGEX, '/')

            return {
              code: `import { registerClientReference } from ${JSON.stringify(RSC_REFERENCES_IMPORT)};
export default registerClientReference(null, ${JSON.stringify(componentId)}, "default");
`,
              moduleType: 'js',
            }
          }

          if (id.startsWith('\0server-action:')) {
            const filePath = id.slice('\0server-action:'.length)
            const actionId =
              serverActionRefs.get(filePath)?.actionId ?? this.getComponentId(filePath)

            return {
              code: this.generateServerActionRuntimeModule(filePath, actionId),
              moduleType: 'js',
            }
          }

          return null
        },
      },
      {
        name: 'use-transformed-server-components',
        resolveId: (source: string, importer: string | undefined) => {
          if (!isPage) return null

          if (source.startsWith('file://')) {
            const filePath = source.replace(FILE_PROTOCOL_REGEX, '')
            if (fs.existsSync(filePath)) return { id: `\0transformed:${filePath}` }

            return null
          }

          let resolvedPath: string | null = null
          const aliases = this.options.alias

          resolvedPath = resolveAlias(source, aliases, this.projectRoot)

          const importerDir = importer?.startsWith('\0')
            ? resolveDir
            : importer != null && importer !== ''
              ? path.dirname(importer)
              : resolveDir
          if (
            (resolvedPath == null || resolvedPath === '') &&
            (source.startsWith('./') || source.startsWith('../'))
          )
            resolvedPath = path.resolve(importerDir, source)

          if (resolvedPath == null || resolvedPath === '') return null

          if (importerDir.includes('node_modules')) return null

          const extensions = ['', '.ts', '.tsx', '.js', '.jsx']
          for (const ext of extensions) {
            const pathWithExt = resolvedPath + ext
            if (fs.existsSync(pathWithExt) && fs.statSync(pathWithExt).isFile()) {
              if (this.isClientComponent(pathWithExt)) return null

              if (this.isServerActionFile(pathWithExt)) return null

              const srcDir = path.join(this.projectRoot, 'src')
              if (!pathWithExt.startsWith(srcDir)) return null

              const componentId = this.getComponentId(pathWithExt)
              const distPath = path.join(
                this.options.outDir,
                this.options.rscDir,
                `${componentId}.js`,
              )

              if (fs.existsSync(distPath)) return { id: `\0transformed:${distPath}` }

              break
            }
          }

          return null
        },
        load(id: string) {
          if (id.startsWith('\0transformed:')) {
            const filePath = id.slice('\0transformed:'.length)
            const contents = fs.readFileSync(filePath, 'utf-8')
            return {
              code: contents,
              moduleType: 'js',
            }
          }

          return null
        },
      },
      {
        name: 'resolve-aliases',
        resolveId: (source: string) => {
          if (source.startsWith('\0')) return null

          const resolved = resolveAlias(source, this.options.alias, this.projectRoot)
          if (resolved == null || resolved === '') return null

          const extensions = ['', '.ts', '.tsx', '.js', '.jsx']
          for (const ext of extensions) {
            const pathWithExt = resolved + ext
            if (fs.existsSync(pathWithExt) && fs.statSync(pathWithExt).isFile()) {
              if (this.isServerActionFile(pathWithExt)) {
                const actionId = this.getComponentId(pathWithExt)
                serverActionRefs.set(pathWithExt, {
                  actionId,
                  hasDefaultExport: this.moduleAnalysisCache.get(pathWithExt).hasDefaultExport,
                })
                return { id: `\0server-action:${pathWithExt}` }
              }

              return pathWithExt
            }
          }

          for (const ext of ['.ts', '.tsx', '.js', '.jsx']) {
            const indexPath = path.join(resolved, `index${ext}`)
            if (fs.existsSync(indexPath)) {
              if (this.isServerActionFile(indexPath)) {
                const actionId = this.getComponentId(indexPath)
                serverActionRefs.set(indexPath, {
                  actionId,
                  hasDefaultExport: this.moduleAnalysisCache.get(indexPath).hasDefaultExport,
                })
                return { id: `\0server-action:${indexPath}` }
              }

              return indexPath
            }
          }

          return resolved
        },
      },
      {
        name: 'resolve-rari-proxy',
        resolveId: (source: string) => {
          if (isProxyFile && source === 'rari') {
            const rariResponsePath = path.join(RARI_DIST_DIR, 'proxy/RariResponse.mjs')
            if (fs.existsSync(rariResponsePath)) return rariResponsePath

            const rariResponseSrcPath = path.join(RARI_PACKAGE_ROOT, 'src/proxy/http/response.ts')
            if (fs.existsSync(rariResponseSrcPath)) return rariResponseSrcPath
          }

          return null
        },
      },
      {
        name: 'css-modules',
        resolveId: (source: string, importer: string | undefined) => {
          if (source.endsWith('.module.css')) {
            const importerDir =
              importer != null && importer !== '' && !importer.startsWith('\0')
                ? path.dirname(importer)
                : resolveDir
            const resolved = path.resolve(importerDir, source)

            if (fs.existsSync(resolved)) {
              return { id: `\0css-module:${resolved}` }
            }
          }

          return null
        },
        load: async (id: string) => {
          const CSS_MODULE_PREFIX = '\0css-module:'
          if (!id.startsWith(CSS_MODULE_PREFIX)) {
            return null
          }

          const filePath = id.slice(CSS_MODULE_PREFIX.length)

          try {
            const transform = await getLightningcssTransform()
            const code = fs.readFileSync(filePath)
            const result = transform({
              filename: path.relative(this.projectRoot, filePath),
              code,
              cssModules: { pattern: RARI_CSS_MODULES_PATTERN },
            })

            if (cssModules) cssModules.push(new TextDecoder().decode(result.code))

            const classes: Record<string, string> = {}
            if (result.exports) {
              for (const [key, value] of Object.entries(result.exports)) {
                classes[key] = value.name
              }
            }

            return { code: `export default ${JSON.stringify(classes)}`, moduleType: 'js' }
          } catch (e) {
            throw new Error(
              `[rari] Failed to process CSS module ${id}: ${e instanceof Error ? e.message : String(e)}`,
            )
          }
        },
      } satisfies Plugin,
      {
        name: 'externalize-deps',
        resolveId: (source: string, importer: string | undefined) => {
          if (source.startsWith('\0')) return null

          if (source.startsWith('node:') || isNodeBuiltinModule(source))
            return { id: source, external: true }

          const externalPackages = [
            'react',
            'react-dom',
            'react/jsx-runtime',
            'react/jsx-dev-runtime',
            'rari/image',
          ]

          if (externalPackages.includes(source)) return { id: source, external: true }

          const externalPackageMappings: Record<string, string | null> = {
            'rari/runtime/cache-wrapper': 'node_modules/rari/dist/runtime/cache-wrapper.mjs',
            'react-server-dom-rari/server': 'node_modules/rari/dist/runtime/rsc-references.mjs',
          }

          if (source in externalPackageMappings) {
            return { id: source, external: true }
          }

          if (source === 'rari') return null

          if (resolveAlias(source, this.options.alias, this.projectRoot) != null) return null

          if (!source.startsWith('.') && !source.startsWith('/')) {
            // App-visible installs stay external for runtime BYONM. Deps that only
            // exist under a workspace package (e.g. markdown-it in shared/) must be
            // force-resolved and bundled -- Rolldown's platform:'node' otherwise
            // leaves them as unresolved externals.
            if (isInstalledFromAppRoot(this.projectRoot, source))
              return { id: source, external: true }

            const resolveFrom = [
              importer != null && importer !== '' && !importer.startsWith('\0') ? importer : null,
              inputPath,
            ].filter((value): value is string => Boolean(value))

            for (const from of resolveFrom) {
              try {
                return { id: createRequire(from).resolve(source) }
              } catch {
                try {
                  const resolved = import.meta.resolve(source, pathToFileURL(from).href)
                  return { id: fileURLToPath(resolved) }
                } catch {
                  // try next candidate
                }
              }
            }

            return null
          }

          return null
        },
      },
      this.createRolldownModuleInfoPlugin(inputPath),
      {
        name: 'use-cache',
        transform: async (code: string, id: string) => {
          if (!this.options.experimental?.useCache && !this.options.experimental?.useCacheRemote)
            return null

          const transform = await getUseCacheTransform()
          if (!transform) {
            return null
          }

          return transform(code, id, {
            hashSalt: `${this.useCacheBuildId ?? 'development'}:rari-use-cache-v1`,
          })
        },
      },
    ]
  }

  private async buildComponentCodeOnly(inputPath: string): Promise<string> {
    const originalCode = await fs.promises.readFile(inputPath, 'utf-8')
    const clientTransformedCode = this.transformClientImports(originalCode, inputPath)
    const isPage = this.isPageComponent(inputPath)
    const transformedCode = isPage
      ? this.transformComponentImportsToGlobal(clientTransformedCode)
      : clientTransformedCode

    const ext = path.extname(inputPath)
    let loader: 'tsx' | 'jsx' | 'ts' | 'js'
    if (ext === '.tsx') loader = 'tsx'
    else if (ext === '.ts') loader = 'ts'
    else if (ext === '.jsx') loader = 'jsx'
    else loader = 'js'

    const virtualModuleId = `\0virtual:${inputPath}`

    const result = await build({
      input: virtualModuleId,
      platform: 'node',
      write: false,
      external: [
        NODE_PROTOCOL_REGEX,
        'react',
        'react-dom',
        'react/jsx-runtime',
        'react/jsx-dev-runtime',
      ],
      output: {
        format: 'esm',
        minify: this.options.minify,
      },
      moduleTypes: {
        [`.${loader}`]: loader,
      },
      resolve: {
        mainFields: ['module', 'main'],
        conditionNames: ['import', 'module', 'default'],
        extensions: ['.ts', '.tsx', '.js', '.jsx'],
      },
      transform: {
        jsx: 'react',
        define: {
          'global': 'globalThis',
          'process.env.NODE_ENV': JSON.stringify(
            process.env.NODE_ENV != null && process.env.NODE_ENV !== ''
              ? process.env.NODE_ENV
              : 'production',
          ),
          ...this.options.define,
        },
      },
      plugins: this.createBuildPlugins(
        virtualModuleId,
        transformedCode,
        loader,
        inputPath,
        isPage,
        [],
      ),
    })

    if (result.output.length === 0) throw new Error('No output generated from Rolldown')

    const entryChunk = result.output.find(chunk => chunk.type === 'chunk' && chunk.isEntry)
    if (entryChunk?.type !== 'chunk') throw new Error('No entry chunk found in Rolldown output')

    let code = entryChunk.code

    const timestamp = new Date().toISOString()
    code = `// Built: ${timestamp}\n${code}`

    return code
  }

  private async buildComponentBatch(
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
    manifest: ServerComponentManifest,
    concurrency: number,
  ): Promise<void> {
    let active = 0
    let index = 0
    const errors: Error[] = []

    await new Promise<void>(resolve => {
      const next = () => {
        while (active < concurrency && index < entries.length) {
          const [filePath, component] = entries[index++]
          const relativePath = path.relative(this.projectRoot, filePath)
          const componentId = this.getComponentId(filePath)
          const bundlePath = path.join(this.options.rscDir, `${componentId}.js`)
          const fullBundlePath = path.join(this.options.outDir, bundlePath)

          active++
          void (async () => {
            try {
              const bundleDir = path.dirname(fullBundlePath)
              await fs.promises.mkdir(bundleDir, { recursive: true })

              const built = await this.buildSingleComponent(filePath, fullBundlePath)
              const css = await this.writeComponentCssAsset(componentId, built.css)

              const moduleSpecifier = pathToFileURL(
                path.resolve(this.projectRoot, fullBundlePath),
              ).href

              manifest.components[componentId] = {
                id: componentId,
                filePath,
                relativePath,
                bundlePath,
                moduleSpecifier,
                dependencies: [...component.dependencies],
                hasNodeImports: component.hasNodeImports,
                css,
              }
            } catch (error) {
              errors.push(error instanceof Error ? error : new Error(String(error)))
            } finally {
              active--
              if (index >= entries.length && active === 0) {
                resolve()
              } else {
                next()
              }
            }
          })()
        }

        if (entries.length === 0) resolve()
      }

      next()
    })

    if (errors.length > 0) throw errors[0]
  }

  async buildServerComponents(): Promise<ServerComponentManifest> {
    const serverOutDir = path.join(this.options.outDir, this.options.rscDir)

    await fs.promises.mkdir(serverOutDir, { recursive: true })

    const manifest: ServerComponentManifest = {
      components: {},
      buildTime: new Date().toISOString(),
    }

    const useCacheEnabled =
      this.options.experimental?.useCache === true ||
      this.options.experimental?.useCacheRemote != null

    const concurrency = Math.min(8, Math.max(1, (await import('node:os')).cpus().length))

    const nonPageComponents = [...this.serverComponents.entries()].filter(
      ([filePath]) => !this.isPageComponent(filePath),
    )
    const pageComponents = [...this.serverComponents.entries()].filter(([filePath]) =>
      this.isPageComponent(filePath),
    )
    const actions = [...this.serverActions.entries()]

    await Promise.all([
      this.buildComponentBatch(nonPageComponents, manifest, concurrency),
      this.buildComponentBatch(actions, manifest, concurrency),
    ])

    await this.buildComponentBatch(pageComponents, manifest, concurrency)

    if (useCacheEnabled) {
      this.useCacheBuildId = sharedHashString(JSON.stringify(manifest.components), 16)
      manifest.useCacheBuildId = this.useCacheBuildId
    }

    const manifestPath = path.join(this.options.outDir, this.options.manifestPath)
    await fs.promises.writeFile(manifestPath, JSON.stringify(manifest), 'utf-8')
    await this.writeRouteCssEntries(manifest)

    const serverConfig: ServerConfig = {}
    if (this.options.csp) serverConfig.csp = this.options.csp
    if (this.options.cacheControl) serverConfig.cacheControl = this.options.cacheControl
    if (this.options.cache) serverConfig.cache = this.options.cache
    if (this.options.jsPoolSize != null) serverConfig.jsPoolSize = this.options.jsPoolSize
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

    if (clientFiles.length === 0) {
      const manifest: Record<
        string,
        { id: string; filePath: string; bundlePath: string; exports: string[] }
      > = {}
      await this.buildExternalClientComponents(manifest, new Map())
      await this.writeClientReferenceManifest(ssrOutDir, manifest)
      return
    }

    const clientModuleSpecifiers = new Map<string, string>()
    for (const { filePath } of clientFiles) {
      const bundleName = ssrClientBundleName(filePath, this.projectRoot)
      clientModuleSpecifiers.set(path.resolve(filePath), `file:///ssr/${bundleName}.js`)
    }

    const manifest: Record<
      string,
      { id: string; filePath: string; bundlePath: string; exports: string[] }
    > = {}
    const concurrency = Math.min(8, Math.max(1, (await import('node:os')).cpus().length))
    let active = 0
    let index = 0

    await new Promise<void>(resolve => {
      const next = () => {
        while (active < concurrency && index < clientFiles.length) {
          const { filePath, code } = clientFiles[index++]
          const componentId = ssrClientComponentId(filePath, this.projectRoot)
          const bundleName = ssrClientBundleName(filePath, this.projectRoot)
          const bundlePath = `ssr/${bundleName}.js`
          const fullBundlePath = path.join(this.options.outDir, bundlePath)

          active++
          void (async () => {
            try {
              const bundleDir = path.dirname(fullBundlePath)
              await fs.promises.mkdir(bundleDir, { recursive: true })

              await this.buildSSRSingleClient(filePath, fullBundlePath, clientModuleSpecifiers)

              const exports = this.extractExportNames(code)
              manifest[componentId] = {
                id: componentId,
                filePath,
                bundlePath,
                exports,
              }
            } catch (error) {
              console.warn(
                `[rari] SSR build failed for ${componentId}:`,
                error instanceof Error ? error.message : error,
              )
            } finally {
              active--
              if (index >= clientFiles.length && active === 0) resolve()
              else next()
            }
          })()
        }

        if (clientFiles.length === 0) resolve()
      }

      next()
    })

    await this.buildExternalClientComponents(manifest, clientModuleSpecifiers)

    await this.writeClientReferenceManifest(ssrOutDir, manifest)
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

  private async buildExternalClientComponents(
    manifest: {
      [key: string]: { id: string; filePath: string; bundlePath: string; exports: string[] }
    },
    clientModuleSpecifiers: Map<string, string>,
  ): Promise<void> {
    for (const {
      componentId,
      devSourceSegments,
      publishedExport,
      exports,
    } of EXTERNAL_CLIENT_COMPONENT_MANIFESTS) {
      const sourcePath = this.resolveExternalClientSourcePath(devSourceSegments, publishedExport)
      if (sourcePath == null || sourcePath === '') continue

      try {
        const bundleName = `external_${sharedHashString(componentId)}`
        const bundlePath = `ssr/${bundleName}.js`
        const fullBundlePath = path.join(this.options.outDir, bundlePath)
        await fs.promises.mkdir(path.dirname(fullBundlePath), { recursive: true })
        await this.buildSSRSingleClient(sourcePath, fullBundlePath, clientModuleSpecifiers)

        manifest[componentId] = {
          id: componentId,
          filePath: sourcePath,
          bundlePath,
          exports: [...exports],
        }
      } catch (error) {
        console.warn(
          `[rari] SSR build failed for ${componentId}:`,
          error instanceof Error ? error.message : error,
        )
      }
    }
  }

  private extractExportNames(code: string): string[] {
    const exports: string[] = []
    if (/export\s+default\b/.test(code)) exports.push('default')
    const namedExportRegex = /export\s+(?:async\s+)?(?:function|const|let|var|class)\s+(\w+)/g
    for (const m of code.matchAll(namedExportRegex)) {
      exports.push(m[1])
    }

    return exports.length > 0 ? exports : ['default']
  }

  private isServerActionFile(filePath: string): boolean {
    try {
      const code = fs.readFileSync(filePath, 'utf-8')
      return this.isServerAction(code, filePath)
    } catch {
      return false
    }
  }

  private generateServerActionRuntimeModule(filePath: string, actionId: string): string {
    const code = fs.readFileSync(filePath, 'utf-8')
    const exports = this.extractExportNames(code)
    const lines = [
      `const __rariActionMod = globalThis.__rari_rsc_require__(${JSON.stringify(actionId)});`,
      `if (!__rariActionMod) throw new Error("Server action module ${actionId} is not registered");`,
    ]

    for (const name of exports) {
      if (name === 'default') lines.push('export default __rariActionMod.default;')
      else lines.push(`export const ${name} = __rariActionMod.${name};`)
    }

    return `${lines.join('\n')}\n`
  }

  private generateServerActionReferenceModule(filePath: string): string {
    const code = fs.readFileSync(filePath, 'utf-8')
    const exports = this.extractExportNames(code)
    const actionId = this.getComponentId(filePath)

    let stub = `import { createServerReference } from 'react-server-dom-webpack/client';\n`
    stub += `import { callServer } from 'rari/runtime/call-server';\n`
    for (const name of exports) {
      const refId = `${actionId}#${name}`
      if (name === 'default')
        stub += `export default createServerReference(${JSON.stringify(refId)}, callServer);\n`
      else
        stub += `export const ${name} = createServerReference(${JSON.stringify(refId)}, callServer);\n`
    }

    return stub
  }

  private async buildSSRSingleClient(
    inputPath: string,
    outputPath: string,
    clientModuleSpecifiers?: Map<string, string>,
  ): Promise<{ code: string }> {
    const originalCode = await fs.promises.readFile(inputPath, 'utf-8')
    const strippedCode = originalCode.replace(/^['"]use client['"];?\s*/m, '')

    const ext = path.extname(inputPath)
    let loader: 'tsx' | 'jsx' | 'ts' | 'js'
    if (ext === '.tsx') loader = 'tsx'
    else if (ext === '.ts') loader = 'ts'
    else if (ext === '.jsx') loader = 'jsx'
    else loader = 'js'

    const virtualModuleId = `\0ssr-virtual:${inputPath}`
    const projectRoot = this.projectRoot
    const aliasRoot = aliasRootForPath(inputPath, projectRoot)
    const generateServerActionReferenceModule = (filePath: string) =>
      this.generateServerActionReferenceModule(filePath)
    const isServerActionFile = (filePath: string) => this.isServerActionFile(filePath)

    const result = await build({
      input: virtualModuleId,
      platform: 'node',
      write: false,
      external: [
        NODE_PROTOCOL_REGEX,
        'react',
        'react-dom',
        /^react\//,
        /^rari/,
        'react-server-dom-webpack/client',
        /^react-server-dom-webpack\//,
      ],
      output: {
        format: 'esm',
        minify: false,
      },
      moduleTypes: {
        [`.${loader}`]: loader,
      },
      resolve: {
        mainFields: ['module', 'main'],
        conditionNames: ['import', 'module', 'default'],
        extensions: ['.ts', '.tsx', '.js', '.jsx'],
      },
      transform: {
        jsx: 'react-jsx',
        define: {
          'global': 'globalThis',
          'process.env.NODE_ENV': JSON.stringify(
            process.env.NODE_ENV != null && process.env.NODE_ENV !== ''
              ? process.env.NODE_ENV
              : 'production',
          ),
        },
      },
      plugins: [
        {
          name: 'ssr-client-virtual',
          resolveId(id) {
            if (id === virtualModuleId) return id

            return null
          },
          load(id) {
            if (id === virtualModuleId) return { code: strippedCode, moduleType: loader }

            if (id.startsWith('\0server-action-ref:')) {
              const filePath = id.slice('\0server-action-ref:'.length)
              return {
                code: generateServerActionReferenceModule(filePath),
                moduleType: 'js',
              }
            }

            return null
          },
        },
        {
          name: 'ssr-client-resolve',
          resolveId(id, importer) {
            if (id.startsWith('\0server-action-ref:')) return id

            if (id.startsWith('.') || id.startsWith('/') || id.startsWith('@/')) {
              let resolved = id
              if (id.startsWith('@/')) {
                resolved = path.join(aliasRoot, id.slice(2))
              } else if (importer === virtualModuleId) {
                resolved = path.resolve(path.dirname(inputPath), id)
              } else if (importer != null && importer !== '') {
                resolved = path.resolve(
                  path.dirname(
                    importer.replace('\0ssr-virtual:', '').replace('\0server-action-ref:', ''),
                  ),
                  id,
                )
              }

              const foundByExt = resolveWithExtensions(resolved, [
                '.mjs',
                '.mts',
                '.ts',
                '.tsx',
                '.js',
                '.jsx',
              ])
              const found =
                foundByExt != null && foundByExt !== ''
                  ? foundByExt
                  : resolveIndexFile(resolved, ['.mjs', '.mts', '.ts', '.tsx', '.js', '.jsx'])

              if (found != null && found !== '' && isServerActionFile(found))
                return `\0server-action-ref:${path.resolve(found)}`

              if (found != null && found !== '' && clientModuleSpecifiers != null) {
                const resolvedAbs = path.resolve(found)
                const specifier = clientModuleSpecifiers.get(resolvedAbs)
                if (
                  specifier != null &&
                  specifier !== '' &&
                  resolvedAbs !== path.resolve(inputPath)
                )
                  return { id: specifier, external: true }
              }

              return found != null && found !== '' ? found : null
            }

            return null
          },
        },
      ],
    })

    const output = result.output[0]
    const code = typeof output === 'object' && 'code' in output ? output.code : ''

    await fs.promises.writeFile(outputPath, code, 'utf-8')
    return { code }
  }

  private async buildSingleComponent(
    inputPath: string,
    outputPath: string,
  ): Promise<BuiltComponent> {
    const originalCode = await fs.promises.readFile(inputPath, 'utf-8')
    const clientTransformedCode = this.transformClientImports(originalCode, inputPath)
    const isPage = this.isPageComponent(inputPath)
    const transformedCode = isPage
      ? this.transformComponentImportsToGlobal(clientTransformedCode)
      : clientTransformedCode

    const ext = path.extname(inputPath)
    let loader: 'tsx' | 'jsx' | 'ts' | 'js'
    if (ext === '.tsx') loader = 'tsx'
    else if (ext === '.ts') loader = 'ts'
    else if (ext === '.jsx') loader = 'jsx'
    else loader = 'js'

    const virtualModuleId = `\0virtual:${inputPath}`
    const cssModules: string[] = []

    const result = await build({
      input: virtualModuleId,
      platform: 'node',
      write: false,
      external: [
        NODE_PROTOCOL_REGEX,
        'react',
        'react-dom',
        'react/jsx-runtime',
        'react/jsx-dev-runtime',
      ],
      output: {
        format: 'esm',
        minify: this.options.minify,
      },
      moduleTypes: {
        [`.${loader}`]: loader,
      },
      resolve: {
        mainFields: ['module', 'main'],
        conditionNames: ['import', 'module', 'default'],
        extensions: ['.ts', '.tsx', '.js', '.jsx'],
      },
      transform: {
        jsx: 'react',
        define: {
          'global': 'globalThis',
          'process.env.NODE_ENV': JSON.stringify(
            process.env.NODE_ENV != null && process.env.NODE_ENV !== ''
              ? process.env.NODE_ENV
              : 'production',
          ),
          ...this.options.define,
        },
      },
      plugins: this.createBuildPlugins(
        virtualModuleId,
        transformedCode,
        loader,
        inputPath,
        isPage,
        cssModules,
      ),
    })

    if (result.output.length === 0) throw new Error('No output generated from Rolldown')

    const entryChunk = result.output.find(chunk => chunk.type === 'chunk' && chunk.isEntry)
    if (entryChunk?.type !== 'chunk') throw new Error('No entry chunk found in Rolldown output')

    let code = entryChunk.code

    const timestamp = new Date().toISOString()
    code = `// Built: ${timestamp}\n${code}`

    await fs.promises.writeFile(outputPath, code, 'utf-8')

    const fd = await fs.promises.open(outputPath, 'r+')
    await fd.sync()
    await fd.close()

    return { code, css: cssModules }
  }

  private transformClientImports(code: string, inputPath: string): string {
    let transformedCode = code

    let match

    const replacements: Array<{ original: string; replacement: string }> = []
    let hasClientComponents = false

    const externalClientComponents = EXTERNAL_CLIENT_COMPONENT_MANIFESTS.map(
      entry => entry.componentId,
    )

    CLIENT_IMPORT_REGEX.lastIndex = 0
    for (;;) {
      match = CLIENT_IMPORT_REGEX.exec(code)
      if (match === null) break

      const [fullMatch, defaultImport, namedImports, importPath] = match

      let isClientComponent = false
      let componentId = importPath

      if (externalClientComponents.includes(importPath)) {
        isClientComponent = true
      } else {
        const resolvedPath = this.resolveImportPath(importPath, inputPath)
        if (this.isClientComponent(resolvedPath)) {
          isClientComponent = true
          componentId = path.relative(this.projectRoot, resolvedPath).replace(BACKSLASH_REGEX, '/')
        }
      }

      if (isClientComponent) {
        hasClientComponents = true

        let replacement = ''

        if (defaultImport) {
          replacement = `const ${defaultImport} = registerClientReference(
  null,
  ${JSON.stringify(componentId)},
  "default"
);`
        } else if (namedImports) {
          const imports = namedImports.split(',').map(imp => imp.trim())
          const registrations = imports
            .map(imp => {
              const [importName, alias] = imp.includes(' as ')
                ? imp.split(' as ').map(s => s.trim())
                : [imp, imp]

              return `const ${alias} = registerClientReference(
  null,
  ${JSON.stringify(componentId)},
  ${JSON.stringify(importName)}
);`
            })
            .join('\n')

          replacement = registrations
        }

        replacements.push({ original: fullMatch, replacement })
      }
    }

    if (hasClientComponents) {
      transformedCode = `import { registerClientReference } from ${JSON.stringify(RSC_REFERENCES_IMPORT)};\n\n${transformedCode}`
    }

    for (const { original, replacement } of replacements)
      transformedCode = transformedCode.replace(original, replacement)

    return transformedCode
  }

  private resolveImportPath(importPath: string, importerPath: string): string {
    let resolvedPath = importPath
    const aliases = this.options.alias

    for (const [alias, replacement] of Object.entries(aliases)) {
      if (importPath.startsWith(`${alias}/`) || importPath === alias) {
        const relativePath = importPath.slice(alias.length).replace(/^\/+/, '')
        resolvedPath = path.join(replacement, relativePath)
        break
      }
    }

    if (!path.isAbsolute(resolvedPath))
      resolvedPath = path.resolve(path.dirname(importerPath), resolvedPath)

    const extensions = ['.tsx', '.jsx', '.ts', '.js']
    const withExt = resolveWithExtensions(resolvedPath, extensions)
    if (withExt != null && withExt !== '') return withExt

    const indexFile = resolveIndexFile(resolvedPath, extensions)
    if (indexFile != null && indexFile !== '') return indexFile

    return `${resolvedPath}.tsx`
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
        bundlePath: path.join(this.options.outDir, relativeBundlePath),
        success: true,
      }
    }

    const bundleDir = path.dirname(fullBundlePath)
    await fs.promises.mkdir(bundleDir, { recursive: true })

    const built = await this.buildSingleComponent(filePath, fullBundlePath)
    const css = await this.writeComponentCssAsset(componentId, built.css)

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
      bundlePath: path.join(this.options.outDir, relativeBundlePath),
      success: true,
    }
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

  async getTransformedComponentCode(filePath: string): Promise<string> {
    return this.buildComponentCodeOnly(filePath)
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
      console.warn(
        `[server-build] Error reading ${fullPath}:`,
        error instanceof Error ? error.message : error,
      )
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
  cacheKey?: string,
): boolean {
  const fileName = path.basename(filePath)
  if (SPECIAL_FILE_REGEX.test(fileName) || fileName.endsWith('.d.ts')) return false

  const moduleAnalysis = analysis ?? builder.getModuleAnalysis(filePath, code)

  if (moduleAnalysis.directives.hasUseClient) return false

  if (moduleAnalysis.directives.hasUseServer) return true

  if (builder.isOnlyImportedByClientComponents(filePath)) return false

  return (
    isServerComponentFromAnalysis(
      filePath,
      moduleAnalysis,
      builder.getHtmlOnlyImports(),
      cacheKey,
    ) && hasComponentExport(code, moduleAnalysis)
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

  for (const { filePath, cacheKey, code, analysis } of files) {
    if (analysis.directives.hasUseClient) {
      clientComponentPaths.push(filePath)
      builder.recordClientComponent(filePath, code)
    }

    if (isEligibleServerComponent(filePath, code, builder, analysis, cacheKey)) {
      builder.addServerComponent(filePath, code, analysis)
      serverComponentPaths.push(filePath)
    }
  }

  return { serverComponentPaths, clientComponentPaths }
}

export function createServerBuildPlugin(options: ServerBuildOptions = {}): Plugin {
  let builder: ServerComponentBuilder | null = null
  let projectRoot: string
  let isDev = false
  let resolvedAliases: Record<string, string> = {}

  return {
    name: 'rari-server-build',

    configResolved(config) {
      projectRoot = config.root
      isDev = config.command === 'serve'

      const excludeAliases = new Set([
        'react',
        'react-dom',
        'react/jsx-runtime',
        'react/jsx-dev-runtime',
        'react-dom/client',
      ])

      const alias: Record<string, string> = {}
      const aliasConfig = config.resolve.alias
      if (Array.isArray(aliasConfig)) {
        aliasConfig.forEach(entry => {
          if (
            typeof entry.find === 'string' &&
            typeof entry.replacement === 'string' &&
            !excludeAliases.has(entry.find)
          )
            alias[entry.find] = entry.replacement
        })
      } else if (typeof aliasConfig === 'object') {
        Object.entries(aliasConfig).forEach(([key, value]) => {
          if (typeof value === 'string' && !excludeAliases.has(key)) alias[key] = value
        })
      }

      resolvedAliases = alias
      builder = new ServerComponentBuilder(projectRoot, { ...options, alias })
    },

    buildStart() {
      if (!builder) return

      const isProduction = process.env.NODE_ENV === 'production'
      const cacheDirs = [
        path.join(projectRoot, 'dist', 'cache', 'og'),
        path.join(projectRoot, 'dist', 'cache', 'images'),
      ]

      if (isProduction) cacheDirs.push('/tmp/rari-og-cache', '/tmp/rari-image-cache')

      for (const dir of cacheDirs) {
        try {
          if (fs.existsSync(dir)) fs.rmSync(dir, { recursive: true, force: true })
        } catch (error) {
          console.warn(`[rari] Failed to clear cache ${dir}:`, error)
        }
      }

      const srcDir = path.join(projectRoot, 'src')
      if (fs.existsSync(srcDir)) scanDirectory(srcDir, builder, Object.values(resolvedAliases))
    },

    async closeBundle() {
      if (builder) {
        await builder.buildServerComponents()
        await builder.buildSSRClientComponents()

        try {
          await builder.buildMdxRegistry(options.mdx)
        } catch (error) {
          console.warn('[rari] Failed to build MDX component registry:', error)
        }

        try {
          const { generateRobotsFile } = await import('@/router/metadata/robots')
          await generateRobotsFile({
            appDir: path.join(projectRoot, 'src', 'app'),
            outDir: path.join(projectRoot, 'dist'),
          })
        } catch (error) {
          console.warn('[rari] Failed to generate robots.txt:', error)
        }

        try {
          const { generateSitemapFiles } = await import('@/router/metadata/sitemap')
          await generateSitemapFiles({
            appDir: path.join(projectRoot, 'src', 'app'),
            outDir: path.join(projectRoot, 'dist'),
            aliases: resolvedAliases,
          })
        } catch (error) {
          console.warn('[rari] Failed to generate sitemap:', error)
        }

        try {
          const { generateFeedFile } = await import('@/router/metadata/feed')
          await generateFeedFile({
            appDir: path.join(projectRoot, 'src', 'app'),
            outDir: path.join(projectRoot, 'dist'),
            aliases: resolvedAliases,
          })
        } catch (error) {
          console.warn('[rari] Failed to generate feed:', error)
        }
      }
    },

    async handleHotUpdate({ file }) {
      if (!builder || !isDev) return

      const relativePath = path.relative(projectRoot, file).replace(BACKSLASH_REGEX, '/')
      if (!relativePath.startsWith('src/') || !TSX_EXT_REGEX.test(relativePath)) return

      try {
        const content = await fs.promises.readFile(file, 'utf-8')
        const analysis = builder.getModuleAnalysis(file, content)
        const isTracked = builder.hasComponent(file)
        const eligible = isEligibleServerComponent(file, content, builder, analysis)

        if (!eligible) {
          if (isTracked) builder.removeComponent(file)

          return
        }

        if (!isTracked) builder.addServerComponent(file, content, analysis)

        await builder.rebuildComponent(file)
      } catch (error) {
        console.error(`[rari] Build: Error rebuilding ${relativePath}:`, error)
      }
    },
  }
}
