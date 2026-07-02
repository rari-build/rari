import type { Plugin } from 'vite-plus'
import type { ServerCacheConfig, ServerCacheControlConfig, ServerCacheLayerConfig, ServerConfig, ServerCSPConfig } from './server-config'
import fs from 'node:fs'
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
} from '../shared/regex-constants'
import { resolveAlias } from './alias-resolver'
import { getReadableComponentId, getComponentId as getSharedComponentId, getProjectRelativePath as getSharedProjectRelativePath, hashString as sharedHashString } from './component-ids'
import { getDirectives, hasDefaultExport, hasTopLevelUseClientDirective, hasTopLevelUseServerDirective } from './directives'
import { resolveIndexFile, resolveWithExtensions } from './file-resolver'
import { getUseCacheTransform } from './use-cache-loader'

const HTML_IMPORT_REGEX = /import\s*\(\s*["']([^"']+)["']\s*\)|import\s+["']([^"']+)["']/g
const CODE_IMPORT_REGEX = /from\s+['"]([^'"]+)['"]|import\s*\(\s*['"]([^'"]+)['"]\s*\)|import\s+['"]([^'"]+)['"]/g
const EXTRACT_DEPENDENCIES_REGEX = /import(?:\s+(?:\w+|\{[^}]*\}|\*\s+as\s+\w+)(?:\s*,\s*(?:\w+|\{[^}]*\}|\*\s+as\s+\w+))*\s+from\s+)?['"]([^'"]+)['"]/g
const COMPONENT_IMPORT_REGEX = /import\s+(\w+)\s+from\s+['"]([^'"]+)['"]/g
const CLIENT_IMPORT_REGEX = /import\s+(?:(\w+)|\{([^}]+)\})\s+from\s+['"]([^'"]+)['"];?\s*$/gm
const PROXY_FILE_REGEX = /^proxy\.(?:tsx?|jsx?|mts|mjs)$/
const COMPONENTS_PATH_REGEX = /\/components\/(\w+)(?:\.tsx?|\.jsx?)?$/
const COMPONENTS_PATH_ALT_REGEX = /[/\\]components[/\\](\w+)(?:\.tsx?|\.jsx?)?$/
const SPECIAL_FILE_REGEX = /^(?:robots|sitemap|feed)\.(?:tsx?|jsx?)$/
const NODE_PROTOCOL_REGEX = /^node:/
const NODE_BUILTINS = new Set([
  'fs',
  'path',
  'os',
  'crypto',
  'util',
  'stream',
  'events',
  'process',
  'buffer',
  'url',
  'querystring',
  'zlib',
  'http',
  'https',
  'net',
  'tls',
  'child_process',
  'cluster',
  'worker_threads',
])
export const RARI_CSS_MODULES_PATTERN = '[hash]_[local]'

const RARI_DIST_DIR = path.dirname(fileURLToPath(import.meta.url))
const RARI_PACKAGE_ROOT = path.dirname(RARI_DIST_DIR)
function isRariInternalPath(filePath: string): boolean {
  return filePath.startsWith(RARI_PACKAGE_ROOT)
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
      css?: string[]
    }
  >
  buildTime: string
}

interface RouteManifestEntry {
  filePath?: string
  css?: string[]
  componentId?: string
}

interface RouteManifest {
  routes?: RouteManifestEntry[]
  layouts?: RouteManifestEntry[]
  loading?: RouteManifestEntry[]
  errors?: RouteManifestEntry[]
  notFound?: RouteManifestEntry[]
  apiRoutes?: RouteManifestEntry[]
}

export interface ServerBuildOptions {
  outDir?: string
  rscDir?: string
  manifestPath?: string
  serverConfigPath?: string
  minify?: boolean
  alias?: Record<string, string>
  define?: Record<string, string>
  csp?: ServerCSPConfig
  cacheControl?: ServerCacheControlConfig
  cache?: ServerCacheConfig
  experimental?: {
    useCache?: boolean
    useCacheRemote?: ServerCacheLayerConfig
  }
}

export interface ComponentRebuildResult {
  componentId: string
  bundlePath: string
  success: boolean
  error?: string
}

type ResolvedServerBuildOptions = Required<Omit<ServerBuildOptions, 'csp' | 'cacheControl' | 'cache' | 'define' | 'serverConfigPath' | 'experimental'>> & {
  serverConfigPath: string
  csp?: ServerBuildOptions['csp']
  cacheControl?: ServerBuildOptions['cacheControl']
  cache?: ServerBuildOptions['cache']
  define?: ServerBuildOptions['define']
  experimental?: ServerBuildOptions['experimental']
}

export class ServerComponentBuilder {
  private serverComponents = new Map<
    string,
    {
      filePath: string
      originalCode: string
      dependencies: string[]
      hasNodeImports: boolean
    }
  >()

  private serverActions = new Map<
    string,
    {
      filePath: string
      originalCode: string
      dependencies: string[]
      hasNodeImports: boolean
    }
  >()

  private options: ResolvedServerBuildOptions
  private projectRoot: string

  private buildCache = new Map<string, {
    code: string
    css: string[]
    timestamp: number
    dependencies: string[]
  }>()

  private htmlOnlyImports = new Set<string>()
  private fileImporters = new Map<string, Set<string>>()
  private directiveResultCache = new Map<string, { hasUseClient: boolean, hasUseServer: boolean, error: boolean }>()
  private discoveredExternalClientComponents = new Set<string>()

  getComponentCount(): number {
    return this.serverComponents.size + this.serverActions.size
  }

  hasComponent(filePath: string): boolean {
    return this.serverComponents.has(filePath) || this.serverActions.has(filePath)
  }

