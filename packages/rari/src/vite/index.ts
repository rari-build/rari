import type { CSSModulesOptions, Plugin, UserConfig } from 'vite-plus'
import type { ProxyPluginOptions } from '../proxy/vite-plugin'
import type { ModuleAnalysis } from './directives'
import type { ServerBuildOptions } from './server-build'
import type { ServerCacheConfig, ServerCacheLayerConfig } from './server-config'
import { Buffer } from 'node:buffer'
import { spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import { DEFAULT_DEVICE_SIZES, DEFAULT_FORMATS, DEFAULT_IMAGE_SIZES, DEFAULT_MAX_CACHE_SIZE, DEFAULT_MINIMUM_CACHE_TTL, DEFAULT_QUALITY_LEVELS } from '../image/constants'
import { rariProxy } from '../proxy/vite-plugin'
import { rariRouter } from '../router/vite-plugin'
import {
  BACKSLASH_REGEX,
  EXPORT_NAMED_DECLARATION_REGEX,
  EXTENSION_REGEX,
  HTTP_PROTOCOL_REGEX,
  TSX_EXT_REGEX,
  WINDOWS_PATH_REGEX,
} from '../shared/regex-constants'
import { resolveIndexFile, resolveWithExtensions } from '../shared/utils/file-resolver'
import {
  buildNamespaceClientReferenceReplacement,
  NAMESPACE_IMPORT_LINE_REGEX,
} from './client-import-transform'
import { getComponentId } from './component-ids'
import { hasDefaultExport } from './directives'
import { HMRCoordinator } from './hmr-coordinator'
import { parseHtmlEntryImports } from './html-entry-imports'
import { scanForImageUsage } from './image-scanner'
import { collectClientComponentPaths, invalidateModuleCachePath, ModuleAnalysisCache, resolveModuleCachePath } from './module-analysis-cache'
import { createServerBuildPlugin, isServerComponentFromAnalysis, RARI_CSS_MODULES_PATTERN, scanDirectory, ServerComponentBuilder } from './server-build'
import { normalizeScanDirs } from './source-file-walker'
import { getUseCacheTransform } from './use-cache-loader'

const DIST_NOT_BUILT_ERROR = '[rari] Runtime dist not built. Run `pnpm build` in the rari package first.'

const IMPORT_TYPE_SPECIFIER_REGEX = /import\s+type\s+(\{[^}]+\})\s+from\s+["']\.\.?\/([^"']+)["'];?/g
const IMPORT_TYPE_NAMESPACE_REGEX = /import\s+type\s+(\*\s+as\s+\w+)\s+from\s+["']\.\.?\/([^"']+)["'];?/g
const IMPORT_TYPE_DEFAULT_REGEX = /import\s+type\s+(\w+)\s+from\s+["']\.\.?\/([^"']+)["'];?/g
const IMPORT_SPECIFIER_REGEX = /import\s+(\{[^}]+\})\s+from\s+["']\.\.?\/([^"']+)["'];?/g
const IMPORT_NAMESPACE_REGEX = /import\s+(\*\s+as\s+\w+)\s+from\s+["']\.\.?\/([^"']+)["'];?/g
const IMPORT_DEFAULT_REGEX = /import\s+(\w+)\s+from\s+["']\.\.?\/([^"']+)["'];?/g
const IMPORT_SIDE_EFFECT_REGEX = /import\s+["']\.\.?\/([^"']+)["'];?/g
const NAMED_EXPORT_REGEX = /export\s*\{([^}]+)\}/g
const AS_SPLIT_REGEX = /\s+as\s+/
const EXPORT_DEFAULT_FUNCTION_OR_CLASS_REGEX = /export\s+default\s+(?:function|class)\s+\w+/
const EXPORT_DEFAULT_FUNCTION_DECL_REGEX = /export\s+default\s+(?:async\s+)?function\s+(\w+)/
const EXPORT_DEFAULT_VALUE_REGEX = /export\s+default\s+([^;]+)/
const EXPORT_DECLARATION_REGEX = /export\s+(?:async\s+)?(?:const|let|var|function|class)\s+(\w+)/g
const USE_CLIENT_DIRECTIVE_REGEX = /^['"]use client['"];?\s*$/gm
const IMPORT_REGEX = /import\s+["']([^"']+)["']/g
const IMPORT_LINE_REGEX = /^\s*import\s+(?:(\w+)(?:\s*,\s*\{\s*(?:(\w+(?:\s*,\s*\w+)*)\s*)?\})?|\{\s*(\w+(?:\s*,\s*\w+)*)\s*\})\s+from\s+['"]([./@][^'"]+)['"].*$/

const REACT_IMPORT_REGEX = /import\s+\{[^}]*\}\s+from\s+['"]react['"]/
const REACT_IMPORT_WITH_DEFAULT_REGEX = /import\s+[^,\s]+\s*,\s*\{[^}]*\}\s+from\s+['"]react['"]/
const REACT_IMPORT_MATCH_REGEX = /import React(,\s*\{([^}]*)\})?\s+from\s+['"]react['"];?/
const IMPORT_PATH_REGEX = /import\s+["']([^"']+)["']/g
const RSC_CLIENT_IMPORT_REGEX = /from(\s*)(['"])(?:\.\/vendor\/react-flight-client\/index|rari\/runtime\/vendor\/react-flight-client\/index)\.mjs\2/g
const JSX_TEST_REGEX = /\bJSX\b/
const IMPORT_SPECIFIERS_REGEX = /\{([^}]*)\}/
const USE_CLIENT_DIRECTIVE_LINE_REGEX = /^['"]use client['"];?\s*\n/

interface ClientReferenceSpecifier {
  bindingName: string
  exportName: string
}

function parseClientImportSpecifiers(line: string, importedDefault?: string): ClientReferenceSpecifier[] {
  const specifiers: ClientReferenceSpecifier[] = []

  if (importedDefault)
    specifiers.push({ bindingName: importedDefault, exportName: 'default' })

  const namedBlock = line.match(/\{([^}]+)\}/)?.[1]
  if (namedBlock) {
    for (const part of namedBlock.split(',')) {
      const trimmed = part.trim()
      if (!trimmed)
        continue

      const asParts = trimmed.split(/\s+as\s+/i)
      if (asParts.length === 2 && asParts[0] && asParts[1]) {
        specifiers.push({
          bindingName: asParts[1].trim(),
          exportName: asParts[0].trim(),
        })
      }
      else {
        specifiers.push({ bindingName: trimmed, exportName: trimmed })
      }
    }
  }

  return specifiers
}

function buildClientReferenceReplacement(
  specifiers: ClientReferenceSpecifier[],
  resolvedImportPath: string,
): string {
  return `import { registerClientReference } from "react-server-dom-rari/server";
${specifiers.map(({ bindingName, exportName }) => `const ${bindingName} = registerClientReference(
  function() {
    throw new Error("Attempted to call ${bindingName} from the server but it's on the client. It can only be rendered as a Component or passed to props of a Client Component.");
  },
  ${JSON.stringify(resolvedImportPath)},
  ${JSON.stringify(exportName)}
);`).join('\n')}`
}

export interface RouterPluginOptions {
  appDir?: string
  extensions?: string[]
}

export interface RariOptions {
  projectRoot?: string
  serverBuild?: ServerBuildOptions
  serverHandler?: boolean
  proxy?: ProxyPluginOptions | false
  router?: RouterPluginOptions | false
  images?: {
    remotePatterns?: Array<{
      protocol?: 'http' | 'https'
      hostname: string
      port?: string
      pathname?: string
      search?: string
    }>
    localPatterns?: Array<{
      pathname: string
      search?: string
    }>
    deviceSizes?: number[]
    imageSizes?: number[]
    formats?: ('avif' | 'webp')[]
    qualityAllowlist?: number[]
    minimumCacheTTL?: number
    maxCacheSize?: number
  }
  csp?: {
    scriptSrc?: string[]
    styleSrc?: string[]
    imgSrc?: string[]
    fontSrc?: string[]
    connectSrc?: string[]
    defaultSrc?: string[]
    workerSrc?: string[]
  }
  cacheControl?: {
    routes: Record<string, string>
  }
  cache?: ServerCacheConfig
  experimental?: {
    useCache?: boolean
    useCacheRemote?: ServerCacheLayerConfig
  }
}

const DEFAULT_IMAGE_CONFIG = {
  remotePatterns: [],
  localPatterns: [],
  deviceSizes: DEFAULT_DEVICE_SIZES,
  imageSizes: DEFAULT_IMAGE_SIZES,
  formats: DEFAULT_FORMATS,
  qualityAllowlist: DEFAULT_QUALITY_LEVELS,
  minimumCacheTTL: DEFAULT_MINIMUM_CACHE_TTL,
  maxCacheSize: DEFAULT_MAX_CACHE_SIZE,
}

const runtimeFileCache = new Map<string, string>()

async function loadRuntimeFile(filename: string): Promise<string> {
  const cached = runtimeFileCache.get(filename)
  if (cached)
    return cached

  const currentFileUrl = import.meta.url
  const currentFilePath = fileURLToPath(currentFileUrl)
  const currentDir = path.dirname(currentFilePath)

  const possiblePaths = [
    path.join(currentDir, 'runtime', filename),
    path.join(currentDir, '../runtime', filename),
  ]

  for (const filePath of possiblePaths) {
    try {
      let content = await fs.promises.readFile(filePath, 'utf-8')

      if (filePath.endsWith('.ts')) {
        content = content.replace(
          IMPORT_TYPE_SPECIFIER_REGEX,
          (match, specifier, modulePath) => `import type ${specifier} from "rari/${modulePath}";`,
        )

        content = content.replace(
          IMPORT_TYPE_NAMESPACE_REGEX,
          (match, specifier, modulePath) => `import type ${specifier} from "rari/${modulePath}";`,
        )

        content = content.replace(
          IMPORT_TYPE_DEFAULT_REGEX,
          (match, specifier, modulePath) => `import type ${specifier} from "rari/${modulePath}";`,
        )

        content = content.replace(
          IMPORT_SPECIFIER_REGEX,
          (match, specifier, modulePath) => `import ${specifier} from "rari/${modulePath}";`,
        )

        content = content.replace(
          IMPORT_NAMESPACE_REGEX,
          (match, specifier, modulePath) => `import ${specifier} from "rari/${modulePath}";`,
        )

        content = content.replace(
          IMPORT_DEFAULT_REGEX,
          (match, specifier, modulePath) => `import ${specifier} from "rari/${modulePath}";`,
        )

        content = content.replace(
          IMPORT_SIDE_EFFECT_REGEX,
          (match, modulePath) => `import "rari/${modulePath}";`,
        )
      }

      runtimeFileCache.set(filename, content)
      return content
    }
    catch (err: any) {
      if (err.code !== 'ENOENT' && err.code !== 'EISDIR') {
        console.warn(`[rari] Unexpected error reading ${filePath}:`, err)
      }
    }
  }

  throw new Error(`Could not find ${filename}. Tried: ${possiblePaths.join(', ')}`)
}

const RARI_DIST_DIR = path.dirname(fileURLToPath(import.meta.url))
const RARI_PACKAGE_ROOT = path.dirname(RARI_DIST_DIR)

function resolveRuntimeDistFile(filename: string): string | null {
  const possiblePaths = [
    path.join(RARI_DIST_DIR, 'runtime', filename),
    path.join(RARI_DIST_DIR, '../runtime', filename),
  ]

  for (const filePath of possiblePaths) {
    if (fs.existsSync(filePath))
      return filePath
  }

  return null
}

function isRariInternalFile(filePath: string): boolean {
  return filePath.startsWith(RARI_PACKAGE_ROOT)
}

async function loadRscClientRuntime(): Promise<string> {
  return loadRuntimeFile('rsc-client-runtime.mjs')
}

async function loadEntryClient(imports: string, registrations: string): Promise<string> {
  const template = await loadRuntimeFile('entry-client.mjs')
  return template
    .replace('/*! @preserve CLIENT_COMPONENT_IMPORTS_PLACEHOLDER */', imports)
    .replace('/*! @preserve CLIENT_COMPONENT_REGISTRATIONS_PLACEHOLDER */', registrations)
}

async function loadRscReferences(): Promise<string> {
  return loadRuntimeFile('rsc-references.mjs')
}

async function writeImageConfig(projectRoot: string, options: RariOptions): Promise<void> {
  const srcDir = path.join(projectRoot, 'src')
  const imageManifest = await scanForImageUsage(srcDir)

  const imageConfig = {
    ...DEFAULT_IMAGE_CONFIG,
    ...options.images,
    preoptimizeManifest: imageManifest.images,
  }

  const distDir = path.join(projectRoot, 'dist')
  const serverDir = path.join(distDir, 'server')
  if (!fs.existsSync(serverDir))
    fs.mkdirSync(serverDir, { recursive: true })

  const configPath = path.join(serverDir, 'image.json')
  fs.writeFileSync(configPath, JSON.stringify(imageConfig, null, 2))
}

export function defineRariOptions(config: RariOptions): RariOptions {
  return config
}

export function rari(options: RariOptions = {}): Plugin[] {
  const componentTypeCache = new Map<string, 'client' | 'server' | 'unknown'>()
  const clientComponents = new Set<string>()
  const moduleAnalysisCache = new ModuleAnalysisCache()
  let devServerComponentBuilder: ServerComponentBuilder | null = null
  let rustServerProcess: any = null

  let hmrCoordinator: HMRCoordinator | null = null
  const resolvedAlias: Record<string, string> = {}

  function getComponentType(filePath: string): 'client' | 'server' | 'unknown' | undefined {
    return componentTypeCache.get(resolveModuleCachePath(filePath))
  }

  function setComponentType(filePath: string, type: 'client' | 'server' | 'unknown'): void {
    componentTypeCache.set(resolveModuleCachePath(filePath), type)
  }

  function deleteComponentType(filePath: string): void {
    invalidateModuleCachePath(componentTypeCache, filePath)
  }

  function addTrackedClientComponent(filePath: string): void {
    clientComponents.add(resolveModuleCachePath(filePath))
  }

  function hasTrackedClientComponent(filePath: string): boolean {
    return clientComponents.has(resolveModuleCachePath(filePath))
  }

  function removeTrackedClientComponent(filePath: string): void {
    clientComponents.delete(filePath)
    try {
      clientComponents.delete(fs.realpathSync(filePath))
    }
    catch {
      clientComponents.delete(path.resolve(filePath))
    }
  }

  function getKnownClientComponentPaths(): Set<string> {
    const paths = new Set(clientComponents)

    if (devServerComponentBuilder) {
      for (const componentPath of devServerComponentBuilder.getClientComponentPaths())
        paths.add(componentPath)
    }

    return paths
  }

  function getModuleDirectives(id: string): { hasUseServer: boolean, hasUseClient: boolean } {
    const result = { hasUseServer: false, hasUseClient: false }

    const normalizedId = id.replace(BACKSLASH_REGEX, '/')
    if (!TSX_EXT_REGEX.test(normalizedId) || !normalizedId.includes('/src/'))
      return result

    try {
      const analysis = moduleAnalysisCache.get(id)
      result.hasUseServer = analysis.directives.hasUseServer
      result.hasUseClient = analysis.directives.hasUseClient
    }
    catch {}

    return result
  }

  let htmlEntryImports: Set<string> | null = null
  let lastIndexHtmlMtime: number | null = null

  function getHtmlEntryImports(): ReadonlySet<string> {
    if (devServerComponentBuilder)
      return devServerComponentBuilder.getHtmlOnlyImports()

    const projectRoot = options.projectRoot || process.cwd()
    const indexHtmlPath = path.join(projectRoot, 'index.html')

    try {
      const mtime = fs.statSync(indexHtmlPath).mtimeMs
      if (htmlEntryImports !== null && mtime === lastIndexHtmlMtime)
        return htmlEntryImports
      lastIndexHtmlMtime = mtime
    }
    catch {
      if (htmlEntryImports === null)
        htmlEntryImports = new Set()

      return htmlEntryImports
    }

    htmlEntryImports = parseHtmlEntryImports(projectRoot)
    return htmlEntryImports
  }

  function isServerComponent(filePath: string): boolean {
    if (filePath.includes('node_modules') || isRariInternalFile(filePath))
      return false

    const resolvedPath = resolveModuleCachePath(filePath)

    try {
      const analysis = moduleAnalysisCache.get(filePath)
      return isServerComponentFromAnalysis(
        resolvedPath,
        analysis,
        getHtmlEntryImports(),
        resolvedPath,
      )
    }
    catch {
      return false
    }
  }

  function parseExportedNames(code: string, analysis?: ModuleAnalysis): string[] {
    try {
      const exportedNames: string[] = []
      const namedExportMatch = code.matchAll(NAMED_EXPORT_REGEX)
      for (const match of namedExportMatch) {
        const exports = match[1].split(',')
        for (const exp of exports) {
          const trimmed = exp.trim()
          const parts = trimmed.split(AS_SPLIT_REGEX)
          const exportedName = parts.at(-1)?.trim()
          if (exportedName)
            exportedNames.push(exportedName)
        }
      }

      if (EXPORT_DEFAULT_FUNCTION_OR_CLASS_REGEX.test(code))
        exportedNames.push('default')
      else if (analysis?.hasDefaultExport ?? hasDefaultExport(code))
        exportedNames.push('default')

      const declarationExports = code.matchAll(EXPORT_DECLARATION_REGEX)
      for (const match of declarationExports) {
        if (match[1])
          exportedNames.push(match[1])
      }

      return [...new Set(exportedNames)]
    }
    catch {
      return []
    }
  }

  function transformServerModule(code: string, id: string, analysis: ModuleAnalysis): string {
    if (!analysis.topLevelUseServer)
      return code

    const exportedNames = parseExportedNames(code, analysis)
    if (exportedNames.length === 0)
      return code

    const idJson = JSON.stringify(id)
    let newCode = code
    newCode
      += '\n\nimport {registerServerReference} from "react-server-dom-rari/server";\n'

    for (const name of exportedNames) {
      if (name === 'default') {
        const functionDeclMatch = code.match(EXPORT_DEFAULT_FUNCTION_DECL_REGEX)

        if (functionDeclMatch) {
          const functionName = functionDeclMatch[1]
          newCode += `\n// Register server reference for default export\n`
          newCode += `registerServerReference(${functionName}, ${idJson}, ${JSON.stringify(name)});\n`
        }
        else {
          const match = code.match(EXPORT_DEFAULT_VALUE_REGEX)
          if (match) {
            const exportedValue = match[1].trim()
            const tempVarName = '__default_export__'
            newCode = newCode.replace(
              EXPORT_DEFAULT_VALUE_REGEX,
              `const ${tempVarName} = ${exportedValue};\nexport default ${tempVarName}`,
            )
            newCode += `\n// Register server reference for default export\n`
            newCode += `if (typeof ${tempVarName} === "function") {\n`
            newCode += `  registerServerReference(${tempVarName}, ${idJson}, ${JSON.stringify(name)});\n`
            newCode += `}\n`
          }
        }
      }
      else {
        newCode += `\n// Register server reference for ${name}\n`
        newCode += `if (typeof ${name} === "function") {\n`
        newCode += `  registerServerReference(${name}, ${idJson}, ${JSON.stringify(name)});\n`
        newCode += `}\n`
      }
    }

    newCode += `

if (import.meta.hot) {
  import.meta.hot.accept(() => {
  });
}`

    return newCode
  }

  function transformClientModule(code: string, id: string, analysis: ModuleAnalysis): string {
    const projectRoot = options.projectRoot || process.cwd()
    const isServerComp = isServerComponent(id)

    if (analysis.topLevelUseServer) {
      const exportedNames = parseExportedNames(code, analysis)
      if (exportedNames.length === 0)
        return ''

      const moduleId = getComponentId(id, projectRoot)
      const moduleIdJson = JSON.stringify(moduleId)

      let newCode = 'import { createServerReference } from "rari/runtime/actions";\n'

      for (const name of exportedNames) {
        if (name === 'default')
          newCode += `export default createServerReference("default", ${moduleIdJson}, "default");\n`
        else
          newCode += `export const ${name} = createServerReference("${name}", ${moduleIdJson}, "${name}");\n`
      }

      return newCode
    }

    if (isServerComp) {
      console.warn(`[rari] Server component ${id} should not be imported in client bundle`)
      return ''
    }

    if (!analysis.topLevelUseClient)
      return code

    const exportedNames = parseExportedNames(code, analysis)
    if (exportedNames.length === 0)
      return ''

    const idJson = JSON.stringify(id)
    let newCode
      = 'import {registerClientReference} from "react-server-dom-rari/server";\n'

    for (const name of exportedNames) {
      if (name === 'default') {
        const errorMsg = `Attempted to call the default export of ${id} from the server but it's on the client. It's not possible to invoke a client function from the server, it can only be rendered as a Component or passed to props of a Client Component.`
        newCode += 'export default '
        newCode += 'registerClientReference(function() {'
        newCode += `throw new Error(${JSON.stringify(errorMsg)});`
      }
      else {
        const errorMsg = `Attempted to call ${name}() from the server but ${name} is on the client. It's not possible to invoke a client function from the server, it can only be rendered as a Component or passed to props of a Client Component.`
        newCode += `export const ${name} = `
        newCode += 'registerClientReference(function() {'
        newCode += `throw new Error(${JSON.stringify(errorMsg)});`
      }
      newCode += '},'
      newCode += `${idJson},`
      newCode += `${JSON.stringify(name)});\n`
    }

    return newCode
  }

  function transformClientModuleForClient(code: string, _id: string, analysis: ModuleAnalysis): string {
    if (!analysis.topLevelUseClient)
      return code

    const exportedNames = parseExportedNames(code, analysis)
    if (exportedNames.length === 0)
      return code

    return code.replace(USE_CLIENT_DIRECTIVE_REGEX, '')
  }

  function resolveImportToFilePath(
    importPath: string,
    importerPath: string,
  ): string {
    let resolvedImportPath = importPath
    for (const [alias, replacement] of Object.entries(resolvedAlias)) {
      if (importPath.startsWith(`${alias}/`)) {
        resolvedImportPath = importPath.replace(alias, replacement)
        break
      }
      else if (importPath === alias) {
        resolvedImportPath = replacement
        break
      }
    }

    const resolvedPath = path.resolve(path.dirname(importerPath), resolvedImportPath)

    const extensions = ['.tsx', '.jsx', '.ts', '.js']
    const withExt = resolveWithExtensions(resolvedPath, extensions)
    if (withExt)
      return withExt

    const indexFile = resolveIndexFile(resolvedPath, extensions)
    if (indexFile)
      return indexFile

    return `${resolvedPath}.tsx`
  }

  let rustServerReady = false

  async function checkRustServerHealth(): Promise<boolean> {
    const serverPort = process.env.SERVER_PORT
      ? Number(process.env.SERVER_PORT)
      : Number(process.env.PORT || process.env.RSC_PORT || 3000)
    const baseUrl = `http://localhost:${serverPort}`

    try {
      const healthResponse = await fetch(`${baseUrl}/_rari/health`, {
        signal: AbortSignal.timeout(1000),
      })
      const isHealthy = healthResponse.ok
      rustServerReady = isHealthy
      return isHealthy
    }
    catch {
      rustServerReady = false
      return false
    }
  }

  const mainPlugin: Plugin = {
    name: 'rari',

    config(config: UserConfig, { command }) {
      config.define = config.define || {}

      if (command === 'serve' || process.env.RARI_SERVER_URL || process.env.RARI_HOST) {
        const rariServerPort = process.env.SERVER_PORT
          ? Number(process.env.SERVER_PORT)
          : Number(process.env.PORT || process.env.RSC_PORT || 3000)

        let serverUrl: string
        if (process.env.RARI_SERVER_URL) {
          serverUrl = process.env.RARI_SERVER_URL
        }
        else if (process.env.RARI_HOST) {
          const host = process.env.RARI_HOST.startsWith('http')
            ? process.env.RARI_HOST
            : `http://${process.env.RARI_HOST}`
          const hostnamePart = host.replace(HTTP_PROTOCOL_REGEX, '')
          serverUrl = hostnamePart.includes(':') ? host : `${host}:${rariServerPort}`
        }
        else {
          serverUrl = `http://localhost:${rariServerPort}`
        }

        config.define['import.meta.env.RARI_SERVER_URL'] = JSON.stringify(serverUrl)
      }

      const existingCssModules = typeof config.css?.modules === 'object' ? config.css.modules : {}
      config.css = {
        ...config.css,
        transformer: config.css?.transformer ?? 'lightningcss' as const,
        modules: { ...existingCssModules, pattern: RARI_CSS_MODULES_PATTERN } as CSSModulesOptions,
      }

      if (command === 'build') {
        const projectRoot = options.projectRoot || process.cwd()
        const indexHtmlPath = path.join(projectRoot, 'index.html')

        if (fs.existsSync(indexHtmlPath)) {
          try {
            const htmlContent = fs.readFileSync(indexHtmlPath, 'utf-8')
            const htmlImports: Array<{ path: string, name: string }> = []

            for (const match of htmlContent.matchAll(IMPORT_REGEX)) {
              const importPath = match[1]
              if (importPath.startsWith('/src/') && TSX_EXT_REGEX.test(importPath)) {
                const relativePath = importPath.slice(1)
                const filename = path.basename(relativePath, path.extname(relativePath))
                htmlImports.push({ path: relativePath, name: filename })
              }
            }

            if (htmlImports.length > 0) {
              config.build = config.build || {}
              config.build.rolldownOptions = config.build.rolldownOptions || {}

              const existingInput = config.build.rolldownOptions.input || { main: './index.html' }
              let inputObj: Record<string, string>
              if (typeof existingInput === 'string')
                inputObj = { main: existingInput }
              else if (Array.isArray(existingInput))
                inputObj = { main: existingInput[0] || './index.html' }
              else
                inputObj = { ...existingInput }

              htmlImports.forEach(({ path: importPath, name }) => {
                inputObj[name] = `./${importPath}`
              })

              config.build.rolldownOptions.input = inputObj
            }
          }
          catch (error) {
            console.warn('[rari] Error parsing index.html for build inputs:', error)
          }
        }
      }

      config.resolve = config.resolve || {}
      const existingDedupe = Array.isArray((config.resolve as any).dedupe)
        ? ((config.resolve as any).dedupe as string[])
        : []
      const toAdd = ['react', 'react-dom'];
      (config.resolve as any).dedupe = [...new Set([...(existingDedupe || []), ...toAdd])]

      let existingAlias: Array<{
        find: string | RegExp
        replacement: string
      }> = []

      if (Array.isArray((config.resolve as any).alias)) {
        existingAlias = (config.resolve as any).alias
      }
      else if ((config.resolve as any).alias && typeof (config.resolve as any).alias === 'object') {
        existingAlias = Object.entries((config.resolve as any).alias).map(([key, value]) => ({
          find: key,
          replacement: value as string,
        }))
      }

      const aliasFinds = new Set(existingAlias.map(a => String(a.find)))
      try {
        const reactPath = fileURLToPath(import.meta.resolve('react'))
        const reactDomClientPath = fileURLToPath(import.meta.resolve('react-dom/client'))
        const reactJsxRuntimePath = fileURLToPath(import.meta.resolve('react/jsx-runtime'))
        const aliasesToAppend: Array<{ find: string, replacement: string }>
          = []
        if (!aliasFinds.has('react/jsx-runtime')) {
          aliasesToAppend.push({
            find: 'react/jsx-runtime',
            replacement: reactJsxRuntimePath,
          })
        }
        try {
          const reactJsxDevRuntimePath = fileURLToPath(import.meta.resolve(
            'react/jsx-dev-runtime',
          ))
          if (!aliasFinds.has('react/jsx-dev-runtime')) {
            aliasesToAppend.push({
              find: 'react/jsx-dev-runtime',
              replacement: reactJsxDevRuntimePath,
            })
          }
        }
        catch (err) {
          if ((err as any)?.code !== 'ENOENT') {
            console.warn('[rari] Unexpected error resolving react/jsx-dev-runtime:', err)
          }
        }
        if (!aliasFinds.has('react'))
          aliasesToAppend.push({ find: 'react', replacement: reactPath })
        if (!aliasFinds.has('react-dom/client')) {
          aliasesToAppend.push({
            find: 'react-dom/client',
            replacement: reactDomClientPath,
          })
        }
        if (aliasesToAppend.length > 0) {
          (config.resolve as any).alias = [
            ...existingAlias,
            ...aliasesToAppend,
          ]
        }
      }
      catch (err) {
        if ((err as any)?.code !== 'ENOENT') {
          console.warn('[rari] Unexpected error configuring React aliases:', err)
        }
      }

      config.environments = config.environments || {}

      config.environments.rsc = {
        resolve: {
          conditions: ['react-server', 'node', 'import'],
        },
        ...config.environments.rsc,
      }

      config.environments.ssr = {
        resolve: {
          conditions: ['node', 'import'],
        },
        ...config.environments.ssr,
      }

      config.environments.client = {
        resolve: {
          conditions: ['browser', 'import'],
        },
        ...config.environments.client,
      }

      config.optimizeDeps = config.optimizeDeps || {}
      config.optimizeDeps.include = config.optimizeDeps.include || []

      const coreOptimizeDeps = [
        'react',
        'react-dom',
        'react-dom/client',
        'react-dom/server',
        'react/jsx-runtime',
        'react/jsx-dev-runtime',
      ]

      for (const dep of coreOptimizeDeps) {
        if (!config.optimizeDeps.include.includes(dep))
          config.optimizeDeps.include.push(dep)
      }

      config.optimizeDeps.exclude = config.optimizeDeps.exclude || []
      if (!config.optimizeDeps.exclude.includes('rari'))
        config.optimizeDeps.exclude.push('rari')

      if (command === 'build') {
        for (const envName of ['rsc', 'ssr', 'client']) {
          const env = config.environments[envName]
          if (env && env.build)
            env.build.rolldownOptions = env.build.rolldownOptions || {}
        }
      }

      config.server = config.server || {}
      config.server.proxy = config.server.proxy || {}

      const serverPort = process.env.SERVER_PORT
        ? Number(process.env.SERVER_PORT)
        : Number(process.env.PORT || process.env.RSC_PORT || 3000)

      config.server.proxy['/api'] = {
        target: `http://localhost:${serverPort}`,
        changeOrigin: true,
        secure: false,
        ws: true,
      }

      config.server.proxy['/_rari'] = {
        target: `http://localhost:${serverPort}`,
        changeOrigin: true,
        secure: false,
        ws: true,
      }

      if (command === 'build') {
        config.build = config.build || {}
        config.build.rolldownOptions = config.build.rolldownOptions || {}

        if (!config.build.rolldownOptions.input) {
          config.build.rolldownOptions.input = {
            main: './index.html',
          }
        }

        config.build.rolldownOptions.output = config.build.rolldownOptions.output || {}

        const outputs = Array.isArray(config.build.rolldownOptions.output)
          ? config.build.rolldownOptions.output
          : [config.build.rolldownOptions.output]

        for (const output of outputs) {
          // Initialize codeSplitting as an object if it's not already
          if (output.codeSplitting !== false && typeof output.codeSplitting !== 'object') {
            output.codeSplitting = {}
          }

          // Only configure groups if codeSplitting is an object
          if (typeof output.codeSplitting === 'object') {
            output.codeSplitting.groups = output.codeSplitting.groups || []

            const userGroups = output.codeSplitting.groups

            output.codeSplitting.groups.push({
              name(moduleId: string) {
                if (moduleId.includes('node_modules')) {
                  for (const group of userGroups) {
                    if (group.test) {
                      let testResult = false
                      if (typeof group.test === 'function') {
                        testResult = Boolean(group.test(moduleId))
                      }
                      else if (group.test instanceof RegExp) {
                        testResult = group.test.test(moduleId)
                      }
                      else if (typeof group.test === 'string') {
                        testResult = moduleId.includes(group.test)
                      }

                      if (testResult) {
                        return null
                      }
                    }
                  }

                  if (moduleId.includes('node_modules/react-dom'))
                    return 'react-dom'
                  if (moduleId.includes('node_modules/react'))
                    return 'react'

                  return 'vendor'
                }

                return null
              },
            })
          }

          if (!output.chunkFileNames) {
            output.chunkFileNames = (chunkInfo) => {
              const hasServerAction = chunkInfo.moduleIds?.some((id: string) => {
                const directives = getModuleDirectives(id)
                return directives.hasUseServer
              })

              if (hasServerAction)
                return 'client/actions/[name]-[hash].js'

              const isClientComponent = chunkInfo.moduleIds?.some((id: string) => {
                const directives = getModuleDirectives(id)
                return directives.hasUseClient
              })

              if (isClientComponent)
                return 'client/components/[name]-[hash].js'

              return 'assets/[name]-[hash].js'
            }
          }
        }
      }

      if (config.environments && config.environments.client) {
        if (!config.environments.client.build)
          config.environments.client.build = {}
        if (!config.environments.client.build.rolldownOptions)
          config.environments.client.build.rolldownOptions = {}
        if (!config.environments.client.build.rolldownOptions.input)
          config.environments.client.build.rolldownOptions.input = {}

        if (!config.environments.client.build.rolldownOptions.external)
          config.environments.client.build.rolldownOptions.external = []

        const external = config.environments.client.build.rolldownOptions.external
        if (Array.isArray(external)) {
          if (!external.includes('react-server-dom-webpack/client'))
            external.push('react-server-dom-webpack/client')
        }
      }

      return config
    },

    configResolved(config) {
      const excludeAliases = new Set(['react', 'react-dom', 'react/jsx-runtime', 'react/jsx-dev-runtime', 'react-dom/client'])

      if (config.resolve?.alias) {
        const aliasConfig = config.resolve.alias
        if (Array.isArray(aliasConfig)) {
          aliasConfig.forEach((entry) => {
            if (typeof entry.find === 'string' && typeof entry.replacement === 'string' && !excludeAliases.has(entry.find))
              resolvedAlias[entry.find] = entry.replacement
          })
        }
        else if (typeof aliasConfig === 'object') {
          Object.entries(aliasConfig).forEach(([key, value]) => {
            if (typeof value === 'string' && !excludeAliases.has(key))
              resolvedAlias[key] = value
          })
        }
      }
    },

    async transform(code, id) {
      if (!TSX_EXT_REGEX.test(id))
        return null

      const originalCode = code
      let wasUseCacheTransformed = false
      if (options.experimental?.useCache) {
        const transform = await getUseCacheTransform()
        if (transform) {
          const useCacheResult = transform(code, id)
          if (useCacheResult) {
            code = useCacheResult
            wasUseCacheTransformed = true
          }
        }
      }

      const environment = (this as any).environment
      const moduleAnalysis = moduleAnalysisCache.get(id, originalCode)

      if (moduleAnalysis.topLevelUseClient) {
        setComponentType(id, 'client')
        addTrackedClientComponent(id)

        const lines = code.split('\n')

        for (const line of lines) {
          const namespaceMatch = line.match(NAMESPACE_IMPORT_LINE_REGEX)
          const importMatch = namespaceMatch ? null : line.match(IMPORT_LINE_REGEX)
          if (!namespaceMatch && !importMatch)
            continue

          const importPath = namespaceMatch?.[2] ?? importMatch![4]
          if (!importPath)
            continue

          const resolvedImportPath = resolveImportToFilePath(importPath, id)

          if (fs.existsSync(resolvedImportPath)) {
            setComponentType(resolvedImportPath, 'client')
            addTrackedClientComponent(resolvedImportPath)
          }
        }

        return transformClientModuleForClient(code, id, moduleAnalysis)
      }

      if (getComponentType(id) === 'client' || hasTrackedClientComponent(id))
        return transformClientModuleForClient(code, id, moduleAnalysis)

      if (isServerComponent(id)) {
        setComponentType(id, 'server')

        if (
          environment
          && (environment.name === 'rsc' || environment.name === 'ssr')
        ) {
          return transformServerModule(code, id, moduleAnalysis)
        }
        else {
          let clientTransformedCode = transformClientModule(code, id, moduleAnalysis)

          clientTransformedCode = `// HMR acceptance for server component
if (import.meta.hot) {
  import.meta.hot.accept();
}

if (typeof globalThis !== 'undefined') {
  if (!globalThis['~rari']) globalThis['~rari'] = {};
  globalThis['~rari'].serverComponents = globalThis['~rari'].serverComponents || new Set();
  globalThis['~rari'].serverComponents.add(${JSON.stringify(id)});
}

${clientTransformedCode}`

          return clientTransformedCode
        }
      }

      if (moduleAnalysis.topLevelUseServer) {
        setComponentType(id, 'server')

        if (
          environment
          && (environment.name === 'rsc' || environment.name === 'ssr')
        ) {
          return transformServerModule(code, id, moduleAnalysis)
        }
        else {
          return transformClientModule(code, id, moduleAnalysis)
        }
      }

      const cachedType = getComponentType(id)
      if (cachedType === 'server') {
        if (
          environment
          && (environment.name === 'rsc' || environment.name === 'ssr')
        ) {
          return transformServerModule(code, id, moduleAnalysis)
        }
        else {
          return transformClientModule(code, id, moduleAnalysis)
        }
      }
      if (cachedType === 'client')
        return transformClientModuleForClient(code, id, moduleAnalysis)

      setComponentType(id, 'unknown')

      const lines = code.split('\n')
      let modifiedCode = code
      let hasServerImports = false
      let needsReactImport = false
      const importingFileIsClient = id.includes('entry-client')

      for (const line of lines) {
        const namespaceMatch = line.match(NAMESPACE_IMPORT_LINE_REGEX)
        const importMatch = namespaceMatch ? null : line.match(IMPORT_LINE_REGEX)
        if (!namespaceMatch && !importMatch)
          continue

        const importedDefault = importMatch?.[1]
        const importPath = namespaceMatch?.[2] ?? importMatch![4]
        const resolvedImportPath = resolveImportToFilePath(importPath, id)

        const isClientComponent
          = getComponentType(resolvedImportPath) === 'client'
            || (fs.existsSync(resolvedImportPath)
              && moduleAnalysisCache.get(resolvedImportPath).topLevelUseClient)

        if (isClientComponent) {
          setComponentType(resolvedImportPath, 'client')
          addTrackedClientComponent(resolvedImportPath)
        }

        if (
          isClientComponent
          && environment
          && (environment.name === 'rsc' || environment.name === 'ssr')
        ) {
          if (!importingFileIsClient) {
            const originalImport = line

            if (namespaceMatch) {
              const clientRefReplacement = buildNamespaceClientReferenceReplacement(
                namespaceMatch[1],
                resolvedImportPath,
              )

              modifiedCode = modifiedCode.replace(
                originalImport,
                clientRefReplacement,
              )
              hasServerImports = true
              needsReactImport = true
              continue
            }

            const specifiers = parseClientImportSpecifiers(line, importedDefault)
            if (specifiers.length === 0)
              continue

            const clientRefReplacement = buildClientReferenceReplacement(
              specifiers,
              resolvedImportPath,
            )

            modifiedCode = modifiedCode.replace(
              originalImport,
              clientRefReplacement,
            )
            hasServerImports = true
            needsReactImport = true
          }
        }
      }

      if (hasServerImports) {
        const hasReactImport
          = modifiedCode.includes('import React')
            || modifiedCode.match(REACT_IMPORT_REGEX)
            || modifiedCode.match(REACT_IMPORT_WITH_DEFAULT_REGEX)

        let importsToAdd = ''

        if (needsReactImport && !hasReactImport)
          importsToAdd += `import React from 'react';\n`
        if (importsToAdd)
          modifiedCode = importsToAdd + modifiedCode
        if (!modifiedCode.includes('Suspense')) {
          const reactImportMatch = modifiedCode.match(REACT_IMPORT_MATCH_REGEX)
          if (reactImportMatch) {
            if (
              reactImportMatch[1]
              && !reactImportMatch[2].includes('Suspense')
            ) {
              modifiedCode = modifiedCode.replace(
                reactImportMatch[0],
                reactImportMatch[0].replace(IMPORT_SPECIFIERS_REGEX, `{ Suspense, $1 }`),
              )
            }
            else if (!reactImportMatch[1]) {
              modifiedCode = modifiedCode.replace(
                reactImportMatch[0],
                `import React, { Suspense } from 'react';`,
              )
            }
          }
        }

        const isDevMode = process.env.NODE_ENV !== 'production'
        const hasJsx
          = modifiedCode.includes('</')
            || modifiedCode.includes('/>')
            || JSX_TEST_REGEX.test(modifiedCode)

        if (hasJsx) {
          if (isDevMode) {
            modifiedCode = `'use client';\n\n${modifiedCode}`
            setComponentType(id, 'client')
          }
        }

        return modifiedCode
      }

      if (wasUseCacheTransformed)
        return code

      return null
    },

    async configureServer(server) {
      const projectRoot = options.projectRoot || process.cwd()
      const srcDir = path.join(projectRoot, 'src')
      await writeImageConfig(projectRoot, options)

      const discoverAndRegisterComponents = async () => {
        try {
          const builder = new ServerComponentBuilder(projectRoot, {
            outDir: 'dist',
            rscDir: 'server',
            manifestPath: 'server/manifest.json',
            serverConfigPath: 'server/config.json',
            alias: resolvedAlias,
            csp: options.csp,
            cacheControl: options.cacheControl,
            cache: options.cache,
            experimental: options.experimental,
            moduleAnalysisCache,
          })

          devServerComponentBuilder = builder

          if (!hmrCoordinator) {
            const serverPort = process.env.SERVER_PORT
              ? Number(process.env.SERVER_PORT)
              : Number(process.env.PORT || process.env.RSC_PORT || 3000)
            hmrCoordinator = new HMRCoordinator(builder, serverPort)
          }

          if (fs.existsSync(srcDir)) {
            const scanResult = scanDirectory(srcDir, builder, Object.values(resolvedAlias))

            if (scanResult.serverComponentPaths.length > 0) {
              server.ws.send({
                type: 'custom',
                event: 'rari:server-components-registry',
                data: { serverComponents: scanResult.serverComponentPaths },
              })
            }
          }

          const components
            = await builder.getTransformedComponentsForDevelopment()

          const serverPort = process.env.SERVER_PORT
            ? Number(process.env.SERVER_PORT)
            : Number(process.env.PORT || process.env.RSC_PORT || 3000)
          const baseUrl = `http://localhost:${serverPort}`

          for (const component of components) {
            try {
              const isAppRouterComponent = component.id.startsWith('app/')
              if (isAppRouterComponent)
                continue

              if (component.isAction)
                continue

              const registerResponse = await fetch(
                `${baseUrl}/_rari/register`,
                {
                  method: 'POST',
                  headers: {
                    'Content-Type': 'application/json',
                  },
                  body: JSON.stringify({
                    component_id: component.id,
                    component_code: component.code,
                  }),
                },
              )

              if (!registerResponse.ok) {
                const errorText = await registerResponse.text()
                throw new Error(
                  `HTTP ${registerResponse.status}: ${errorText}`,
                )
              }
            }
            catch (error) {
              console.error(
                `[rari] Runtime: Failed to register component ${component.id}:`,
                error instanceof Error ? error.message : String(error),
              )
            }
          }
        }
        catch (error) {
          console.error(
            '[rari] Runtime: Component discovery failed:',
            error instanceof Error ? error.message : String(error),
          )
        }
      }

      const ensureClientComponentsRegistered = async () => {
        try {
          const serverPort = process.env.SERVER_PORT
            ? Number(process.env.SERVER_PORT)
            : Number(process.env.PORT || process.env.RSC_PORT || 3000)
          const baseUrl = `http://localhost:${serverPort}`

          const clientComponentFiles = getKnownClientComponentPaths()

          for (const componentPath of clientComponentFiles) {
            const relativePath = path.relative(process.cwd(), componentPath)
            const componentName = path
              .basename(componentPath)
              .replace(EXTENSION_REGEX, '')

            try {
              await fetch(`${baseUrl}/_rari/register-client`, {
                method: 'POST',
                headers: {
                  'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                  component_id: componentName,
                  file_path: relativePath,
                  export_name: 'default',
                }),
              })
            }
            catch (error) {
              console.error(
                `[rari] Runtime: Failed to pre-register client component ${componentName}:`,
                error,
              )
            }
          }
        }
        catch (error) {
          console.error('[rari] Runtime: Failed to pre-register client components:', error)
        }
      }

      const startRustServer = async () => {
        if (rustServerProcess)
          return

        const { getBinaryPath, getInstallationInstructions } = await import(
          '../cli/platform',
        )

        let binaryPath: string
        try {
          binaryPath = getBinaryPath()
        }
        catch (error) {
          console.error('rari binary not found')
          console.error(`   ${(error as Error).message}`)
          console.error(getInstallationInstructions())
          return
        }

        const serverPort = process.env.SERVER_PORT
          ? Number(process.env.SERVER_PORT)
          : Number(process.env.PORT || process.env.RSC_PORT || 3000)
        const mode
          = process.env.NODE_ENV === 'production' ? 'production' : 'development'

        const vitePort = server.config.server.port || 5173

        const args = [
          '--mode',
          mode,
          '--port',
          serverPort.toString(),
          '--host',
          '127.0.0.1',
        ]

        rustServerProcess = spawn(binaryPath, args, {
          stdio: ['ignore', 'pipe', 'pipe'],
          cwd: process.cwd(),
          env: {
            ...process.env,
            RUST_LOG: process.env.RUST_LOG || 'error',
            RARI_VITE_PORT: vitePort.toString(),
          },
        })

        rustServerProcess.stdout?.on('data', (data: Buffer) => {
          const output = data.toString().trim()
          if (output)
            console.error(`${output}`)
        })

        rustServerProcess.stderr?.on('data', (data: Buffer) => {
          const output = data.toString().trim()
          if (output && !output.includes('warning'))
            console.error(`${output}`)
        })

        rustServerProcess.on('error', (error: Error) => {
          rustServerReady = false
          console.error('Failed to start rari server:', error.message)
          if (error.message.includes('ENOENT')) {
            console.error(
              '   Binary not found. Please ensure rari is properly installed.',
            )
          }
        })

        rustServerProcess.on('exit', (code: number, signal: string) => {
          rustServerProcess = null
          rustServerReady = false
          if (signal)
            console.error(`rari server stopped by signal ${signal}`)
          else if (code === 0)
            console.error('rari server stopped successfully')
          else if (code)
            console.error(`rari server exited with code ${code}`)
        })

        let serverReady = false
        for (let i = 0; i < 20; i++) {
          serverReady = await checkRustServerHealth()
          if (serverReady) {
            break
          }
          await new Promise(resolve => setTimeout(resolve, 500))
        }

        if (serverReady) {
          await discoverAndRegisterComponents()
          await ensureClientComponentsRegistered()
        }
        else {
          console.error(
            'Server failed to become ready for component registration',
          )
        }
      }

      const handleServerComponentHMR = async (filePath: string) => {
        try {
          if (!isServerComponent(filePath))
            return

          if (!devServerComponentBuilder) {
            await discoverAndRegisterComponents()
            if (!devServerComponentBuilder)
              return
          }

          const code = moduleAnalysisCache.getSource(filePath)
          devServerComponentBuilder.addServerComponent(filePath, code)

          const components
            = await devServerComponentBuilder.getTransformedComponentsForDevelopment()

          if (components.length === 0)
            return

          const serverPort = process.env.SERVER_PORT
            ? Number(process.env.SERVER_PORT)
            : Number(process.env.PORT || process.env.RSC_PORT || 3000)
          const baseUrl = `http://localhost:${serverPort}`

          for (const component of components) {
            try {
              const registerResponse = await fetch(
                `${baseUrl}/_rari/register`,
                {
                  method: 'POST',
                  headers: {
                    'Content-Type': 'application/json',
                  },
                  body: JSON.stringify({
                    component_id: component.id,
                    component_code: component.code,
                  }),
                },
              )

              if (!registerResponse.ok) {
                const errorText = await registerResponse.text()
                throw new Error(
                  `HTTP ${registerResponse.status}: ${errorText}`,
                )
              }
            }
            catch (error) {
              console.error(
                '[rari] Failed to register component',
                `${component.id}:`,
                error instanceof Error ? error.message : String(error),
              )
            }
          }
        }
        catch (error) {
          console.error(
            '[rari] Targeted HMR failed for',
            `${filePath}:`,
            error instanceof Error ? error.message : String(error),
          )
          setTimeout(discoverAndRegisterComponents, 1000)
        }
      }

      startRustServer().catch((error) => {
        console.error('[rari] Failed to start Rust server:', error)
      })

      server.middlewares.use(async (req, res, next) => {
        const acceptHeader = req.headers.accept
        const isRscRequest = acceptHeader && acceptHeader.includes('text/x-component')

        if (isRscRequest && req.url && !req.url.startsWith('/api') && !req.url.startsWith('/rsc') && !req.url.includes('.')) {
          if (!rustServerReady) {
            const isHealthy = await checkRustServerHealth()
            if (!isHealthy) {
              const maxWait = 10000
              const startWait = Date.now()
              const checkInterval = 100

              while ((Date.now() - startWait) < maxWait) {
                if (await checkRustServerHealth())
                  break

                await new Promise(resolve => setTimeout(resolve, checkInterval))
              }

              if (!rustServerReady) {
                console.error('[rari] Rust server not ready, cannot proxy RSC request')
                if (!res.headersSent) {
                  res.statusCode = 503
                  res.end('Server not ready')
                }

                return
              }
            }
          }

          const serverPort = process.env.SERVER_PORT
            ? Number(process.env.SERVER_PORT)
            : Number(process.env.PORT || process.env.RSC_PORT || 3000)

          const targetUrl = `http://localhost:${serverPort}${req.url}`

          try {
            const headers: Record<string, string> = {}
            for (const [key, value] of Object.entries(req.headers)) {
              if (typeof value === 'string')
                headers[key] = value
              else if (Array.isArray(value))
                headers[key] = value.join(',')
            }
            headers.host = `localhost:${serverPort}`
            headers['accept-encoding'] = 'identity'

            const response = await fetch(targetUrl, {
              method: req.method,
              headers,
            })

            res.statusCode = response.status
            response.headers.forEach((value, key) => {
              if (key.toLowerCase() !== 'content-encoding')
                res.setHeader(key, value)
            })

            if (response.body) {
              const reader = response.body.getReader()

              try {
                while (true) {
                  const { done, value } = await reader.read()
                  if (done)
                    break
                  res.write(Buffer.from(value))
                }
                res.end()
              }
              catch (streamError) {
                console.error('[rari] Stream error:', streamError)
                if (!res.headersSent)
                  res.statusCode = 500
                res.end()
              }
            }
            else {
              res.end()
            }

            return
          }
          catch (error) {
            console.error('[rari] Failed to proxy RSC request:', error)
            if (!res.headersSent) {
              res.statusCode = 500
              res.end('Internal Server Error')
            }

            return
          }
        }

        next()
      })

      server.watcher.on('change', async (filePath) => {
        if (TSX_EXT_REGEX.test(filePath)) {
          deleteComponentType(filePath)
          removeTrackedClientComponent(filePath)
          moduleAnalysisCache.invalidate(filePath)
        }

        if (TSX_EXT_REGEX.test(filePath) && filePath.includes(srcDir)) {
          if (isServerComponent(filePath)) {
            server.ws.send({
              type: 'custom',
              event: 'rari:register-server-component',
              data: { filePath },
            })
            await handleServerComponentHMR(filePath)
          }
          else {
            setTimeout(discoverAndRegisterComponents, 1000)
          }
        }
      })

      server.middlewares.use('/api/vite/hmr-transform', async (req, res) => {
        if (req.method !== 'POST') {
          res.statusCode = 405
          res.end('Method Not Allowed')
          return
        }

        let body = ''
        req.on('data', (chunk) => {
          body += chunk.toString()
        })

        req.on('end', async () => {
          try {
            const { filePath } = JSON.parse(body)

            if (!filePath) {
              res.statusCode = 400
              res.end(JSON.stringify({ error: 'filePath is required' }))
              return
            }

            await handleServerComponentHMR(filePath)

            res.statusCode = 200
            res.setHeader('Content-Type', 'application/json')
            res.end(
              JSON.stringify({
                success: true,
                filePath,
                message: 'Component transformation completed',
              }),
            )
          }
          catch (error) {
            res.statusCode = 500
            res.setHeader('Content-Type', 'application/json')
            res.end(
              JSON.stringify({
                success: false,
                error: error instanceof Error ? error.message : String(error),
              }),
            )
          }
        })
      })

      server.httpServer?.on('close', () => {
        if (hmrCoordinator) {
          hmrCoordinator.dispose()
          hmrCoordinator = null
        }

        if (rustServerProcess) {
          rustServerProcess.kill('SIGTERM')
          rustServerProcess = null
        }

        rustServerReady = false
      })
    },

    resolveId(id, importer) {
      if (id === 'virtual:rsc-integration' || id === 'virtual:rsc-integration.ts')
        return 'virtual:rsc-integration.ts'
      if (id === 'virtual:rari-entry-client' || id === 'virtual:rari-entry-client.ts')
        return 'virtual:rari-entry-client.ts'
      if (id === 'virtual:react-flight-client' || id === 'virtual:react-flight-client.ts')
        return 'virtual:react-flight-client.ts'
      if (id === 'virtual:app-router-provider' || id === 'virtual:app-router-provider.tsx')
        return 'virtual:app-router-provider.tsx'
      if (id === 'virtual:error-boundary-wrapper' || id === 'virtual:error-boundary-wrapper.tsx')
        return 'virtual:error-boundary-wrapper.tsx'

      if ((id === 'react-server-dom-webpack/client' || id === 'react-server-dom-webpack/client.browser') && importer?.startsWith('virtual:')) {
        try {
          const packageDir = path.dirname(fileURLToPath(import.meta.resolve('react-server-dom-webpack/package.json')))
          return path.join(packageDir, 'client.browser.js')
        }
        catch {}
      }
      if (id === './LoadingErrorBoundary' || id === './LoadingErrorBoundary.tsx')
        return 'virtual:loading-error-boundary.tsx'
      if (id === 'react-server-dom-rari/server')
        return id

      if (importer && importer.startsWith('virtual:') && id.startsWith('../')) {
        const currentFileUrl = import.meta.url
        const currentFilePath = fileURLToPath(currentFileUrl)
        const currentDir = path.dirname(currentFilePath)

        let runtimeDir: string | null = null
        const possibleRuntimeDirs = [
          path.join(currentDir, 'runtime'),
          path.join(currentDir, '../runtime'),
        ]

        for (const dir of possibleRuntimeDirs) {
          if (fs.existsSync(dir)) {
            runtimeDir = dir
            break
          }
        }

        if (runtimeDir) {
          const chunkPath = path.join(runtimeDir, id)
          if (fs.existsSync(chunkPath))
            return chunkPath

          const altChunkPath = path.join(runtimeDir, '../dist', path.basename(id))
          if (fs.existsSync(altChunkPath))
            return altChunkPath
        }
        else {
          console.warn(
            `[rari] Runtime directory not found, attempting fallback resolution for virtual import.\n`
            + `  Importer: ${importer}\n`
            + `  ID: ${id}\n`
            + `  Current Dir: ${currentDir}\n`
            + `  Hint: Runtime lookup failed, trying currentDir as fallback`,
          )
        }

        const chunkPath = path.join(currentDir, id)
        if (fs.existsSync(chunkPath))
          return chunkPath

        const altChunkPath = path.join(currentDir, '../dist', path.basename(id))
        if (fs.existsSync(altChunkPath))
          return altChunkPath
      }

      if (process.env.NODE_ENV === 'production') {
        try {
          const resolvedPath = path.resolve(id)
          if (fs.existsSync(resolvedPath) && isServerComponent(resolvedPath))
            return { id, external: true }
        }
        catch (err) {
          if ((err as any)?.code !== 'ENOENT') {
            console.warn('[rari] Unexpected error resolving server component:', id, err)
          }
        }
      }

      return null
    },

    async load(id) {
      if (TSX_EXT_REGEX.test(id)) {
        const environment = (this as any).environment

        if (environment && environment.name === 'client') {
          try {
            const analysis = moduleAnalysisCache.get(id)
            if (analysis.topLevelUseServer)
              return transformClientModule(moduleAnalysisCache.getSource(id), id, analysis)
          }
          catch {
            // File doesn't exist or can't be read
          }
        }
      }

      if (id === 'virtual:rari-entry-client.ts') {
        const projectRoot = options.projectRoot || process.cwd()
        const srcDir = path.join(projectRoot, 'src')
        const scannedClientComponents = collectClientComponentPaths(
          normalizeScanDirs(srcDir, Object.values(resolvedAlias)),
          moduleAnalysisCache,
        )

        const allClientComponents = new Set([
          ...getKnownClientComponentPaths(),
          ...scannedClientComponents,
        ])

        const externalClientComponents = [
          { path: 'rari/image', exports: ['Image'] },
          { path: 'virtual:error-boundary-wrapper.tsx', exports: ['ErrorBoundaryWrapper'] },
        ]

        const clientComponentsArray = [...allClientComponents].filter((componentPath) => {
          try {
            return moduleAnalysisCache.get(componentPath).topLevelUseClient
          }
          catch {
            return false
          }
        })

        const lazyLoaderRegistry = clientComponentsArray.map((componentPath) => {
          const relativePath = path.relative(process.cwd(), componentPath).replace(BACKSLASH_REGEX, '/')
          const componentId = relativePath.replace(TSX_EXT_REGEX, '')
          const registrationPath = relativePath.startsWith('..') ? componentPath.replace(BACKSLASH_REGEX, '/') : relativePath

          let hasNamedExport = false
          let namedExportName = ''
          try {
            const analysis = moduleAnalysisCache.get(componentPath)
            const hasDefault = analysis.hasDefaultExport

            if (!hasDefault) {
              const code = moduleAnalysisCache.getSource(componentPath)
              const namedExportMatch = code.match(EXPORT_NAMED_DECLARATION_REGEX)

              if (namedExportMatch) {
                hasNamedExport = true
                namedExportName = namedExportMatch[1]
              }
            }
          }
          catch (err) {
            if ((err as any)?.code !== 'ENOENT') {
              console.warn('[rari] Unexpected error reading component for export detection:', componentPath, err)
            }
          }

          const exportName = hasNamedExport ? namedExportName : 'default'
          const displayName = hasNamedExport ? namedExportName : path.basename(componentPath, path.extname(componentPath))

          const normalizedPath = registrationPath.replace(BACKSLASH_REGEX, '/')
          const importPath = normalizedPath.startsWith('/') || WINDOWS_PATH_REGEX.test(normalizedPath)
            ? normalizedPath
            : `/${normalizedPath}`
          const importStatement = `import(${JSON.stringify(importPath)})`

          return `  "${registrationPath}": {
    id: "${componentId}",
    path: "${registrationPath}",
    exportName: "${exportName}",
    displayName: "${displayName}",
    type: "client",
    loader: () => ${importStatement},
    component: null,
    loading: false,
    registered: false
  }`
        }).join(',\n')

        const externalImports = externalClientComponents.map((ext, index) => {
          return `import * as ExternalModule${index} from '${ext.path}';`
        }).join('\n')

        const externalRegistrations = externalClientComponents.flatMap((ext, index) => {
          return ext.exports.map((exportName) => {
            const fullId = `${ext.path}#${exportName}`
            return `
globalThis['~clientComponents'] = globalThis['~clientComponents'] || {};
globalThis['~clientComponents']["${fullId}"] = {
  id: "${exportName}",
  path: "${ext.path}",
  type: "client",
  component: ExternalModule${index},
  registered: true
};
globalThis['~clientComponents']["${ext.path}"] = globalThis['~clientComponents']["${ext.path}"] || {};
globalThis['~clientComponents']["${ext.path}"].component = ExternalModule${index};
globalThis['~clientComponentPaths'] = globalThis['~clientComponentPaths'] || {};
globalThis['~clientComponentPaths']["${ext.path}"] = "${exportName}";`
          })
        }).join('\n')

        const registrations = `
const lazyComponentRegistry = {
${lazyLoaderRegistry}
};

for (const [path, config] of Object.entries(lazyComponentRegistry)) {
  globalThis['~clientComponents'][path] = config;
  globalThis['~clientComponents'][config.id] = config;
  const fullId = path + '#' + config.exportName;
  globalThis['~clientComponents'][fullId] = config;
  globalThis['~clientComponentPaths'][path] = config.id;
}
`

        const allImports = externalImports
        const allRegistrations = [registrations, externalRegistrations].filter(Boolean).join('\n')

        return await loadEntryClient(allImports, allRegistrations)
      }

      if (id === 'react-server-dom-rari/server')
        return await loadRscReferences()

      if (id === 'virtual:app-router-provider.tsx') {
        const runtimeFile = resolveRuntimeDistFile('AppRouterProvider.mjs')
        if (runtimeFile)
          return fs.readFileSync(runtimeFile, 'utf-8')

        throw new Error(DIST_NOT_BUILT_ERROR)
      }

      if (id === 'virtual:loading-error-boundary.tsx') {
        const runtimeFile = resolveRuntimeDistFile('LoadingErrorBoundary.mjs')
        if (runtimeFile)
          return fs.readFileSync(runtimeFile, 'utf-8')

        throw new Error(DIST_NOT_BUILT_ERROR)
      }

      if (id === 'virtual:error-boundary-wrapper.tsx') {
        const runtimeFile = resolveRuntimeDistFile('ErrorBoundaryWrapper.mjs')
        if (runtimeFile) {
          const content = fs.readFileSync(runtimeFile, 'utf-8')
          if (!content.includes('import React') && !content.includes('from "react"') && !content.includes('from \'react\'')) {
            const useClientMatch = content.match(USE_CLIENT_DIRECTIVE_LINE_REGEX)
            if (useClientMatch) {
              const directive = useClientMatch[0]
              const rest = content.slice(directive.length)
              return `
${directive}import * as React from 'react';\n${rest}`
            }

            return `
import * as React from 'react';\n${content}`
          }

          return content
        }

        throw new Error(DIST_NOT_BUILT_ERROR)
      }

      if (id === 'virtual:rsc-integration.ts') {
        const code = await loadRscClientRuntime()
        return code.replace(
          RSC_CLIENT_IMPORT_REGEX,
          (match, whitespace, quote) => `from${whitespace}${quote}virtual:react-flight-client.ts${quote}`,
        )
      }

      if (id === 'virtual:react-flight-client.ts') {
        let browserClientPath: string
        try {
          const packageDir = path.dirname(fileURLToPath(import.meta.resolve('react-server-dom-webpack/package.json')))
          browserClientPath = path.join(packageDir, 'cjs/react-server-dom-webpack-client.browser.production.js')
        }
        catch {
          const rariDir = path.dirname(fileURLToPath(import.meta.url))
          const nmDir = path.resolve(rariDir, '../../node_modules')
          browserClientPath = path.join(nmDir, 'react-server-dom-webpack/cjs/react-server-dom-webpack-client.browser.production.js')
        }

        const cjsSource = fs.readFileSync(browserClientPath, 'utf-8')

        return {
          code: `
import * as React from 'react';
import * as ReactDOM from 'react-dom';

const module = { exports: {} };
const exports = module.exports;
(function(module, exports, require) {
${cjsSource}
})(module, module.exports, function require(id) {
  if (id === 'react') return React;
  if (id === 'react-dom') return ReactDOM;
  throw new Error('Cannot require "' + id + '" from react-server-dom-webpack client bundle');
});
export const createFromFetch = module.exports.createFromFetch;
export const createFromReadableStream = module.exports.createFromReadableStream;
`,
        }
      }

      if (id.endsWith('.mjs') && fs.existsSync(id)) {
        try {
          const projectRoot = options.projectRoot || process.cwd()
          const realId = fs.realpathSync(id)
          const relativeToRoot = path.relative(projectRoot, realId)

          const isInProjectRoot = !relativeToRoot.startsWith('..') && !path.isAbsolute(relativeToRoot)
          const isInNodeModules = realId.includes(`${path.sep}node_modules${path.sep}`)

          const isInRariPackage = realId.includes(`${path.sep}packages${path.sep}rari${path.sep}`)
            || realId.includes(`${path.sep}node_modules${path.sep}rari${path.sep}`)

          if (isInProjectRoot || isInNodeModules || isInRariPackage)
            return fs.readFileSync(id, 'utf-8')

          console.warn(`[rari] Refusing to load .mjs file outside project root and node_modules: ${id}`)
          return null
        }
        catch (err) {
          console.warn(`[rari] Error validating .mjs file path: ${id}`, err)
          return null
        }
      }
    },

    async handleHotUpdate({ file, server }) {
      const isReactFile = TSX_EXT_REGEX.test(file)

      if (!isReactFile)
        return undefined

      deleteComponentType(file)
      removeTrackedClientComponent(file)
      moduleAnalysisCache.invalidate(file)

      if (file.includes('/dist/') || file.includes('\\dist\\'))
        return []

      const componentType = hmrCoordinator?.detectComponentType(file) || 'unknown'

      const isAppRouterFile = file.includes('/app/') || file.includes('\\app\\')
      const hasExtension = (fileName: string, baseName: string) =>
        fileName.endsWith(`${baseName}.tsx`) || fileName.endsWith(`${baseName}.jsx`)
        || fileName.endsWith(`${baseName}.ts`) || fileName.endsWith(`${baseName}.js`)

      const SPECIAL_ROUTE_FILE_BASES = ['page', 'layout', 'template', 'loading', 'error', 'not-found'] as const
      const isSpecialRouteFile = SPECIAL_ROUTE_FILE_BASES.some(base => hasExtension(file, base))

      if (isAppRouterFile && isSpecialRouteFile)
        return undefined

      if (componentType === 'client')
        return

      if (componentType === 'server') {
        if (hmrCoordinator)
          await hmrCoordinator.handleServerComponentUpdate(file, server)

        return []
      }

      return undefined
    },

    transformIndexHtml: {
      order: 'pre',
      handler(html) {
        const imports: string[] = []

        for (const match of html.matchAll(IMPORT_PATH_REGEX)) {
          const importPath = match[1]
          if (importPath.startsWith('/src/'))
            imports.push(importPath)
        }

        const tags = []

        tags.push({
          tag: 'script',
          attrs: {
            type: 'module',
          },
          children: 'import \'virtual:rari-entry-client\';',
          injectTo: 'head-prepend' as const,
        })

        if (imports.length > 0) {
          tags.push(...imports.map(importPath => ({
            tag: 'script',
            attrs: {
              type: 'module',
              src: importPath,
            },
            injectTo: 'head-prepend' as const,
          })))
        }

        let modifiedHtml = html

        modifiedHtml = modifiedHtml.replace(
          /^\s*import\s+["']\/src\/[^"']+["'];?\s*$/gm,
          '',
        )

        modifiedHtml = modifiedHtml.replace(
          /^\s*import\s+["']virtual:rari-entry-client["'];?\s*$/gm,
          '',
        )

        let previousHtml: string
        do {
          previousHtml = modifiedHtml
          modifiedHtml = modifiedHtml.replace(
            /<script\s+type=["']module["'][^>]*>\s*<\/script>/gi,
            '',
          )
        } while (modifiedHtml !== previousHtml)

        return { html: modifiedHtml, tags }
      },
    },

    async writeBundle() {
      const projectRoot = options.projectRoot || process.cwd()
      await writeImageConfig(projectRoot, options)
    },
  }

  const serverBuildPlugin = createServerBuildPlugin({
    ...options.serverBuild,
    csp: options.csp,
    cacheControl: options.cacheControl,
    cache: options.cache,
    experimental: options.experimental,
    moduleAnalysisCache,
  })

  const webpackRequirePatchPlugin: Plugin = {
    name: 'rari:patch-react-server-dom-webpack',
    transform(code) {
      if (!code.includes('__webpack_require__') && !code.includes('__webpack_chunk_load__'))
        return null

      let modifiedCode = code

      if (modifiedCode.includes('__webpack_chunk_load__'))
        modifiedCode = modifiedCode.replaceAll('__webpack_chunk_load__', '__rari_chunk_load__')

      if (modifiedCode.includes('__webpack_require__.u'))
        modifiedCode = modifiedCode.replaceAll('__webpack_require__.u', '({}).u')

      if (modifiedCode.includes('__webpack_require__'))
        modifiedCode = modifiedCode.replaceAll('__webpack_require__', '__rari_rsc_require__')

      if (modifiedCode !== code) {
        return {
          code: modifiedCode,
          map: null,
        }
      }

      return null
    },
  }

  const plugins: Plugin[] = [mainPlugin, webpackRequirePatchPlugin, serverBuildPlugin]

  if (options.proxy !== false)
    plugins.push(rariProxy(options.proxy || {}))

  if (options.router !== false)
    plugins.push(rariRouter(options.router || {}))

  return plugins
}

export function defineRariConfig(
  config: UserConfig & { plugins?: Plugin[] },
): UserConfig {
  return {
    plugins: [rari(), ...(config.plugins || [])],
    ...config,
  }
}

export { rariProxy } from '../proxy/vite-plugin'