  removeComponent(filePath: string): void {
    this.serverComponents.delete(filePath)
    this.serverActions.delete(filePath)
    this.directiveResultCache.delete(filePath)
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

  private hashString(value: string, length = 8): string {
    return sharedHashString(value, length)
  }

  private async writeComponentCssAsset(componentId: string, cssModules: string[]): Promise<string[]> {
    if (cssModules.length === 0)
      return []

    const assetsDir = path.join(this.options.outDir, 'assets', 'server')
    await fs.promises.mkdir(assetsDir, { recursive: true })

    const cssContent = `${cssModules.join('\n')}\n`
    const cssFileName = `${this.hashString(componentId + cssContent, 12)}.css`
    const cssPath = path.join(assetsDir, cssFileName)
    await fs.promises.writeFile(cssPath, cssContent, 'utf-8')

    return [`/assets/server/${cssFileName}`]
  }

  private getComponentIdFromRouteManifestPath(filePath: string): string {
    return this.getComponentId(path.join(this.projectRoot, 'src', 'app', filePath))
  }

  private getLegacyComponentReferenceId(filePath: string): string {
    return this.getReadableComponentId(this.getProjectRelativePath(filePath))
  }

  private async writeRouteCssEntries(manifest: ServerComponentManifest): Promise<void> {
    const routesPath = path.join(this.options.outDir, this.options.rscDir, 'routes.json')
    if (!fs.existsSync(routesPath))
      return

    const content = await fs.promises.readFile(routesPath, 'utf-8')
    const routeManifest = JSON.parse(content) as RouteManifest

    const applyCss = (entries?: RouteManifestEntry[]) => {
      if (!entries)
        return

      for (const entry of entries) {
        if (!entry.filePath) {
          continue
        }

        const componentId = this.getComponentIdFromRouteManifestPath(entry.filePath)
        entry.componentId = componentId

        const css = manifest.components[componentId]?.css ?? []
        if (css.length) {
          entry.css = css
        }
        else {
          delete entry.css
        }
      }
    }

    applyCss(routeManifest.routes)
    applyCss(routeManifest.layouts)
    applyCss(routeManifest.loading)
    applyCss(routeManifest.errors)
    applyCss(routeManifest.notFound)

    if (routeManifest.apiRoutes) {
      for (const entry of routeManifest.apiRoutes) {
        if (entry.filePath) {
          entry.componentId = this.getComponentIdFromRouteManifestPath(entry.filePath)
        }
      }
    }

    await fs.promises.writeFile(routesPath, JSON.stringify(routeManifest, null, 2), 'utf-8')
  }

  constructor(projectRoot: string, options: ServerBuildOptions = {}) {
    this.projectRoot = projectRoot
    const rscDir = options.rscDir || 'server'
    this.options = {
      outDir: options.outDir || path.join(projectRoot, 'dist'),
      rscDir,
      manifestPath: options.manifestPath || path.join(rscDir, 'manifest.json'),
      serverConfigPath: options.serverConfigPath || path.join(rscDir, 'config.json'),
      minify: options.minify ?? process.env.NODE_ENV === 'production',
      alias: options.alias || {},
      define: options.define,
      csp: options.csp,
      cacheControl: options.cacheControl,
      experimental: options.experimental,
    }

    this.parseHtmlImports()
  }

  private parseHtmlImports() {
    const indexHtmlPath = path.join(this.projectRoot, 'index.html')
    if (!fs.existsSync(indexHtmlPath))
      return

    try {
      const htmlContent = fs.readFileSync(indexHtmlPath, 'utf-8')
      for (const match of htmlContent.matchAll(HTML_IMPORT_REGEX)) {
        const importPath = match[1] || match[2]
        if (importPath.startsWith('/src/')) {
          const absolutePath = path.join(this.projectRoot, importPath.slice(1))
          this.htmlOnlyImports.add(absolutePath)
        }
      }
    }
    catch (error) {
      console.warn('[server-build] Error parsing index.html:', error)
    }
  }

  private isHtmlOnlyImport(filePath: string): boolean {
    return this.htmlOnlyImports.has(filePath)
  }

  private getDirectivesCached(filePath: string, source?: string): { hasUseClient: boolean, hasUseServer: boolean, error: boolean } {
    if (!source) {
      const cached = this.directiveResultCache.get(filePath)
      if (cached)
        return cached
    }

    try {
      const code = source ?? fs.readFileSync(filePath, 'utf-8')
      const directives = getDirectives(code)
      const result = { hasUseClient: directives.hasUseClient, hasUseServer: directives.hasUseServer, error: false }
      this.directiveResultCache.set(filePath, result)
      return result
    }
    catch {
      return { hasUseClient: false, hasUseServer: false, error: true }
    }
  }

  isServerComponent(filePath: string, source?: string): boolean {
    if (filePath.includes('node_modules'))
      return false

    if (this.isHtmlOnlyImport(filePath))
      return false

    const directives = this.getDirectivesCached(filePath, source)
    if (directives.error)
      return false

    return !directives.hasUseClient && !directives.hasUseServer
  }

  private isClientComponent(filePath: string, source?: string): boolean {
    return this.getDirectivesCached(filePath, source).hasUseClient
  }

  buildImportGraph(srcDir: string) {
    this.fileImporters.clear()

    const scanForImports = (dir: string) => {
      if (!fs.existsSync(dir))
        return

      const entries = fs.readdirSync(dir, { withFileTypes: true })

      for (const entry of entries) {
        const fullPath = path.join(dir, entry.name)

        if (entry.isDirectory()) {
          if (entry.name === 'node_modules')
            continue
          scanForImports(fullPath)
        }
        else if (entry.isFile() && TSX_EXT_REGEX.test(entry.name)) {
          try {
            const code = fs.readFileSync(fullPath, 'utf-8')
            let match

            CODE_IMPORT_REGEX.lastIndex = 0
            match = CODE_IMPORT_REGEX.exec(code)
            while (match !== null) {
              const importPath = match[1] || match[2] || match[3]
              let resolvedPath: string | null = null

              if (importPath.startsWith('./') || importPath.startsWith('../')) {
                const importerDir = path.dirname(fullPath)
                resolvedPath = path.resolve(importerDir, importPath)
              }
              else if (importPath.startsWith('@/')) {
                const relativePath = importPath.slice(2)
                resolvedPath = path.join(this.projectRoot, 'src', relativePath)
              }

              if (resolvedPath) {
                const extensions = ['', '.ts', '.tsx', '.js', '.jsx']
                let foundPath: string | null = null

                for (const ext of extensions) {
                  const pathWithExt = resolvedPath + ext
                  try {
                    if (fs.statSync(pathWithExt).isFile()) {
                      foundPath = pathWithExt
                      break
                    }
                  }
                  catch {}
                }

                if (foundPath) {
                  if (!this.fileImporters.has(foundPath))
                    this.fileImporters.set(foundPath, new Set())

                  this.fileImporters.get(foundPath)!.add(fullPath)
                }
              }

              match = CODE_IMPORT_REGEX.exec(code)
            }
          }
          catch (err) {
            if ((err as any)?.code !== 'ENOENT')
              console.warn('[rari] Unexpected error building import graph:', fullPath, err)
          }
        }
      }
    }

    scanForImports(srcDir)
  }

  isOnlyImportedByClientComponents(filePath: string): boolean {
    const importers = this.fileImporters.get(filePath)

    if (!importers || importers.size === 0)
      return false

    for (const importer of importers) {
      if (this.isClientComponent(importer))
        continue

      if (!this.isOnlyImportedByClientComponents(importer))
        return false
    }

    return true
  }

  addServerComponent(filePath: string, source?: string) {
    const code = source ?? fs.readFileSync(filePath, 'utf-8')

    if (this.isServerAction(code)) {
      const dependencies = this.extractDependencies(code)
      const hasNodeImports = this.hasNodeImports(code)

      this.serverActions.set(filePath, {
        filePath,
        originalCode: code,
        dependencies,
        hasNodeImports,
      })
      return
    }

    if (!this.isServerComponent(filePath, code))
      return

    const dependencies = this.extractDependencies(code)
    const hasNodeImports = this.hasNodeImports(code)

    this.serverComponents.set(filePath, {
      filePath,
      originalCode: code,
      dependencies,
      hasNodeImports,
    })
  }

  private isServerAction(code: string): boolean {
    return getDirectives(code).hasUseServer
  }

  private extractDependencies(code: string): string[] {
    const dependencies: string[] = []
    let match

    EXTRACT_DEPENDENCIES_REGEX.lastIndex = 0
    while (true) {
      match = EXTRACT_DEPENDENCIES_REGEX.exec(code)
      if (match === null)
        break

      const importPath = match[1]
      if (
        !importPath.startsWith('.')
        && !importPath.startsWith('/')
        && !importPath.startsWith('node:')
        && !this.isNodeBuiltin(importPath)
      ) {
        dependencies.push(importPath)
      }
    }

    return [...new Set(dependencies)]
  }

  private isNodeBuiltin(moduleName: string): boolean {
    return NODE_BUILTINS.has(moduleName)
  }

  private hasNodeImports(code: string): boolean {
    return (
      code.includes('from \'node:')
      || code.includes('from "node:')
      || code.includes('from \'fs\'')
      || code.includes('from "fs"')
      || code.includes('from \'path\'')
      || code.includes('from "path"')
      || code.includes('from \'os\'')
      || code.includes('from "os"')
      || code.includes('from \'crypto\'')
      || code.includes('from "crypto"')
      || code.includes('from \'util\'')
      || code.includes('from "util"')
      || code.includes('from \'stream\'')
      || code.includes('from "stream"')
      || code.includes('from \'events\'')
      || code.includes('from "events"')
    )
  }

  async getTransformedComponentsForDevelopment(): Promise<Array<{ id: string, code: string }>> {
    const components: Array<{ id: string, code: string }> = []

    for (const [filePath] of this.serverComponents) {
      const relativePath = path.relative(this.projectRoot, filePath)
      const componentId = this.getComponentId(relativePath)

      const transformedCode = await this.buildComponentCodeOnly(filePath)

      components.push({
        id: componentId,
        code: transformedCode,
      })
    }

    for (const [filePath] of this.serverActions) {
      const relativePath = path.relative(this.projectRoot, filePath)
      const actionId = this.getComponentId(relativePath)

      const transformedCode = await this.buildComponentCodeOnly(filePath)

      components.push({
        id: actionId,
        code: transformedCode,
      })
    }

    return components
  }

  private transformComponentImportsToGlobal(code: string): string {
    const replacements: Array<{ original: string, replacement: string }> = []

    for (const match of code.matchAll(COMPONENT_IMPORT_REGEX)) {
      const [fullMatch, importName, importPath] = match

      if (!importPath.startsWith('.') && !importPath.startsWith('@') && !importPath.startsWith('~') && !importPath.startsWith('#'))
        continue

      let resolvedPath: string | null = null

      if (importPath.startsWith('.')) {
        if (importPath.includes('/components/')) {
          const componentMatch = importPath.match(COMPONENTS_PATH_REGEX)
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

            if (!isClient)
              continue

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

      const aliases = this.options.alias || {}
      for (const [alias, replacement] of Object.entries(aliases)) {
        if (importPath.startsWith(`${alias}/`) || importPath === alias) {
          const relativePath = importPath.slice(alias.length)
          resolvedPath = path.join(replacement, relativePath)
          break
        }
      }

      if (resolvedPath) {
        const componentMatch = resolvedPath.match(COMPONENTS_PATH_ALT_REGEX)
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
              if (this.isClientComponent(possiblePath))
                isClient = true
              break
            }
          }

          if (!isClient)
            continue

          const componentId = this.getLegacyComponentReferenceId(actualPath)

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
    const self = this

    const clientComponentRefs = new Map<string, string>()
    const serverActionRefs = new Map<string, { actionId: string, hasDefaultExport: boolean }>()

    return [
      {
        name: 'virtual-module',
        resolveId(id: string, importer: string | undefined) {
          if (id === virtualModuleId)
            return id

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
              if (fs.existsSync(indexPath))
                return indexPath
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
        resolveId: (source: string, importer: string | undefined) => {
          if (!importer || importer.includes('node_modules') || isRariInternalPath(importer))
            return null

          if (
            source.startsWith('node:')
            || self.isNodeBuiltin(source)
            || source === 'react'
            || source === 'react-dom'
            || source === 'react/jsx-runtime'
            || source === 'react/jsx-dev-runtime'
          ) {
            return null
          }

          let resolvedPath: string | null = null
          const aliases = self.options.alias || {}

          resolvedPath = resolveAlias(source, aliases, self.projectRoot)

          if (!resolvedPath && (source.startsWith('./') || source.startsWith('../'))) {
            const importerDir = importer === virtualModuleId ? resolveDir : path.dirname(importer)
            resolvedPath = path.resolve(importerDir, source)
          }

          if (resolvedPath) {
            const extensions = ['', '.ts', '.tsx', '.js', '.jsx']
            for (const ext of extensions) {
              const pathWithExt = resolvedPath + ext
              if (fs.existsSync(pathWithExt) && fs.statSync(pathWithExt).isFile()) {
                if (self.isClientComponent(pathWithExt)) {
                  const relativePath = path.relative(self.projectRoot, pathWithExt)
                  const componentId = (relativePath.startsWith('..') ? pathWithExt : relativePath).replace(BACKSLASH_REGEX, '/')
                  clientComponentRefs.set(pathWithExt, componentId)

                  if (relativePath.startsWith('..'))
                    self.discoveredExternalClientComponents.add(pathWithExt)

                  return { id: `\0client-ref:${pathWithExt}` }
                }

                try {
                  const content = fs.readFileSync(pathWithExt, 'utf-8')
                  if (hasTopLevelUseServerDirective(content)) {
                    const actionId = self.getComponentId(pathWithExt)
                    const hasDefault = hasDefaultExport(content)
                    serverActionRefs.set(pathWithExt, { actionId, hasDefaultExport: hasDefault })
                    return { id: `\0server-action:${pathWithExt}` }
                  }
                }
                catch (error) {
                  console.error(`[rari] Failed to read file for server action detection: ${pathWithExt}`, error)
                }
                break
              }
            }
          }

          return null
        },
        load(id: string) {
          if (id.startsWith('\0client-ref:')) {
            const filePath = id.slice('\0client-ref:'.length)
            const relativePath = path.relative(self.projectRoot, filePath)
            const componentId = (clientComponentRefs.get(filePath) || (relativePath.startsWith('..') ? filePath : relativePath)).replace(BACKSLASH_REGEX, '/')

            return {
              code: `
function registerClientReference(clientReference, id, exportName) {
  const key = id + '#' + exportName;
  const clientProxy = {};
  Object.defineProperty(clientProxy, '$$typeof', {
    value: Symbol.for('react.client.reference'),
    enumerable: false
  });
  Object.defineProperty(clientProxy, '$$id', {
    value: key,
    enumerable: false
  });
  Object.defineProperty(clientProxy, '$$async', {
    value: false,
    enumerable: false
  });
  return clientProxy;
}

export default registerClientReference(null, ${JSON.stringify(componentId)}, "default");
`,
              moduleType: 'js',
            }
          }

          if (id.startsWith('\0server-action:')) {
            const filePath = id.slice('\0server-action:'.length)

            const actionId = serverActionRefs.get(filePath)?.actionId ?? self.getComponentId(filePath)
            const builtPath = path.join(self.options.outDir, self.options.rscDir, `${actionId}.js`)
            const absoluteBuiltPath = path.resolve(self.projectRoot, builtPath)

            const builtFileUrl = pathToFileURL(absoluteBuiltPath).href

            const actionInfo = serverActionRefs.get(filePath)
            const hasDefault = actionInfo?.hasDefaultExport ?? false

            const exportStatement = hasDefault
              ? `export * from ${JSON.stringify(builtFileUrl)};\nexport { default } from ${JSON.stringify(builtFileUrl)};`
              : `export * from ${JSON.stringify(builtFileUrl)};`

            return {
              code: exportStatement,
              moduleType: 'js',
            }
          }

          return null
        },
      },
      {
        name: 'use-transformed-server-components',
        resolveId: (source: string, importer: string | undefined) => {
          if (!isPage)
            return null

          if (source.startsWith('file://')) {
            const filePath = source.replace(FILE_PROTOCOL_REGEX, '')
            if (fs.existsSync(filePath))
              return { id: `\0transformed:${filePath}` }

            return null
          }

          let resolvedPath: string | null = null
          const aliases = self.options.alias || {}

          resolvedPath = resolveAlias(source, aliases, self.projectRoot)

          const importerDir = importer?.startsWith('\0') ? resolveDir : (importer ? path.dirname(importer) : resolveDir)
          if (!resolvedPath && (source.startsWith('./') || source.startsWith('../')))
            resolvedPath = path.resolve(importerDir, source)

          if (!resolvedPath)
            return null

          if (importerDir.includes('node_modules'))
            return null

          const extensions = ['', '.ts', '.tsx', '.js', '.jsx']
          for (const ext of extensions) {
            const pathWithExt = resolvedPath + ext
            if (fs.existsSync(pathWithExt) && fs.statSync(pathWithExt).isFile()) {
              if (self.isClientComponent(pathWithExt))
                return null

              const srcDir = path.join(self.projectRoot, 'src')
              if (!pathWithExt.startsWith(srcDir))
                return null

              const componentId = self.getComponentId(pathWithExt)
              const distPath = path.join(self.options.outDir, self.options.rscDir, `${componentId}.js`)

              if (fs.existsSync(distPath))
                return { id: `\0transformed:${distPath}` }

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
          if (source.startsWith('\0'))
            return null

          const aliases = self.options.alias || {}
          for (const [alias, replacement] of Object.entries(aliases)) {
            if (source.startsWith(`${alias}/`) || source === alias) {
              const relativePath = source.slice(alias.length)
              const resolvedPath = path.join(replacement, relativePath)
              const absolutePath = path.isAbsolute(resolvedPath)
                ? resolvedPath
                : path.resolve(self.projectRoot, resolvedPath)

              const extensions = ['', '.ts', '.tsx', '.js', '.jsx']
              for (const ext of extensions) {
                const pathWithExt = absolutePath + ext
                if (fs.existsSync(pathWithExt) && fs.statSync(pathWithExt).isFile())
                  return pathWithExt
              }

              for (const ext of ['.ts', '.tsx', '.js', '.jsx']) {
                const indexPath = path.join(absolutePath, `index${ext}`)
                if (fs.existsSync(indexPath))
                  return indexPath
              }

              return absolutePath
            }
          }

          return null
        },
      },
      {
        name: 'resolve-rari-proxy',
        resolveId: (source: string) => {
          if (isProxyFile && source === 'rari') {
            const rariResponsePath = path.join(RARI_DIST_DIR, 'proxy/RariResponse.mjs')
            if (fs.existsSync(rariResponsePath))
              return rariResponsePath

            const rariResponseSrcPath = path.join(RARI_PACKAGE_ROOT, 'src/proxy/RariResponse.ts')
            if (fs.existsSync(rariResponseSrcPath))
              return rariResponseSrcPath
          }

          return null
        },
      },
      {
        name: 'css-modules',
        resolveId: (source: string, importer: string | undefined) => {
          if (source.endsWith('.module.css')) {
            const importerDir = !importer?.startsWith('\0') && importer ? path.dirname(importer) : resolveDir
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

            if (cssModules)
              cssModules.push(new TextDecoder().decode(result.code))

            const classes: Record<string, string> = {}
            if (result.exports) {
              for (const [key, value] of Object.entries(result.exports)) {
                classes[key] = value.name
              }
            }

            return { code: `export default ${JSON.stringify(classes)}`, moduleType: 'js' }
          }
          catch (e) {
            throw new Error(`[rari] Failed to process CSS module ${id}: ${e instanceof Error ? e.message : String(e)}`)
          }
        },
      } satisfies Plugin,
      {
        name: 'externalize-deps',
        resolveId: (source: string) => {
          if (source.startsWith('\0'))
            return null

          if (source.startsWith('node:') || self.isNodeBuiltin(source))
            return { id: source, external: true }

          const externalPackages = [
            'react',
            'react-dom',
            'react/jsx-runtime',
            'react/jsx-dev-runtime',
            'rari/image',
          ]

          if (externalPackages.includes(source))
            return { id: source, external: true }

          const externalPackageMappings: Record<string, string | null> = {
            'rari/runtime/cache-wrapper': 'node_modules/rari/dist/runtime/cache-wrapper.mjs',
            'react-server-dom-rari/server': 'node_modules/rari/dist/runtime/react-server-dom-shim.mjs',
          }

          if (source in externalPackageMappings) {
            return { id: source, external: true }
          }

          if (source === 'rari' || source === 'rari/client')
            return null

          if (!source.startsWith('.') && !source.startsWith('/'))
            return { id: source, external: true }

          return null
        },
      },
      {
        name: 'use-cache',
        async transform(code: string, id: string) {
          if (!self.options.experimental?.useCache)
            return null

          const transform = await getUseCacheTransform()
          if (!transform) {
            return null
          }

          return transform(code, id)
        },
      },
    ]
  }

  private async buildComponentCodeOnly(
    inputPath: string,
  ): Promise<string> {
    const originalCode = await fs.promises.readFile(inputPath, 'utf-8')
    const clientTransformedCode = this.transformClientImports(
      originalCode,
      inputPath,
    )
    const isPage = this.isPageComponent(inputPath)
    const transformedCode = isPage
      ? this.transformComponentImportsToGlobal(clientTransformedCode)
      : clientTransformedCode

    const ext = path.extname(inputPath)
    let loader: 'tsx' | 'jsx' | 'ts' | 'js'
    if (ext === '.tsx')
      loader = 'tsx'
    else if (ext === '.ts')
      loader = 'ts'
    else if (ext === '.jsx')
      loader = 'jsx'
    else
      loader = 'js'

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
          'process.env.NODE_ENV': JSON.stringify(process.env.NODE_ENV || 'production'),
          ...this.options.define,
        },
      },
      plugins: this.createBuildPlugins(virtualModuleId, transformedCode, loader, inputPath, isPage, []),
    })

    if (!result.output || result.output.length === 0)
      throw new Error('No output generated from Rolldown')

    const entryChunk = result.output.find(chunk => chunk.type === 'chunk' && chunk.isEntry)
    if (!entryChunk || entryChunk.type !== 'chunk')
      throw new Error('No entry chunk found in Rolldown output')

    let code = entryChunk.code

    const timestamp = new Date().toISOString()
    code = `// Built: ${timestamp}\n${code}`

    return code
  }

  private async buildComponentBatch(
    entries: Array<[string, { filePath: string, dependencies: string[], hasNodeImports: boolean }]>,
    manifest: ServerComponentManifest,
    concurrency: number,
  ): Promise<void> {
    let active = 0
    let index = 0
    const errors: Error[] = []

    await new Promise<void>((resolve) => {
      const next = () => {
        while (active < concurrency && index < entries.length) {
          const [filePath, component] = entries[index++]
          const relativePath = path.relative(this.projectRoot, filePath)
          const componentId = this.getComponentId(filePath)
          const bundlePath = path.join(this.options.rscDir, `${componentId}.js`)
          const fullBundlePath = path.join(this.options.outDir, bundlePath)

          active++
          ;(async () => {
            try {
              const bundleDir = path.dirname(fullBundlePath)
              await fs.promises.mkdir(bundleDir, { recursive: true })

              const built = await this.buildSingleComponent(filePath, fullBundlePath)
              const css = await this.writeComponentCssAsset(componentId, built.css)

              const moduleSpecifier = pathToFileURL(path.resolve(this.projectRoot, fullBundlePath)).href

              manifest.components[componentId] = {
                id: componentId,
                filePath,
                relativePath,
                bundlePath,
                moduleSpecifier,
                dependencies: component.dependencies,
                hasNodeImports: component.hasNodeImports,
                css,
              }
            }
            catch (error) {
              errors.push(error instanceof Error ? error : new Error(String(error)))
            }
            finally {
              active--
              if (index >= entries.length && active === 0) {
                resolve()
              }
              else {
                next()
              }
            }
          })()
        }

        if (entries.length === 0)
          resolve()
      }

      next()
    })

    if (errors.length > 0)
      throw errors[0]
  }

  async buildServerComponents(): Promise<ServerComponentManifest> {
    const serverOutDir = path.join(this.options.outDir, this.options.rscDir)

    await fs.promises.mkdir(serverOutDir, { recursive: true })

    const manifest: ServerComponentManifest = {
      components: {},
      buildTime: new Date().toISOString(),
    }

    const concurrency = Math.min(8, Math.max(1, (await import('node:os')).cpus().length))

    const nonPageComponents = [...this.serverComponents.entries()]
      .filter(([filePath]) => !this.isPageComponent(filePath))
    const pageComponents = [...this.serverComponents.entries()]
      .filter(([filePath]) => this.isPageComponent(filePath))
    const actions = [...this.serverActions.entries()]

    await Promise.all([
      this.buildComponentBatch(nonPageComponents, manifest, concurrency),
      this.buildComponentBatch(actions, manifest, concurrency),
    ])

    await this.buildComponentBatch(pageComponents, manifest, concurrency)

    const manifestPath = path.join(
      this.options.outDir,
      this.options.manifestPath,
    )
    await fs.promises.writeFile(
      manifestPath,
      JSON.stringify(manifest, null, 2),
      'utf-8',
    )
    await this.writeRouteCssEntries(manifest)

    const serverConfig: ServerConfig = {}
    if (this.options.csp)
      serverConfig.csp = this.options.csp
    if (this.options.cacheControl)
      serverConfig.cacheControl = this.options.cacheControl
    if (this.options.cache)
      serverConfig.cache = this.options.cache
    if (this.options.experimental?.useCacheRemote) {
      serverConfig.useCache = {
        remote: this.options.experimental.useCacheRemote,
      }
    }

    const serverConfigPath = path.join(
      this.options.outDir,
      this.options.serverConfigPath,
    )

    if (Object.keys(serverConfig).length === 0) {
      try {
        await fs.promises.unlink(serverConfigPath)
      }
      catch (error: unknown) {
        const e = error as NodeJS.ErrnoException
        if (e.code !== 'ENOENT')
          console.warn(`Failed to remove server config file:`, error)
      }
    }
    else {
      await fs.promises.writeFile(
        serverConfigPath,
        JSON.stringify(serverConfig, null, 2),
        'utf-8',
      )
    }

    return manifest
  }

  async buildSSRClientComponents(): Promise<void> {
    const ssrOutDir = path.join(this.options.outDir, 'ssr')
    await fs.promises.mkdir(ssrOutDir, { recursive: true })

    const clientFiles: Array<{ filePath: string, code: string }> = []
    const srcDir = path.join(this.projectRoot, 'src')

    const scanForClientComponents = (dir: string) => {
      if (!fs.existsSync(dir))
        return
      const entries = fs.readdirSync(dir, { withFileTypes: true })
      for (const entry of entries) {
        const fullPath = path.join(dir, entry.name)
        if (entry.isDirectory()) {
          if (entry.name === 'node_modules')
            continue
          scanForClientComponents(fullPath)
        }
        else if (entry.isFile() && TSX_EXT_REGEX.test(entry.name)) {
          try {
            const code = fs.readFileSync(fullPath, 'utf-8')
            if (hasTopLevelUseClientDirective(code))
              clientFiles.push({ filePath: fullPath, code })
          }
          catch {}
        }
      }
    }

    scanForClientComponents(srcDir)

    for (const extPath of this.discoveredExternalClientComponents) {
      if (clientFiles.some(f => f.filePath === extPath))
        continue
      try {
        const code = fs.readFileSync(extPath, 'utf-8')
        clientFiles.push({ filePath: extPath, code })
      }
      catch {}
    }

    try {
      const rariPkgDir = path.dirname(fileURLToPath(import.meta.resolve('rari/package.json')))
      const errorBoundarySource = path.join(rariPkgDir, 'src', 'runtime', 'ErrorBoundaryWrapper.tsx')
      if (fs.existsSync(errorBoundarySource)) {
        const code = fs.readFileSync(errorBoundarySource, 'utf-8')
        clientFiles.push({ filePath: errorBoundarySource, code })
      }
    }
    catch {}

    if (clientFiles.length === 0) {
      const manifestPath = path.join(ssrOutDir, 'manifest.json')
      await fs.promises.writeFile(manifestPath, '{}', 'utf-8')
      return
    }

    const clientModuleSpecifiers = new Map<string, string>()
    for (const { filePath } of clientFiles) {
      const bundleName = this.getComponentId(filePath)
      clientModuleSpecifiers.set(
        path.resolve(filePath),
        `file:///ssr/${bundleName}.js`,
      )
    }

    const manifest: Record<string, { id: string, filePath: string, bundlePath: string, exports: string[] }> = {}
    const concurrency = Math.min(8, Math.max(1, (await import('node:os')).cpus().length))
    let active = 0
    let index = 0

    await new Promise<void>((resolve) => {
      const next = () => {
        while (active < concurrency && index < clientFiles.length) {
          const { filePath, code } = clientFiles[index++]
          const relativePath = path.relative(this.projectRoot, filePath).replace(BACKSLASH_REGEX, '/')
          const componentId = relativePath.startsWith('..') ? filePath.replace(BACKSLASH_REGEX, '/') : relativePath
          const bundleName = this.getComponentId(filePath)
          const bundlePath = `ssr/${bundleName}.js`
          const fullBundlePath = path.join(this.options.outDir, bundlePath)

          active++
          ;(async () => {
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
            }
            catch (error) {
              console.warn(`[rari] SSR build failed for ${componentId}:`, error instanceof Error ? error.message : error)
            }
            finally {
              active--
              if (index >= clientFiles.length && active === 0)
                resolve()
              else
                next()
            }
          })()
        }

        if (clientFiles.length === 0)
          resolve()
      }

      next()
    })

    const manifestPath = path.join(ssrOutDir, 'manifest.json')

    const ebEntry = Object.entries(manifest).find(([_, v]) =>
      v.filePath?.includes('ErrorBoundaryWrapper'),
    )
    if (ebEntry) {
      const [, ebInfo] = ebEntry
      manifest['virtual:error-boundary-wrapper.tsx'] = {
        id: 'virtual:error-boundary-wrapper.tsx',
        filePath: 'virtual:error-boundary-wrapper.tsx',
        bundlePath: ebInfo.bundlePath,
        exports: ['ErrorBoundaryWrapper'],
      }
    }

    await fs.promises.writeFile(manifestPath, JSON.stringify(manifest, null, 2), 'utf-8')

    const clientReferenceManifest: Record<string, { id: string, chunks: string, name: string }> = {}
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

    if (ebEntry) {
      const [, ebInfo] = ebEntry
      clientReferenceManifest['virtual:error-boundary-wrapper.tsx#ErrorBoundaryWrapper'] = {
        id: 'virtual:error-boundary-wrapper.tsx#ErrorBoundaryWrapper',
        chunks: `/${ebInfo.bundlePath}`,
        name: 'ErrorBoundaryWrapper',
      }
    }

    const serverOutDir = path.join(this.options.outDir, 'server')
    await fs.promises.mkdir(serverOutDir, { recursive: true })
    const clientRefManifestPath = path.join(serverOutDir, 'client-reference-manifest.json')
    await fs.promises.writeFile(clientRefManifestPath, JSON.stringify(clientReferenceManifest, null, 2), 'utf-8')
  }

  private extractExportNames(code: string): string[] {
    const exports: string[] = []
    if (/export\s+default\b/.test(code))
      exports.push('default')
    const namedExportRegex = /export\s+(?:function|const|let|var|class)\s+(\w+)/g
    for (const m of code.matchAll(namedExportRegex)) {
      exports.push(m[1])
    }

    return exports.length > 0 ? exports : ['default']
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
    if (ext === '.tsx')
      loader = 'tsx'
    else if (ext === '.ts')
      loader = 'ts'
    else if (ext === '.jsx')
      loader = 'jsx'
    else
      loader = 'js'

    const virtualModuleId = `\0ssr-virtual:${inputPath}`
    const projectRoot = this.projectRoot

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
          'process.env.NODE_ENV': JSON.stringify(process.env.NODE_ENV || 'production'),
        },
      },
      plugins: [
        {
          name: 'ssr-client-virtual',
          resolveId(id) {
            if (id === virtualModuleId)
              return id

            return null
          },
          load(id) {
            if (id === virtualModuleId)
              return { code: strippedCode, moduleType: loader }

            return null
          },
        },
        {
          name: 'ssr-client-resolve',
          resolveId(id, importer) {
            if (id.startsWith('.') || id.startsWith('/') || id.startsWith('@/')) {
              let resolved = id
              if (id.startsWith('@/'))
                resolved = path.join(projectRoot, 'src', id.slice(2))
              else if (importer === virtualModuleId)
                resolved = path.resolve(path.dirname(inputPath), id)
              else if (importer)
                resolved = path.resolve(path.dirname(importer.replace('\0ssr-virtual:', '')), id)

              const found = resolveWithExtensions(resolved, ['.ts', '.tsx', '.js', '.jsx'])
                || resolveIndexFile(resolved, ['.ts', '.tsx', '.js', '.jsx'])

              if (found && clientModuleSpecifiers) {
                const resolvedAbs = path.resolve(found)
                const specifier = clientModuleSpecifiers.get(resolvedAbs)
                if (specifier && resolvedAbs !== path.resolve(inputPath))
                  return { id: specifier, external: true }
              }

              return found || null
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
    const clientTransformedCode = this.transformClientImports(
      originalCode,
      inputPath,
    )
    const isPage = this.isPageComponent(inputPath)
    const transformedCode = isPage
      ? this.transformComponentImportsToGlobal(clientTransformedCode)
      : clientTransformedCode

    const ext = path.extname(inputPath)
    let loader: 'tsx' | 'jsx' | 'ts' | 'js'
    if (ext === '.tsx')
      loader = 'tsx'
    else if (ext === '.ts')
      loader = 'ts'
    else if (ext === '.jsx')
      loader = 'jsx'
    else
      loader = 'js'

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
          'process.env.NODE_ENV': JSON.stringify(process.env.NODE_ENV || 'production'),
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

    if (!result.output || result.output.length === 0)
      throw new Error('No output generated from Rolldown')

    const entryChunk = result.output.find(chunk => chunk.type === 'chunk' && chunk.isEntry)
    if (!entryChunk || entryChunk.type !== 'chunk')
      throw new Error('No entry chunk found in Rolldown output')

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

    const replacements: Array<{ original: string, replacement: string }> = []
    let hasClientComponents = false

    const externalClientComponents = ['rari/image']

    CLIENT_IMPORT_REGEX.lastIndex = 0
    while (true) {
      match = CLIENT_IMPORT_REGEX.exec(code)
      if (match === null)
        break

      const [fullMatch, defaultImport, namedImports, importPath] = match

      let isClientComponent = false
      let componentId = importPath

      if (externalClientComponents.includes(importPath)) {
        isClientComponent = true
      }
      else {
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
        }
        else if (namedImports) {
          const imports = namedImports.split(',').map(imp => imp.trim())
          const registrations = imports.map((imp) => {
            const [importName, alias] = imp.includes(' as ')
              ? imp.split(' as ').map(s => s.trim())
              : [imp, imp]

            return `const ${alias} = registerClientReference(
  null,
  ${JSON.stringify(componentId)},
  ${JSON.stringify(importName)}
);`
          }).join('\n')

          replacement = registrations
        }

        replacements.push({ original: fullMatch, replacement })
      }
    }

    if (hasClientComponents) {
      const functionDefinition = `
function registerClientReference(clientReference, id, exportName) {
  const key = id + '#' + exportName;

  const clientProxy = {};

  Object.defineProperty(clientProxy, '$$typeof', {
    value: Symbol.for('react.client.reference'),
    enumerable: false
  });

  Object.defineProperty(clientProxy, '$$id', {
    value: key,
    enumerable: false
  });

  Object.defineProperty(clientProxy, '$$async', {
    value: false,
    enumerable: false
  });

  try {
    if (typeof globalThis.registerClientComponent === 'function') {
      globalThis.registerClientComponent(key, id, clientProxy);
    }
  } catch (error) {
    console.error('[rari] Build: Failed to register client reference:', error);
  }

  return clientProxy;
}

`
      transformedCode = functionDefinition + transformedCode
    }

    for (const { original, replacement } of replacements)
      transformedCode = transformedCode.replace(original, replacement)

    return transformedCode
  }

  private resolveImportPath(importPath: string, importerPath: string): string {
    let resolvedPath = importPath
    const aliases = this.options.alias || {}

    for (const [alias, replacement] of Object.entries(aliases)) {
      if (importPath.startsWith(`${alias}/`) || importPath === alias) {
        const relativePath = importPath.slice(alias.length)
        resolvedPath = path.join(replacement, relativePath)
        break
      }
    }

    if (!path.isAbsolute(resolvedPath))
      resolvedPath = path.resolve(path.dirname(importerPath), resolvedPath)

    const extensions = ['.tsx', '.jsx', '.ts', '.js']
    const withExt = resolveWithExtensions(resolvedPath, extensions)
    if (withExt)
      return withExt

    const indexFile = resolveIndexFile(resolvedPath, extensions)
    if (indexFile)
      return indexFile

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
    const dependencies = this.extractDependencies(code)
    const hasNodeImports = this.hasNodeImports(code)

    const componentData = {
      filePath,
      originalCode: code,
      dependencies,
      hasNodeImports,
    }

    if (this.isServerAction(code)) {
      this.serverActions.set(filePath, componentData)
      this.serverComponents.delete(filePath)
    }
    else {
      this.serverComponents.set(filePath, componentData)
      this.serverActions.delete(filePath)
    }

    const relativeBundlePath = path.join(
      this.options.rscDir,
      `${componentId}.js`,
    )
    const fullBundlePath = path.join(this.options.outDir, relativeBundlePath)

    const cached = this.buildCache.get(filePath)
    const fileStats = await fs.promises.stat(filePath)
    const fileTimestamp = fileStats.mtimeMs

    if (
      cached
      && cached.timestamp >= fileTimestamp
      && JSON.stringify(cached.dependencies) === JSON.stringify(dependencies)
    ) {
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

    const built = await this.buildSingleComponent(
      filePath,
      fullBundlePath,
    )
    const css = await this.writeComponentCssAsset(componentId, built.css)

    this.buildCache.set(filePath, {
      code: built.code,
      css,
      timestamp: Date.now(),
      dependencies,
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
    css: string[] = [],
  ): Promise<void> {
    const manifestPath = path.join(
      this.options.outDir,
      this.options.manifestPath,
    )

    let manifest: ServerComponentManifest

    if (this.manifestCache) {
      manifest = this.manifestCache
    }
    else if (fs.existsSync(manifestPath)) {
      const content = await fs.promises.readFile(manifestPath, 'utf-8')
      manifest = JSON.parse(content)
      this.manifestCache = manifest
    }
    else {
      manifest = {
        components: {},
        buildTime: new Date().toISOString(),
      }
      this.manifestCache = manifest
    }

    const componentData = this.serverComponents.get(filePath) || this.serverActions.get(filePath)
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
        dependencies: this.extractDependencies(code),
        hasNodeImports: this.hasNodeImports(code),
        css,
      }
    }
    else {
      manifest.components[componentId] = {
        id: componentId,
        filePath,
        relativePath: path.relative(this.projectRoot, filePath),
        bundlePath,
        moduleSpecifier,
        dependencies: componentData.dependencies,
        hasNodeImports: componentData.hasNodeImports,
        css,
      }
    }

    manifest.buildTime = new Date().toISOString()

    await fs.promises.writeFile(
      manifestPath,
      JSON.stringify(manifest, null, 2),
      'utf-8',
    )
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
    return await this.buildComponentCodeOnly(filePath)
  }
}

export function hasComponentExport(code: string): boolean {
  return hasDefaultExport(code)
    || EXPORTED_FUNCTION_REGEX.test(code)
    || EXPORTED_DEFAULT_ARROW_REGEX.test(code)
    || EXPORTED_CONST_FUNCTION_REGEX.test(code)
}

export function isEligibleServerComponent(
  filePath: string,
  code: string,
  builder: ServerComponentBuilder,
): boolean {
  const fileName = path.basename(filePath)
  if (SPECIAL_FILE_REGEX.test(fileName) || fileName.endsWith('.d.ts'))
    return false

  if (hasTopLevelUseClientDirective(code))
    return false

  if (hasTopLevelUseServerDirective(code))
    return true

  if (builder.isOnlyImportedByClientComponents(filePath))
    return false

  return builder.isServerComponent(filePath, code) && hasComponentExport(code)
}

export function scanDirectory(dir: string, builder: ServerComponentBuilder, isTopLevel = true) {
  if (isTopLevel)
    builder.buildImportGraph(dir)

  const entries = fs.readdirSync(dir, { withFileTypes: true })

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name)

    if (entry.isDirectory()) {
      scanDirectory(fullPath, builder, false)
    }
    else if (entry.isFile() && TSX_EXT_REGEX.test(entry.name)) {
      try {
        const code = fs.readFileSync(fullPath, 'utf-8')

        if (isEligibleServerComponent(fullPath, code, builder))
          builder.addServerComponent(fullPath, code)
      }
      catch (error) {
        console.warn(
          `[server-build] Error checking ${fullPath}:`,
          error instanceof Error ? error.message : error,
        )
      }
    }
  }
}

export function createServerBuildPlugin(
  options: ServerBuildOptions = {},
): Plugin {
  let builder: ServerComponentBuilder | null = null
  let projectRoot: string
  let isDev = false
  let resolvedAliases: Record<string, string> = {}

  return {
    name: 'rari-server-build',

    configResolved(config) {
      projectRoot = config.root
      isDev = config.command === 'serve'

      const alias: Record<string, string> = {}
      if (config.resolve?.alias) {
        const aliasConfig = config.resolve.alias
        if (Array.isArray(aliasConfig)) {
          aliasConfig.forEach((entry) => {
            if (typeof entry.find === 'string' && typeof entry.replacement === 'string')
              alias[entry.find] = entry.replacement
          })
        }
        else if (typeof aliasConfig === 'object') {
          Object.entries(aliasConfig).forEach(([key, value]) => {
            if (typeof value === 'string')
              alias[key] = value
          })
        }
      }

      resolvedAliases = alias
      builder = new ServerComponentBuilder(projectRoot, { ...options, alias })
    },

    buildStart() {
      if (!builder)
        return

      const isProduction = process.env.NODE_ENV === 'production'
      const cacheDirs = [
        path.join(projectRoot, 'dist', 'cache', 'og'),
        path.join(projectRoot, 'dist', 'cache', 'images'),
      ]

      if (isProduction)
        cacheDirs.push('/tmp/rari-og-cache', '/tmp/rari-image-cache')

      for (const dir of cacheDirs) {
        try {
          if (fs.existsSync(dir))
            fs.rmSync(dir, { recursive: true, force: true })
        }
        catch (error) {
          console.warn(`[rari] Failed to clear cache ${dir}:`, error)
        }
      }

      const srcDir = path.join(projectRoot, 'src')
      if (fs.existsSync(srcDir))
        scanDirectory(srcDir, builder)
    },

    async closeBundle() {
      if (builder) {
        await builder.buildServerComponents()
        await builder.buildSSRClientComponents()

        try {
          const { generateRobotsFile } = await import('../router/robots-generator')
          await generateRobotsFile({
            appDir: path.join(projectRoot, 'src', 'app'),
            outDir: path.join(projectRoot, 'dist'),
          })
        }
        catch (error) {
          console.warn('[rari] Failed to generate robots.txt:', error)
        }

        try {
          const { generateSitemapFiles } = await import('../router/sitemap-generator')
          await generateSitemapFiles({
            appDir: path.join(projectRoot, 'src', 'app'),
            outDir: path.join(projectRoot, 'dist'),
            aliases: resolvedAliases,
          })
        }
        catch (error) {
          console.warn('[rari] Failed to generate sitemap:', error)
        }

        try {
          const { generateFeedFile } = await import('../router/feed-generator')
          await generateFeedFile({
            appDir: path.join(projectRoot, 'src', 'app'),
            outDir: path.join(projectRoot, 'dist'),
            aliases: resolvedAliases,
          })
        }
        catch (error) {
          console.warn('[rari] Failed to generate feed:', error)
        }
      }
    },

    async handleHotUpdate({ file }) {
      if (!builder || !isDev)
        return

      const relativePath = path.relative(projectRoot, file).replace(BACKSLASH_REGEX, '/')
      if (!relativePath.startsWith('src/') || !TSX_EXT_REGEX.test(relativePath))
        return

      try {
        const content = await fs.promises.readFile(file, 'utf-8')
        const isTracked = builder.hasComponent(file)
        const eligible = isEligibleServerComponent(file, content, builder)

        if (!eligible) {
          if (isTracked)
            builder.removeComponent(file)

          return
        }

        if (!isTracked)
          builder.addServerComponent(file, content)

        await builder.rebuildComponent(file)
      }
      catch (error) {
        console.error(`[rari] Build: Error rebuilding ${relativePath}:`, error)
      }
    },
  }
}
