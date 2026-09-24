import type { ChildProcess } from 'node:child_process'
import type { IncomingMessage } from 'node:http'
import type { CSSModulesOptions, Plugin, UserConfig } from 'vite-plus'
import type { ModuleAnalysis } from './analysis/directives'
import type { MdxPluginOptions } from './mdx/registry'
import type { RariPlugin } from './plugin/types'
import type { ServerBuildOptions } from './server/build'
import type { ServerCacheConfig, ServerCacheLayerConfig } from './server/config'
import type { ReactCompilerOptions } from './transform/react-compiler'
import type { ProxyPluginOptions } from '@/proxy/build/vite-plugin'
import { Buffer } from 'node:buffer'
import { spawn, spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { text as readRequestText } from 'node:stream/consumers'
import { fileURLToPath } from 'node:url'
import {
  DEFAULT_DEVICE_SIZES,
  DEFAULT_FORMATS,
  DEFAULT_IMAGE_SIZES,
  DEFAULT_MINIMUM_CACHE_TTL,
  DEFAULT_QUALITY_LEVELS,
} from '@/image/constants'
import { rariProxy } from '@/proxy/build/vite-plugin'
import { rariRouter } from '@/router/build/vite-plugin'
import { patchBrowserClientForFormActions } from '@/shared/patch-flight-browser-client'
import {
  EXPORT_NAMED_DECLARATION_REGEX,
  EXTENSION_REGEX,
  HTTP_PROTOCOL_REGEX,
  TSX_EXT_REGEX,
  WINDOWS_PATH_REGEX,
} from '@/shared/regex-constants'
import { clearFileResolverCache, resolveImportToFilePath } from '@/shared/utils/file-resolver'
import { isPathInside, normalizeAssetsDir, pathnameFromUrl, toPosixPath } from '@/shared/utils/path'
import { getRariServerPort } from '@/shared/utils/server-port'
import {
  aliasEntriesFromRecord,
  errorMessage,
  getErrnoCode,
  isAliasArray,
  isRecord,
  parseJsonArrayRecord,
  parseJsonRecord,
} from '@/shared/utils/type-guards'
import { readViteAliases, resolvePluginPaths } from '@/shared/utils/vite-aliases'
import { getComponentId } from './analysis/component-ids'
import {
  analyzeModuleSource,
  collectExportNames,
  hasDefaultExport,
  rewriteExportDefaultAsBinding,
  scanImportStatements,
} from './analysis/directives'
import {
  collectClientComponentPaths,
  invalidateModuleCachePath,
  ModuleAnalysisCache,
  resolveModuleCachePath,
} from './analysis/module-cache'
import { normalizeScanDirs } from './analysis/source-walker'
import { createSilenceReactDirectiveLogsPlugin } from './build/silence-directive-logs'
import {
  buildClientHeadFromBundle,
  buildLayoutCssImportStatements,
  CLIENT_HEAD_FILE,
  collectLayoutCssDevHrefs,
  resetClientHeadExtras,
  VIRTUAL_CLIENT_ENTRY,
} from './client-head'
import { createEnvTypesPlugin } from './env-types'
import { createFindSourceMapURLPlugin } from './find-source-map-url'
import { createFontPlugin } from './font/plugin'
import { HMRCoordinator } from './hmr/coordinator'
import { walkImporters } from './hmr/import-graph'
import { createStaticImagePlugin } from './image/static-import'
import {
  generateMdxRegistryModule,
  isMdxRegistryModuleId,
  resolveMdxRegistryEntries,
} from './mdx/registry'
import { toRariPlugins } from './plugin/types'
import {
  createServerBuildPlugin,
  isServerComponentFromAnalysis,
  RARI_CSS_MODULES_PATTERN,
  scanDirectory,
  ServerComponentBuilder,
} from './server/build'
import { clearViteEmitBuilder, getOrCreateViteEmitBuilder } from './server/rsc-vite-build'
import { emitTransformed } from './sourcemap'
import {
  buildClientReferenceReplacementFromImport,
  ensureNamedImportFromModule,
} from './transform/client-import'
import {
  hasRegisterServerReferenceImport,
  transformInlineServerActions,
} from './transform/inline-server-action'
import { transformDefineMdxComponents } from './transform/mdx-components'
import { createReactCompilerPlugin } from './transform/react-compiler'
import { createReactRefreshPlugins } from './transform/react-refresh'
import { getUseCacheTransform } from './transform/use-cache'

const DIST_NOT_BUILT_ERROR =
  '[rari] Runtime dist not built. Run `pnpm build` in the rari package first.'

const PROXY_BODY_MAX_BYTES = 10 * 1024 * 1024
const DOCUMENT_ASSET_EXT_RE =
  /\.(?:js|mjs|cjs|ts|tsx|jsx|css|map|json|svg|png|jpe?g|gif|webp|avif|ico|woff2?|ttf|eot|txt|xml|html|wasm)$/i

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
async function readRequestBodyAsBlob(req: IncomingMessage, maxBytes: number): Promise<Blob> {
  const chunks: Uint8Array[] = []
  let totalBytes = 0

  for await (const chunk of req) {
    const bytes = Uint8Array.from(Buffer.from(chunk))
    totalBytes += bytes.byteLength
    if (totalBytes > maxBytes) {
      req.destroy()
      throw Object.assign(new Error('Request body too large'), { statusCode: 413 })
    }
    chunks.push(bytes)
  }

  const body = new Uint8Array(totalBytes)
  let offset = 0
  for (const chunk of chunks) {
    body.set(chunk, offset)
    offset += chunk.byteLength
  }
  return new Blob([body])
}

function isLikelyStaticAssetPath(pathname: string): boolean {
  const basename = pathname.slice(pathname.lastIndexOf('/') + 1)
  return DOCUMENT_ASSET_EXT_RE.test(basename)
}

function isReservedRoutePrefix(pathname: string, segment: string): boolean {
  return pathname === segment || pathname.startsWith(`${segment}/`)
}

const IMPORT_TYPE_SPECIFIER_REGEX =
  /import\s+type\s+(\{[^}]+\})\s+from\s+["']\.\.?\/([^"']+)["'];?/g
const IMPORT_TYPE_NAMESPACE_REGEX =
  /import\s+type\s+(\*\s+as\s+\w+)\s+from\s+["']\.\.?\/([^"']+)["'];?/g
const IMPORT_TYPE_DEFAULT_REGEX = /import\s+type\s+(\w+)\s+from\s+["']\.\.?\/([^"']+)["'];?/g
const IMPORT_SPECIFIER_REGEX = /import\s+(\{[^}]+\})\s+from\s+["']\.\.?\/([^"']+)["'];?/g
const IMPORT_NAMESPACE_REGEX = /import\s+(\*\s+as\s+\w+)\s+from\s+["']\.\.?\/([^"']+)["'];?/g
const IMPORT_DEFAULT_REGEX = /import\s+(\w+)\s+from\s+["']\.\.?\/([^"']+)["'];?/g
const IMPORT_SIDE_EFFECT_REGEX = /import\s+["']\.\.?\/([^"']+)["'];?/g
const EXPORT_DEFAULT_FUNCTION_DECL_REGEX = /export\s+default\s+(?:async\s+)?function\s+(\w+)/
const USE_CLIENT_DIRECTIVE_REGEX = /^['"]use client['"];?\s*$/gm
const LOCAL_IMPORT_SOURCE_REGEX = /^[./@~#]/

function matchesAliasImport(source: string, aliases: Readonly<Record<string, string>>): boolean {
  for (const alias of Object.keys(aliases)) {
    if (source === alias || source.startsWith(`${alias}/`)) return true
  }
  return false
}

function isMissingPackageExportError(error: unknown): boolean {
  const code = getErrnoCode(error)
  return (
    code === 'ENOENT' || code === 'ERR_PACKAGE_PATH_NOT_EXPORTED' || code === 'ERR_MODULE_NOT_FOUND'
  )
}

function isExactReactAliasFind(find: string | RegExp): boolean {
  return (
    find === 'react' || (find instanceof RegExp && find.source === '^react$' && find.flags === '')
  )
}

const REACT_IMPORT_REGEX = /import\s+\{[^}]*\}\s+from\s+['"]react['"]/
const REACT_IMPORT_WITH_DEFAULT_REGEX = /import\s+[^,\s]+\s*,\s*\{[^}]*\}\s+from\s+['"]react['"]/
const REACT_IMPORT_MATCH_REGEX = /import React(,\s*\{([^}]*)\})?\s+from\s+['"]react['"];?/
const RSC_CLIENT_IMPORT_REGEX =
  /from(\s*)(['"])(?:\.\/vendor\/react-flight-client\/index|rari\/runtime\/vendor\/react-flight-client\/index)\.mjs\2/g
const JSX_TEST_REGEX = /\bJSX\b/
const IMPORT_SPECIFIERS_REGEX = /\{([^}]*)\}/
const USE_CLIENT_DIRECTIVE_LINE_REGEX = /^['"]use client['"];?\s*\n/

export interface RouterPluginOptions {
  readonly appDir?: string
  readonly extensions?: readonly string[]
}

export interface RariOptions {
  readonly projectRoot?: string
  readonly serverBuild?: ServerBuildOptions
  readonly serverHandler?: boolean
  readonly proxy?: ProxyPluginOptions | false
  readonly router?: RouterPluginOptions | false
  readonly images?: {
    readonly remotePatterns?: ReadonlyArray<{
      readonly protocol?: 'http' | 'https'
      readonly hostname: string
      readonly port?: string
      readonly pathname?: string
      readonly search?: string
    }>
    readonly localPatterns?: ReadonlyArray<{
      readonly pathname: string
      readonly search?: string
    }>
    readonly includeBundledAssets?: boolean
    readonly deviceSizes?: readonly number[]
    readonly imageSizes?: readonly number[]
    readonly formats?: readonly ('avif' | 'webp')[]
    readonly qualityAllowlist?: readonly number[]
    readonly minimumCacheTTL?: number
  }
  readonly csp?: {
    readonly scriptSrc?: readonly string[]
    readonly styleSrc?: readonly string[]
    readonly imgSrc?: readonly string[]
    readonly fontSrc?: readonly string[]
    readonly connectSrc?: readonly string[]
    readonly defaultSrc?: readonly string[]
    readonly workerSrc?: readonly string[]
    readonly frameAncestors?: readonly string[]
    readonly frameSrc?: readonly string[]
    readonly baseUri?: readonly string[]
    readonly formAction?: readonly string[]
    readonly useNonces?: boolean
    readonly embedderPolicy?: 'credentialless' | 'unsafe-none'
  }
  readonly cacheControl?: {
    readonly routes: Readonly<Record<string, string>>
  }
  readonly action?: {
    readonly allowedOrigins?: readonly string[]
  }
  readonly jsPoolSize?: number
  readonly origin?: string
  readonly htmlLimitedBots?: string
  readonly cache?: ServerCacheConfig
  readonly experimental?: {
    readonly useCache?: boolean
    readonly useCacheRemote?: ServerCacheLayerConfig
  }
  readonly mdx?: MdxPluginOptions
  readonly compiler?: boolean | ReactCompilerOptions
}

const DEFAULT_IMAGE_CONFIG = {
  remotePatterns: [],
  localPatterns: [],
  deviceSizes: DEFAULT_DEVICE_SIZES,
  imageSizes: DEFAULT_IMAGE_SIZES,
  formats: DEFAULT_FORMATS,
  qualityAllowlist: DEFAULT_QUALITY_LEVELS,
  minimumCacheTTL: DEFAULT_MINIMUM_CACHE_TTL,
}

function staticImageLocalPatterns(assetsDir: string): ReadonlyArray<{ readonly pathname: string }> {
  return [{ pathname: `/${normalizeAssetsDir(assetsDir)}/**` }]
}

function mergeLocalPatterns(
  patterns:
    | ReadonlyArray<{
        readonly pathname: string
        readonly search?: string
      }>
    | undefined,
  assetsDir: string,
  includeBundledAssets = true,
): Array<{ pathname: string; search?: string }> {
  const merged: Array<{ pathname: string; search?: string }> = [...(patterns ?? [])]
  if (!includeBundledAssets) return merged
  for (const pattern of staticImageLocalPatterns(assetsDir)) {
    if (!merged.some(entry => entry.pathname === pattern.pathname)) merged.push({ ...pattern })
  }
  return merged
}

const runtimeFileCache = new Map<string, string>()

async function loadRuntimeFile(filename: string): Promise<string> {
  const cached = runtimeFileCache.get(filename)
  if (cached != null && cached !== '') return cached

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
    } catch (err) {
      const code = getErrnoCode(err)
      if (code !== 'ENOENT' && code !== 'EISDIR') {
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
    if (fs.existsSync(filePath)) return filePath
  }

  return null
}

function isRariInternalFile(filePath: string): boolean {
  return filePath.startsWith(RARI_PACKAGE_ROOT)
}

async function loadRscClientRuntime(): Promise<string> {
  return loadRuntimeFile('rsc-client-runtime.mjs')
}

async function loadEntryClient(
  imports: string,
  registrations: string,
  layoutCssImports = '',
): Promise<string> {
  const template = await loadRuntimeFile('entry-client.mjs')
  const body = template
    .replace('/*! @preserve CLIENT_COMPONENT_IMPORTS_PLACEHOLDER */', imports)
    .replace('/*! @preserve CLIENT_COMPONENT_REGISTRATIONS_PLACEHOLDER */', registrations)
  return layoutCssImports !== '' ? `${layoutCssImports}\n${body}` : body
}

async function loadRscReferences(): Promise<string> {
  return loadRuntimeFile('rsc-references.mjs')
}

async function writeImageConfig(
  projectRoot: string,
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  options: RariOptions,
  assetsDir = 'assets',
  outDir?: string,
): Promise<void> {
  const srcDir = path.join(projectRoot, 'src')
  const { getBinaryPath } = await import('@/cli/platform')
  const binaryPath = getBinaryPath()

  const scanImagesTimeoutMs = 60_000
  const result = spawnSync(binaryPath, ['scan-images', '--src', srcDir], {
    encoding: 'utf8',
    cwd: projectRoot,
    shell: false,
    timeout: scanImagesTimeoutMs,
  })

  if (result.error) {
    throw new Error(`Failed to scan for image usage: ${result.error.message}`)
  }

  if (result.signal) {
    throw new Error(`Failed to scan for image usage: process terminated by signal ${result.signal}`)
  }

  if (result.status !== 0) {
    throw new Error(
      `Failed to scan for image usage: ${result.stderr || result.stdout || `exit code ${result.status}`}`,
    )
  }

  let imageManifest: { images: Array<Record<string, unknown>> }
  try {
    imageManifest = { images: parseJsonArrayRecord(result.stdout, 'images') ?? [] }
  } catch (error) {
    const parseMessage =
      error instanceof SyntaxError ? error.message : errorMessage(error, String(error))
    const outputPreview = (result.stdout || result.stderr || '(empty)').slice(0, 500)
    throw new Error(
      `Failed to parse image scanner output as JSON: ${parseMessage}. Scanner output: ${outputPreview}`,
      { cause: error },
    )
  }

  const normalizedAssetsDir = normalizeAssetsDir(assetsDir)
  const resolvedOutDir = (() => {
    const candidate = outDir ?? options.serverBuild?.outDir ?? path.join(projectRoot, 'dist')
    return path.isAbsolute(candidate) ? candidate : path.resolve(projectRoot, candidate)
  })()
  const relativeOutDir = toPosixPath(path.relative(projectRoot, resolvedOutDir)) || 'dist'
  const imageConfig = {
    ...DEFAULT_IMAGE_CONFIG,
    ...options.images,
    assetsDir: normalizedAssetsDir,
    outDir: relativeOutDir,
    localPatterns: mergeLocalPatterns(
      options.images?.localPatterns,
      normalizedAssetsDir,
      options.images?.includeBundledAssets !== false,
    ),
    preoptimizeManifest: imageManifest.images,
  }

  const serverDir = path.join(resolvedOutDir, 'server')
  if (!fs.existsSync(serverDir)) fs.mkdirSync(serverDir, { recursive: true })

  const configPath = path.join(serverDir, 'image.json')
  fs.writeFileSync(configPath, JSON.stringify(imageConfig))
}

function resolveConfiguredRariServerUrl(): string | null {
  const rariServerPort = getRariServerPort()
  if (process.env.RARI_SERVER_URL != null && process.env.RARI_SERVER_URL !== '') {
    return process.env.RARI_SERVER_URL
  }
  if (process.env.RARI_HOST != null && process.env.RARI_HOST !== '') {
    const host = process.env.RARI_HOST.startsWith('http')
      ? process.env.RARI_HOST
      : `http://${process.env.RARI_HOST}`
    const hostnamePart = host.replace(HTTP_PROTOCOL_REGEX, '')
    return hostnamePart.includes(':') ? host : `${host}:${rariServerPort}`
  }
  return `http://localhost:${rariServerPort}`
}

function shouldDefineRariServerUrl(command: string): boolean {
  return (
    command === 'serve' ||
    (process.env.RARI_SERVER_URL != null && process.env.RARI_SERVER_URL !== '') ||
    (process.env.RARI_HOST != null && process.env.RARI_HOST !== '')
  )
}

function tryAppendOptionalReactAlias(
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  aliasesToAppend: Array<{ find: string | RegExp; replacement: string }>,
  aliasFinds: ReadonlySet<string>,
  exportPath: string,
): void {
  try {
    const resolved = fileURLToPath(import.meta.resolve(exportPath))
    if (!aliasFinds.has(exportPath)) {
      aliasesToAppend.push({ find: exportPath, replacement: resolved })
    }
  } catch (err) {
    if (!isMissingPackageExportError(err)) {
      console.warn(`[rari] Unexpected error resolving ${exportPath}:`, err)
    }
  }
}

function collectReactAliasAppendages(
  aliasFinds: ReadonlySet<string>,
  hasExactReactAlias: boolean,
): Array<{ find: string | RegExp; replacement: string }> {
  const reactPath = fileURLToPath(import.meta.resolve('react'))
  const reactDomClientPath = fileURLToPath(import.meta.resolve('react-dom/client'))
  const reactJsxRuntimePath = fileURLToPath(import.meta.resolve('react/jsx-runtime'))
  const aliasesToAppend: Array<{ find: string | RegExp; replacement: string }> = []

  if (!aliasFinds.has('react/jsx-runtime')) {
    aliasesToAppend.push({ find: 'react/jsx-runtime', replacement: reactJsxRuntimePath })
  }

  tryAppendOptionalReactAlias(aliasesToAppend, aliasFinds, 'react/jsx-dev-runtime')
  tryAppendOptionalReactAlias(aliasesToAppend, aliasFinds, 'react/compiler-runtime')

  if (!hasExactReactAlias) aliasesToAppend.push({ find: /^react$/, replacement: reactPath })
  if (!aliasFinds.has('react-dom/client')) {
    aliasesToAppend.push({ find: 'react-dom/client', replacement: reactDomClientPath })
  }
  return aliasesToAppend
}

function loadRuntimeDistVirtual(filename: string): string {
  const runtimeFile = resolveRuntimeDistFile(filename)
  if (runtimeFile != null && runtimeFile !== '') return fs.readFileSync(runtimeFile, 'utf-8')
  throw new Error(DIST_NOT_BUILT_ERROR)
}

function ensureReactImportInErrorBoundary(content: string): string {
  if (
    content.includes('import React') ||
    content.includes('from "react"') ||
    content.includes("from 'react'")
  ) {
    return content
  }
  const useClientMatch = USE_CLIENT_DIRECTIVE_LINE_REGEX.exec(content)
  if (useClientMatch) {
    const directive = useClientMatch[0]
    const rest = content.slice(directive.length)
    return `\n${directive}import * as React from 'react';\n${rest}`
  }
  return `\nimport * as React from 'react';\n${content}`
}

function resolveFlightClientPaths(flightBuild: string): {
  browserClientPath: string
  edgeClientPath: string
} {
  const browserFile = `react-server-dom-webpack-client.browser.${flightBuild}.js`
  const edgeFile = `react-server-dom-webpack-client.edge.${flightBuild}.js`
  try {
    const packageDir = path.dirname(
      fileURLToPath(import.meta.resolve('react-server-dom-webpack/package.json')),
    )
    return {
      browserClientPath: path.join(packageDir, 'cjs', browserFile),
      edgeClientPath: path.join(packageDir, 'cjs', edgeFile),
    }
  } catch {
    const rariDir = path.dirname(fileURLToPath(import.meta.url))
    const nmDir = path.resolve(rariDir, '../../node_modules')
    return {
      browserClientPath: path.join(nmDir, 'react-server-dom-webpack/cjs', browserFile),
      edgeClientPath: path.join(nmDir, 'react-server-dom-webpack/cjs', edgeFile),
    }
  }
}

function shouldSkipDevProxy(
  isRscRequest: boolean,
  isDocumentRequest: boolean,
  url: string,
  pathname: string,
): boolean {
  return (
    (!isRscRequest && !isDocumentRequest) ||
    url === '' ||
    isReservedRoutePrefix(pathname, '/api') ||
    isReservedRoutePrefix(pathname, '/rsc')
  )
}

function buildFlightClientVirtualModule(flightBuild: string): { code: string } {
  const { browserClientPath, edgeClientPath } = resolveFlightClientPaths(flightBuild)
  const browserSource = fs.readFileSync(browserClientPath, 'utf-8')
  const edgeSource = fs.readFileSync(edgeClientPath, 'utf-8')
  const cjsSource = patchBrowserClientForFormActions(browserSource, edgeSource)
  const embeddedSource =
    flightBuild === 'development'
      ? cjsSource.replace('"production" !== process.env.NODE_ENV &&', 'true &&')
      : cjsSource

  return {
    code: `
import * as React from 'react';
import * as ReactDOM from 'react-dom';
import { callServer as rariCallServer } from 'rari/runtime/call-server';

const module = { exports: {} };
const exports = module.exports;
(function(module, exports, require) {
${embeddedSource}
})(module, module.exports, function require(id) {
  if (id === 'react') return React;
  if (id === 'react-dom') return ReactDOM;
  throw new Error('Cannot require "' + id + '" from react-server-dom-webpack client bundle');
});

const rariFindSourceMapURL = import.meta.env.DEV
  ? function rariFindSourceMapURL(filename, environmentName) {
      const url = new URL('/__rari_findSourceMapURL', window.location.origin);
      url.searchParams.set('filename', filename);
      url.searchParams.set('environmentName', environmentName ?? 'Server');
      return url.href;
    }
  : undefined;

export function createFromFetch(promise, options) {
  return module.exports.createFromFetch(promise, {
    ...options,
    callServer: options?.callServer ?? rariCallServer,
    findSourceMapURL: options?.findSourceMapURL ?? rariFindSourceMapURL,
  });
}

export function createFromReadableStream(stream, options) {
  return module.exports.createFromReadableStream(stream, {
    ...options,
    callServer: options?.callServer ?? rariCallServer,
    findSourceMapURL: options?.findSourceMapURL ?? rariFindSourceMapURL,
  });
}

export function createServerReference(id, callServer, encodeFormAction, findSourceMapURL, functionName) {
  return module.exports.createServerReference(
    id,
    callServer ?? rariCallServer,
    encodeFormAction,
    findSourceMapURL ?? rariFindSourceMapURL,
    functionName,
  );
}

export const encodeReply = module.exports.encodeReply;
export const createTemporaryReferenceSet = module.exports.createTemporaryReferenceSet;
`,
  }
}

function isAllowedMjsLoadPath(realId: string, projectRoot: string): boolean {
  const relativeToRoot = path.relative(projectRoot, realId)
  const isInProjectRoot = !relativeToRoot.startsWith('..') && !path.isAbsolute(relativeToRoot)
  const isInNodeModules = realId.includes(`${path.sep}node_modules${path.sep}`)
  const isInAllowedWorkspacePackage =
    realId.includes(`${path.sep}packages${path.sep}rari${path.sep}`) ||
    realId.includes(`${path.sep}packages${path.sep}use-cache${path.sep}`) ||
    realId.includes(`${path.sep}node_modules${path.sep}rari${path.sep}`) ||
    realId.includes(`${path.sep}node_modules${path.sep}@rari${path.sep}use-cache${path.sep}`)
  return isInProjectRoot || isInNodeModules || isInAllowedWorkspacePackage
}

function tryLoadSafeMjsFile(id: string, projectRoot: string): string | null {
  if (!id.endsWith('.mjs') || !fs.existsSync(id)) return null
  try {
    const realId = fs.realpathSync(id)
    if (isAllowedMjsLoadPath(realId, projectRoot)) return fs.readFileSync(id, 'utf-8')
    console.warn(`[rari] Refusing to load .mjs file outside project root and node_modules: ${id}`)
    return null
  } catch (err) {
    console.warn(`[rari] Error validating .mjs file path: ${id}`, err)
    return null
  }
}

const RUNTIME_DIST_VIRTUALS: ReadonlyMap<string, string> = new Map([
  ['virtual:app-router-provider.tsx', 'AppRouterProvider.mjs'],
  ['virtual:client-router.tsx', 'ClientRouter.mjs'],
])

async function loadNamedVirtualModule(
  id: string,
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types -- ctx holds mutable analysis cache
  ctx: {
    buildMdxRegistryModule: () => string
    resolveProjectRoot: () => string
    resolvedAlias: Readonly<Record<string, string>>
    moduleAnalysisCache: ModuleAnalysisCache
    getKnownClientComponentPaths: () => ReadonlySet<string>
    environmentMode: string
  },
): Promise<string | { code: string } | undefined> {
  if (id === 'virtual:rari-mdx-components.ts') return ctx.buildMdxRegistryModule()

  if (id === 'virtual:rari-entry-client.ts') {
    return buildVirtualEntryClientModule(
      ctx.resolveProjectRoot(),
      ctx.resolvedAlias,
      ctx.moduleAnalysisCache,
      ctx.getKnownClientComponentPaths,
    )
  }

  if (id === 'react-server-dom-rari/server') return loadRscReferences()

  const runtimeDist = RUNTIME_DIST_VIRTUALS.get(id)
  if (runtimeDist != null) return loadRuntimeDistVirtual(runtimeDist)

  if (id === 'virtual:error-boundary-wrapper.tsx') {
    return ensureReactImportInErrorBoundary(loadRuntimeDistVirtual('ErrorBoundaryWrapper.mjs'))
  }

  if (id === 'virtual:rsc-integration.ts') {
    const code = await loadRscClientRuntime()
    return code.replace(
      RSC_CLIENT_IMPORT_REGEX,
      (_match, whitespace, quote) =>
        `from${whitespace}${quote}virtual:react-flight-client.ts${quote}`,
    )
  }

  if (id === 'virtual:react-flight-client.ts') {
    const flightBuild = ctx.environmentMode === 'dev' ? 'development' : 'production'
    return buildFlightClientVirtualModule(flightBuild)
  }

  return undefined
}

function tryResolveProductionServerComponent(
  id: string,
  isServerComponent: (filePath: string) => boolean,
): { id: string; external: true } | null {
  if (process.env.NODE_ENV !== 'production') return null
  try {
    const resolvedPath = path.resolve(id)
    if (fs.existsSync(resolvedPath) && isServerComponent(resolvedPath)) {
      return { id, external: true }
    }
  } catch (err) {
    if (getErrnoCode(err) !== 'ENOENT') {
      console.warn('[rari] Unexpected error resolving server component:', id, err)
    }
  }
  return null
}

function vendorChunkNameForModule(
  moduleId: string,
  userGroups: ReadonlyArray<{ readonly test?: unknown }>,
): string | null {
  if (!moduleId.includes('node_modules')) return null

  for (const group of userGroups) {
    if (group.test == null) continue
    let testResult = false
    if (typeof group.test === 'function') {
      // oxlint-disable-next-line typescript/no-unsafe-type-assertion
      const testFn = group.test as (id: string) => unknown
      testResult = Boolean(testFn(moduleId))
    } else if (group.test instanceof RegExp) {
      testResult = group.test.test(moduleId)
    } else if (typeof group.test === 'string') {
      testResult = moduleId.includes(group.test)
    }
    if (testResult) return null
  }

  if (moduleId.includes('node_modules/react-dom')) return 'react-dom'
  if (moduleId.includes('node_modules/react')) return 'react'
  return 'vendor'
}

function isDocumentHtmlRequest(
  method: string,
  acceptHeader: string | undefined,
  pathname: string,
): boolean {
  return (
    (method === 'GET' || method === 'HEAD') &&
    (acceptHeader?.includes('text/html') ?? false) &&
    !pathname.startsWith('/@') &&
    !isReservedRoutePrefix(pathname, '/node_modules') &&
    !isReservedRoutePrefix(pathname, '/api') &&
    !isReservedRoutePrefix(pathname, '/_rari') &&
    !isReservedRoutePrefix(pathname, '/assets') &&
    !isReservedRoutePrefix(pathname, '/vite-server') &&
    !isLikelyStaticAssetPath(pathname)
  )
}

function copyProxyRequestHeaders(
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  reqHeaders: Readonly<Record<string, string | string[] | undefined>>,
  serverPort: number,
): Record<string, string> {
  const headers: Record<string, string> = {}
  for (const [key, value] of Object.entries(reqHeaders)) {
    if (typeof value === 'string') headers[key] = value
    else if (Array.isArray(value)) headers[key] = value.join(',')
  }
  headers.host = `localhost:${serverPort}`
  headers['accept-encoding'] = 'identity'
  return headers
}

async function streamProxyResponseBody(
  response: Response,
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  res: {
    write: (chunk: Buffer) => void
    end: () => void
    headersSent: boolean
    statusCode: number
  },
  method: string,
): Promise<void> {
  if (method === 'HEAD' || !response.body) {
    res.end()
    return
  }

  const reader = response.body.getReader()
  try {
    let streamDone = false
    while (!streamDone) {
      const { done, value } = await reader.read()
      streamDone = done
      if (!streamDone && value != null) res.write(Buffer.from(value))
    }
    res.end()
  } catch (streamError) {
    console.error('[rari] Stream error:', streamError)
    if (!res.headersSent) res.statusCode = 500
    res.end()
  }
}

const VIRTUAL_MODULE_RESOLUTIONS: ReadonlyMap<string, string> = new Map([
  ['virtual:rsc-integration', 'virtual:rsc-integration.ts'],
  ['virtual:rsc-integration.ts', 'virtual:rsc-integration.ts'],
  ['virtual:rari-entry-client', 'virtual:rari-entry-client.ts'],
  ['virtual:rari-entry-client.ts', 'virtual:rari-entry-client.ts'],
  ['virtual:react-flight-client', 'virtual:react-flight-client.ts'],
  ['virtual:react-flight-client.ts', 'virtual:react-flight-client.ts'],
  ['virtual:app-router-provider', 'virtual:app-router-provider.tsx'],
  ['virtual:app-router-provider.tsx', 'virtual:app-router-provider.tsx'],
  ['virtual:client-router', 'virtual:client-router.tsx'],
  ['virtual:client-router.tsx', 'virtual:client-router.tsx'],
  ['virtual:error-boundary-wrapper', 'virtual:error-boundary-wrapper.tsx'],
  ['virtual:error-boundary-wrapper.tsx', 'virtual:error-boundary-wrapper.tsx'],
  ['virtual:rari-mdx-components', 'virtual:rari-mdx-components.ts'],
  ['virtual:rari-mdx-components.ts', 'virtual:rari-mdx-components.ts'],
])

function resolveVirtualModuleId(id: string): string | null {
  if (isMdxRegistryModuleId(id)) return 'virtual:rari-mdx-components.ts'
  return VIRTUAL_MODULE_RESOLUTIONS.get(id) ?? null
}

function findExistingFile(candidates: readonly string[]): string | null {
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) return candidate
  }
  return null
}

function resolveVirtualRelativeImport(id: string, importer: string): string | null {
  if (id.startsWith('./')) {
    const runtimeFile = resolveRuntimeDistFile(path.basename(id))
    return runtimeFile != null && runtimeFile !== '' ? runtimeFile : null
  }

  if (!id.startsWith('../')) return null

  const currentDir = path.dirname(fileURLToPath(import.meta.url))
  const runtimeDir = findExistingFile([
    path.join(currentDir, 'runtime'),
    path.join(currentDir, '../runtime'),
  ])

  if (runtimeDir != null) {
    const found = findExistingFile([
      path.join(runtimeDir, id),
      path.join(runtimeDir, '../dist', path.basename(id)),
    ])
    if (found != null) return found
  } else {
    console.warn(
      `[rari] Runtime directory not found, attempting fallback resolution for virtual import.\n` +
        `  Importer: ${importer}\n` +
        `  ID: ${id}\n` +
        `  Current Dir: ${currentDir}\n` +
        `  Hint: Runtime lookup failed, trying currentDir as fallback`,
    )
  }

  return findExistingFile([
    path.join(currentDir, id),
    path.join(currentDir, '../dist', path.basename(id)),
  ])
}

function detectClientComponentExportMeta(
  componentPath: string,
  moduleAnalysisCache: ModuleAnalysisCache,
): { exportName: string; displayName: string } {
  let hasNamedExport = false
  let namedExportName = ''
  try {
    const analysis = moduleAnalysisCache.get(componentPath)
    if (!analysis.hasDefaultExport) {
      const code = moduleAnalysisCache.getSource(componentPath)
      const namedExportMatch = code.match(EXPORT_NAMED_DECLARATION_REGEX)
      if (namedExportMatch) {
        hasNamedExport = true
        namedExportName = namedExportMatch[1]
      }
    }
  } catch (err) {
    if (getErrnoCode(err) !== 'ENOENT') {
      console.warn(
        '[rari] Unexpected error reading component for export detection:',
        componentPath,
        err,
      )
    }
  }

  return {
    exportName: hasNamedExport ? namedExportName : 'default',
    displayName: hasNamedExport
      ? namedExportName
      : path.basename(componentPath, path.extname(componentPath)),
  }
}

function buildLazyLoaderEntry(
  componentPath: string,
  moduleAnalysisCache: ModuleAnalysisCache,
): string {
  const relativePath = toPosixPath(path.relative(process.cwd(), componentPath))
  const componentId = relativePath.replace(TSX_EXT_REGEX, '')
  const registrationPath = relativePath.startsWith('..') ? toPosixPath(componentPath) : relativePath
  const { exportName, displayName } = detectClientComponentExportMeta(
    componentPath,
    moduleAnalysisCache,
  )

  const normalizedPath = toPosixPath(registrationPath)
  const importPath =
    normalizedPath.startsWith('/') || WINDOWS_PATH_REGEX.test(normalizedPath)
      ? normalizedPath
      : `/${normalizedPath}`

  return `  "${registrationPath}": {
    id: "${componentId}",
    path: "${registrationPath}",
    exportName: "${exportName}",
    displayName: "${displayName}",
    type: "client",
    loader: () => import(${JSON.stringify(importPath)}),
    component: null,
    loading: false,
    registered: false
  }`
}

function buildExternalClientRegistrations(
  externalClientComponents: ReadonlyArray<{
    readonly path: string
    readonly exports: readonly string[]
  }>,
): { imports: string; registrations: string } {
  const imports = externalClientComponents
    .map((ext, index) => `import * as ExternalModule${index} from '${ext.path}';`)
    .join('\n')

  const registrations = externalClientComponents
    .flatMap((ext, index) =>
      ext.exports.map(exportName => {
        const fullId = `${ext.path}#${exportName}`
        return `
globalThis['~clientComponents'] = globalThis['~clientComponents'] || {};
{
  const entry = {
    id: "${exportName}",
    path: "${ext.path}",
    type: "client",
    component: ExternalModule${index},
    registered: true
  };
  globalThis['~clientComponents']["${fullId}"] = entry;
  globalThis['~clientComponents']["${ext.path}"] = entry;
}
globalThis['~clientComponentPaths'] = globalThis['~clientComponentPaths'] || {};
globalThis['~clientComponentPaths']["${ext.path}"] = "${exportName}";`
      }),
    )
    .join('\n')

  return { imports, registrations }
}

async function buildVirtualEntryClientModule(
  projectRoot: string,
  resolvedAlias: Readonly<Record<string, string>>,
  moduleAnalysisCache: ModuleAnalysisCache,
  getKnownClientComponentPaths: () => ReadonlySet<string>,
): Promise<string> {
  const srcDir = path.join(projectRoot, 'src')
  const scannedClientComponents = collectClientComponentPaths(
    normalizeScanDirs(srcDir, Object.values(resolvedAlias)),
    moduleAnalysisCache,
  )

  const allClientComponents = getKnownClientComponentPaths().union(new Set(scannedClientComponents))

  const externalClientComponents = [
    { path: 'rari/image', exports: ['Image'] },
    { path: 'virtual:error-boundary-wrapper.tsx', exports: ['ErrorBoundaryWrapper'] },
  ]

  const clientComponentsArray = [...allClientComponents].filter(componentPath => {
    try {
      return moduleAnalysisCache.get(componentPath).topLevelUseClient
    } catch {
      return false
    }
  })

  const lazyLoaderRegistry = clientComponentsArray
    .map(componentPath => buildLazyLoaderEntry(componentPath, moduleAnalysisCache))
    .join(',\n')

  const external = buildExternalClientRegistrations(externalClientComponents)

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

  const allRegistrations = [registrations, external.registrations].filter(Boolean).join('\n')
  const layoutCssImports = buildLayoutCssImportStatements(projectRoot, resolvedAlias)

  return loadEntryClient(external.imports, allRegistrations, layoutCssImports)
}

function applyRariEnvironments(config: UserConfig): void {
  config.environments ??= {}

  config.environments.rsc = {
    consumer: 'server',
    resolve: { conditions: ['react-server', 'node', 'import'] },
    build: {
      outDir: 'dist/server',
      write: false,
      copyPublicDir: false,
      emitAssets: true,
    },
    ...config.environments.rsc,
  }

  config.environments.ssr = {
    consumer: 'server',
    resolve: { conditions: ['node', 'import'] },
    build: {
      outDir: 'dist/ssr',
      write: false,
      copyPublicDir: false,
      emitAssets: true,
    },
    ...config.environments.ssr,
  }

  config.environments.client = {
    consumer: 'client',
    resolve: { conditions: ['browser', 'import'] },
    ...config.environments.client,
  }
}

function applyRariOptimizeDeps(config: UserConfig): void {
  config.optimizeDeps ??= {}
  config.optimizeDeps.include ??= []

  const coreOptimizeDeps = [
    'react',
    'react-dom',
    'react-dom/client',
    'react-dom/server',
    'react/jsx-runtime',
    'react/jsx-dev-runtime',
  ]

  for (const dep of coreOptimizeDeps) {
    if (!config.optimizeDeps.include.includes(dep)) config.optimizeDeps.include.push(dep)
  }

  config.optimizeDeps.exclude ??= []
  if (!config.optimizeDeps.exclude.includes('rari')) config.optimizeDeps.exclude.push('rari')
}

function applyRariServerProxy(config: UserConfig): void {
  config.server ??= {}
  config.server.proxy ??= {}
  const serverPort = getRariServerPort()
  const proxyTarget = {
    target: `http://localhost:${serverPort}`,
    changeOrigin: true,
    secure: false,
  }

  config.server.proxy['/api'] = { ...proxyTarget, ws: true }
  config.server.proxy['/_rari'] = { ...proxyTarget, ws: true }
  config.server.proxy['/assets'] = proxyTarget
}

function applyRariBuildCodeSplitting(config: UserConfig): void {
  config.build ??= {}
  config.build.rolldownOptions ??= {}
  config.build.rolldownOptions.input = { main: VIRTUAL_CLIENT_ENTRY }
  config.build.rolldownOptions.output ??= {}

  const outputs = Array.isArray(config.build.rolldownOptions.output)
    ? config.build.rolldownOptions.output
    : [config.build.rolldownOptions.output]

  for (const output of outputs) {
    if (output.codeSplitting !== false && typeof output.codeSplitting !== 'object') {
      output.codeSplitting = {}
    }
    if (typeof output.codeSplitting === 'object') {
      output.codeSplitting.groups ??= []
      const userGroups = output.codeSplitting.groups
      output.codeSplitting.groups.push({
        name(moduleId: string) {
          return vendorChunkNameForModule(moduleId, userGroups)
        },
      })
    }
  }
}

function applyRariClientBuildInput(config: UserConfig): void {
  config.environments!.client.build ??= {}
  config.environments!.client.build.rolldownOptions ??= {}
  config.environments!.client.build.rolldownOptions.input = {
    main: VIRTUAL_CLIENT_ENTRY,
  }
  config.environments!.client.build.rolldownOptions.external ??= []

  const external = config.environments!.client.build.rolldownOptions.external
  if (Array.isArray(external)) {
    const rsdwClientIndex = external.indexOf('react-server-dom-webpack/client')
    if (rsdwClientIndex !== -1) external.splice(rsdwClientIndex, 1)
  }
}

function normalizeResolveAliases(
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  resolveOptions: NonNullable<UserConfig['resolve']>,
): Array<{ find: string | RegExp; replacement: string }> {
  let existingAlias: Array<{ find: string | RegExp; replacement: string }> = []
  if (isAliasArray(resolveOptions.alias)) {
    existingAlias = resolveOptions.alias
  } else if (isRecord(resolveOptions.alias)) {
    existingAlias = aliasEntriesFromRecord(resolveOptions.alias)
  }
  return existingAlias.map(entry =>
    entry.find === 'react' ? { find: /^react$/, replacement: entry.replacement } : entry,
  )
}

function handleDevProxyError(
  error: unknown,
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  res: { headersSent: boolean; statusCode: number; end: (body?: string) => void },
  isRscRequest: boolean,
): void {
  const statusCode =
    isRecord(error) && typeof error.statusCode === 'number' ? error.statusCode : undefined
  if (statusCode === 413) {
    if (!res.headersSent) {
      res.statusCode = 413
      res.end('Request Entity Too Large')
    }
    return
  }
  console.error(`[rari] Failed to proxy ${isRscRequest ? 'RSC' : 'HTML'} request:`, error)
  if (!res.headersSent) {
    res.statusCode = 500
    res.end('Internal Server Error')
  }
}

export function defineRariOptions(
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  config: RariOptions,
): RariOptions {
  return config
}

export function rari(
  // oxlint-disable-next-line typescript/prefer-readonly-parameter-types
  options: RariOptions = {},
): RariPlugin[] {
  if (options.jsPoolSize != null) {
    const size = options.jsPoolSize
    if (!Number.isInteger(size) || size < 1 || !Number.isFinite(size)) {
      throw new Error(`jsPoolSize must be a finite positive integer; received ${String(size)}`)
    }
  }

  const componentTypeCache = new Map<string, 'client' | 'server' | 'unknown'>()
  const clientComponents = new Set<string>()
  const moduleAnalysisCache = new ModuleAnalysisCache()
  let devServerComponentBuilder: ServerComponentBuilder | null = null
  let rustServerProcess: ChildProcess | null = null

  let hmrCoordinator: HMRCoordinator | null = null
  const resolvedAlias: Record<string, string> = {}
  let resolvedAssetsDir = 'assets'
  let resolvedOutDir = path.join(process.cwd(), 'dist')
  let cachedMdxRegistryModule: string | null = null

  function invalidateMdxRegistryModuleCache(): void {
    cachedMdxRegistryModule = null
  }

  function buildMdxRegistryModule(): string {
    if (cachedMdxRegistryModule != null && cachedMdxRegistryModule !== '')
      return cachedMdxRegistryModule

    const projectRoot =
      options.projectRoot != null && options.projectRoot !== ''
        ? options.projectRoot
        : process.cwd()
    const entries = resolveMdxRegistryEntries({
      projectRoot,
      mdxOptions: options.mdx,
      alias: resolvedAlias,
      cache: moduleAnalysisCache,
    })

    cachedMdxRegistryModule = generateMdxRegistryModule(entries)
    return cachedMdxRegistryModule
  }

  function getComponentType(filePath: string): 'client' | 'server' | 'unknown' | undefined {
    return componentTypeCache.get(resolveModuleCachePath(filePath))
  }

  function clientReferenceIdForPath(absolutePath: string): string {
    const projectRoot =
      options.projectRoot != null && options.projectRoot !== ''
        ? options.projectRoot
        : process.cwd()
    const relative = toPosixPath(path.relative(projectRoot, absolutePath))
    if (relative.startsWith('..') || path.isAbsolute(relative)) return toPosixPath(absolutePath)
    return relative
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
    } catch {
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

  function isServerComponent(filePath: string): boolean {
    if (filePath.includes('node_modules') || isRariInternalFile(filePath)) return false

    const resolvedPath = resolveModuleCachePath(filePath)

    try {
      const analysis = moduleAnalysisCache.get(filePath)
      return isServerComponentFromAnalysis(resolvedPath, analysis)
    } catch {
      return false
    }
  }

  function parseExportedNames(code: string, analysis?: ModuleAnalysis): string[] {
    try {
      const exportedNames = new Set(collectExportNames(code))
      if (analysis?.hasDefaultExport ?? hasDefaultExport(code)) exportedNames.add('default')
      return [...exportedNames]
    } catch {
      return []
    }
  }

  function appendHmrAcceptStub(code: string): string {
    return `${code}

if (import.meta.hot) {
  import.meta.hot.accept(() => {
  });
}`
  }

  function registerDefaultServerExport(newCode: string, idJson: string): string {
    const functionDeclMatch = EXPORT_DEFAULT_FUNCTION_DECL_REGEX.exec(newCode)
    if (functionDeclMatch) {
      const functionName = functionDeclMatch[1]
      newCode += `\n// Register server reference for default export\n`
      newCode += `registerServerReference(${functionName}, ${idJson}, ${JSON.stringify('default')});\n`
      return newCode
    }

    const tempVarName = '__default_export__'
    const rewritten = rewriteExportDefaultAsBinding(newCode, tempVarName)
    if (rewritten == null) return newCode
    newCode = rewritten
    newCode += `\n// Register server reference for default export\n`
    newCode += `if (typeof ${tempVarName} === "function") {\n`
    newCode += `  registerServerReference(${tempVarName}, ${idJson}, ${JSON.stringify('default')});\n`
    newCode += `}\n`
    return newCode
  }

  function appendNamedServerExportRegistration(
    newCode: string,
    name: string,
    idJson: string,
  ): string {
    newCode += `\n// Register server reference for ${name}\n`
    newCode += `if (typeof ${name} === "function") {\n`
    newCode += `  registerServerReference(${name}, ${idJson}, ${JSON.stringify(name)});\n`
    newCode += `}\n`
    return newCode
  }

  function transformServerModule(code: string, id: string, analysis: ModuleAnalysis): string {
    const projectRoot =
      options.projectRoot != null && options.projectRoot !== ''
        ? options.projectRoot
        : process.cwd()
    const moduleId = getComponentId(id, projectRoot)

    const inlineTransformed = transformInlineServerActions(code, moduleId)
    let newCode = inlineTransformed?.code ?? code

    if (!analysis.topLevelUseServer) {
      if (inlineTransformed == null) return code
      return appendHmrAcceptStub(newCode)
    }

    const exportedNames = parseExportedNames(newCode, analysis)
    if (exportedNames.length === 0 && inlineTransformed == null) return code

    const idJson = JSON.stringify(moduleId)
    if (!hasRegisterServerReferenceImport(newCode)) {
      newCode += '\n\nimport {registerServerReference} from "react-server-dom-rari/server";\n'
    } else {
      newCode += '\n'
    }

    for (const name of exportedNames) {
      if (name === 'default') {
        newCode = registerDefaultServerExport(newCode, idJson)
      } else if (inlineTransformed?.rewrittenExportNames.includes(name)) {
        continue
      } else {
        newCode = appendNamedServerExportRegistration(newCode, name, idJson)
      }
    }

    return appendHmrAcceptStub(newCode)
  }

  function buildServerReferenceClientStub(
    code: string,
    id: string,
    analysis: ModuleAnalysis,
    projectRoot: string,
  ): string {
    const exportedNames = parseExportedNames(code, analysis)
    if (exportedNames.length === 0) return ''

    const moduleId = getComponentId(id, projectRoot)
    let newCode = 'import { createServerReference } from "virtual:react-flight-client";\n'
    newCode += 'import { callServer } from "rari/runtime/call-server";\n'

    for (const name of exportedNames) {
      const refId = `${moduleId}#${name}`
      const refIdJson = JSON.stringify(refId)
      if (name === 'default')
        newCode += `export default createServerReference(${refIdJson}, callServer);\n`
      else newCode += `export const ${name} = createServerReference(${refIdJson}, callServer);\n`
    }

    return newCode
  }

  function transformClientModule(code: string, id: string, analysis: ModuleAnalysis): string {
    const projectRoot =
      options.projectRoot != null && options.projectRoot !== ''
        ? options.projectRoot
        : process.cwd()
    const isServerComp = isServerComponent(id)

    if (analysis.topLevelUseServer) {
      return buildServerReferenceClientStub(code, id, analysis, projectRoot)
    }

    if (isServerComp) {
      console.warn(`[rari] Server component ${id} should not be imported in client bundle`)
      return ''
    }

    if (!analysis.topLevelUseClient) return code

    const exportedNames = parseExportedNames(code, analysis)
    if (exportedNames.length === 0) return ''

    const idJson = JSON.stringify(id)
    let newCode = 'import {registerClientReference} from "react-server-dom-rari/server";\n'

    for (const name of exportedNames) {
      if (name === 'default') {
        const errorMsg = `Attempted to call the default export of ${id} from the server but it's on the client. It's not possible to invoke a client function from the server, it can only be rendered as a Component or passed to props of a Client Component.`
        newCode += 'export default '
        newCode += 'registerClientReference(function() {'
        newCode += `throw new Error(${JSON.stringify(errorMsg)});`
      } else {
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

  function transformClientModuleForClient(
    code: string,
    _id: string,
    analysis: ModuleAnalysis,
  ): string {
    if (!analysis.topLevelUseClient) return code

    const exportedNames = parseExportedNames(code, analysis)
    if (exportedNames.length === 0) return code

    return code.replace(USE_CLIENT_DIRECTIVE_REGEX, '')
  }

  let rustServerReady = false

  async function checkRustServerHealth(): Promise<boolean> {
    const baseUrl = `http://localhost:${getRariServerPort()}`

    try {
      const healthResponse = await fetch(`${baseUrl}/_rari/health`, {
        signal: AbortSignal.timeout(1000),
      })
      const isHealthy = healthResponse.ok
      rustServerReady = isHealthy
      return isHealthy
    } catch {
      rustServerReady = false
      return false
    }
  }

  let rustServerReadyWait: Promise<boolean> | null = null

  async function waitForRustServerReady(timeoutMs: number): Promise<boolean> {
    if (await checkRustServerHealth()) return true

    rustServerReadyWait ??= (async () => {
      try {
        const deadline = Date.now() + timeoutMs
        let interval = 50

        while (Date.now() < deadline) {
          await new Promise(resolve => {
            setTimeout(resolve, Math.min(interval, deadline - Date.now()))
          })
          if (await checkRustServerHealth()) return true
          interval = Math.min(interval * 2, 500)
        }

        return false
      } finally {
        rustServerReadyWait = null
      }
    })()

    return rustServerReadyWait
  }

  function isClientBoundaryModule(filePath: string): boolean {
    if (!fs.existsSync(filePath)) return false
    try {
      return moduleAnalysisCache.get(filePath).topLevelUseClient
    } catch {
      return false
    }
  }

  function rewriteClientImportsForServerEnvironment(source: string, fileId: string): string {
    let modifiedCode = source
    const replacements: Array<{ start: number; end: number; replacement: string }> = []
    const clientRefHelpers = new Set<string>()

    for (const imp of scanImportStatements(modifiedCode)) {
      if (imp.typeOnly || imp.sideEffectOnly) continue
      if (
        !LOCAL_IMPORT_SOURCE_REGEX.test(imp.source) &&
        !matchesAliasImport(imp.source, resolvedAlias)
      ) {
        continue
      }

      const resolvedImportPath = resolveImportToFilePath(imp.source, fileId, resolvedAlias)
      if (!isClientBoundaryModule(resolvedImportPath)) continue

      setComponentType(resolvedImportPath, 'client')
      addTrackedClientComponent(resolvedImportPath)

      const clientRefReplacement = buildClientReferenceReplacementFromImport(
        imp,
        clientReferenceIdForPath(resolvedImportPath),
      )
      if (clientRefReplacement.code === '') continue

      for (const helper of clientRefReplacement.helpers) clientRefHelpers.add(helper)
      replacements.push({
        start: imp.start,
        end: imp.end,
        replacement: clientRefReplacement.code,
      })
    }

    if (replacements.length === 0) return modifiedCode

    for (const { start, end, replacement } of [...replacements].sort((a, b) => b.start - a.start)) {
      modifiedCode = modifiedCode.slice(0, start) + replacement + modifiedCode.slice(end)
    }

    if (clientRefHelpers.size > 0) {
      modifiedCode = ensureNamedImportFromModule(modifiedCode, 'react-server-dom-rari/server', [
        ...clientRefHelpers,
      ])
    }

    return modifiedCode
  }

  function trackClientImportsFromAnalysis(id: string, moduleAnalysis: ModuleAnalysis): void {
    for (const importPath of moduleAnalysis.importSources) {
      if (
        !LOCAL_IMPORT_SOURCE_REGEX.test(importPath) &&
        !matchesAliasImport(importPath, resolvedAlias)
      ) {
        continue
      }
      const resolvedImportPath = resolveImportToFilePath(importPath, id, resolvedAlias)
      if (!fs.existsSync(resolvedImportPath)) continue
      const importedAnalysis = moduleAnalysisCache.get(resolvedImportPath)
      if (importedAnalysis.topLevelUseServer) continue
      setComponentType(resolvedImportPath, 'client')
      addTrackedClientComponent(resolvedImportPath)
    }
  }

  function ensureSuspenseReactImport(modifiedCode: string): string {
    if (modifiedCode.includes('Suspense')) return modifiedCode
    const reactImportMatch = REACT_IMPORT_MATCH_REGEX.exec(modifiedCode)
    if (!reactImportMatch) return modifiedCode
    if (reactImportMatch[1] && !reactImportMatch[2].includes('Suspense')) {
      return modifiedCode.replace(
        reactImportMatch[0],
        reactImportMatch[0].replace(IMPORT_SPECIFIERS_REGEX, `{ Suspense, $1 }`),
      )
    }
    if (!reactImportMatch[1]) {
      return modifiedCode.replace(reactImportMatch[0], `import React, { Suspense } from 'react';`)
    }
    return modifiedCode
  }

  function shouldRewriteImportAsClientRef(
    isClientComponent: boolean,
    importingFileIsClient: boolean,
    environmentName: string,
  ): boolean {
    return (
      isClientComponent &&
      !importingFileIsClient &&
      (environmentName === 'rsc' || environmentName === 'ssr')
    )
  }

  function collectUnknownClientRefReplacements(
    code: string,
    id: string,
    environmentName: string,
  ): {
    replacements: Array<{ start: number; end: number; replacement: string }>
    clientRefHelpers: Set<string>
    needsReactImport: boolean
  } | null {
    const importingFileIsClient = id.includes('entry-client')
    const replacements: Array<{ start: number; end: number; replacement: string }> = []
    const clientRefHelpers = new Set<string>()
    let needsReactImport = false

    for (const imp of scanImportStatements(code)) {
      if (imp.typeOnly || imp.sideEffectOnly) continue
      if (
        !LOCAL_IMPORT_SOURCE_REGEX.test(imp.source) &&
        !matchesAliasImport(imp.source, resolvedAlias)
      ) {
        continue
      }

      const resolvedImportPath = resolveImportToFilePath(imp.source, id, resolvedAlias)
      const isClientComponent = isClientBoundaryModule(resolvedImportPath)

      if (isClientComponent) {
        setComponentType(resolvedImportPath, 'client')
        addTrackedClientComponent(resolvedImportPath)
      }

      if (
        !shouldRewriteImportAsClientRef(isClientComponent, importingFileIsClient, environmentName)
      ) {
        continue
      }

      const clientRefReplacement = buildClientReferenceReplacementFromImport(
        imp,
        clientReferenceIdForPath(resolvedImportPath),
      )
      if (clientRefReplacement.code === '') continue

      for (const helper of clientRefReplacement.helpers) clientRefHelpers.add(helper)
      replacements.push({
        start: imp.start,
        end: imp.end,
        replacement: clientRefReplacement.code,
      })
      needsReactImport = true
    }

    if (replacements.length === 0) return null
    return { replacements, clientRefHelpers, needsReactImport }
  }

  function transformUnknownModuleClientRefs(
    code: string,
    id: string,
    environmentName: string,
  ): string | null {
    const collected = collectUnknownClientRefReplacements(code, id, environmentName)
    if (collected == null) return null

    let modifiedCode = code
    for (const { start, end, replacement } of [...collected.replacements].sort(
      (a, b) => b.start - a.start,
    )) {
      modifiedCode = modifiedCode.slice(0, start) + replacement + modifiedCode.slice(end)
    }

    if (collected.clientRefHelpers.size > 0) {
      modifiedCode = ensureNamedImportFromModule(modifiedCode, 'react-server-dom-rari/server', [
        ...collected.clientRefHelpers,
      ])
    }

    const hasReactImport =
      modifiedCode.includes('import React') ||
      REACT_IMPORT_REGEX.test(modifiedCode) ||
      REACT_IMPORT_WITH_DEFAULT_REGEX.test(modifiedCode)

    if (collected.needsReactImport && !hasReactImport) {
      modifiedCode = `import React from 'react';\n${modifiedCode}`
    }
    modifiedCode = ensureSuspenseReactImport(modifiedCode)

    const isDevMode = process.env.NODE_ENV !== 'production'
    const hasJsx =
      modifiedCode.includes('</') ||
      modifiedCode.includes('/>') ||
      JSX_TEST_REGEX.test(modifiedCode)

    if (hasJsx && isDevMode) {
      modifiedCode = `'use client';\n\n${modifiedCode}`
      setComponentType(id, 'client')
    }

    return modifiedCode
  }

  function transformServerComponentForEnv(
    code: string,
    id: string,
    moduleAnalysis: ModuleAnalysis,
    environmentName: string,
  ): string {
    if (environmentName === 'rsc' || environmentName === 'ssr') {
      return rewriteClientImportsForServerEnvironment(
        transformServerModule(code, id, moduleAnalysis),
        id,
      )
    }

    const clientTransformedCode = transformClientModule(code, id, moduleAnalysis)
    return `// HMR acceptance for server component
if (import.meta.hot) {
  import.meta.hot.accept();
  if (typeof globalThis !== 'undefined') {
    if (!globalThis['~rari']) globalThis['~rari'] = {};
    globalThis['~rari'].serverComponents = globalThis['~rari'].serverComponents || new Set();
    globalThis['~rari'].serverComponents.add(${JSON.stringify(id)});
  }
}

${clientTransformedCode}`
  }

  function resolveModuleAnalysisForId(code: string, id: string): ModuleAnalysis {
    if (id.startsWith('\0') || id.includes('virtual:')) return analyzeModuleSource(code)
    try {
      return moduleAnalysisCache.get(id)
    } catch {
      return analyzeModuleSource(code)
    }
  }

  async function applyOptionalUseCacheTransform(
    code: string,
    id: string,
  ): Promise<{ code: string; wasTransformed: boolean }> {
    if (!options.experimental?.useCache && !options.experimental?.useCacheRemote) {
      return { code, wasTransformed: false }
    }
    const transform = await getUseCacheTransform()
    if (!transform) return { code, wasTransformed: false }
    const useCacheResult = transform(code, id)
    if (useCacheResult == null || useCacheResult === '') return { code, wasTransformed: false }
    return { code: useCacheResult, wasTransformed: true }
  }

  function transformByModuleKind(
    code: string,
    id: string,
    moduleAnalysis: ModuleAnalysis,
    environmentName: string,
  ): string | null {
    if (moduleAnalysis.topLevelUseServer) {
      setComponentType(id, 'server')
      if (environmentName === 'rsc') return transformServerModule(code, id, moduleAnalysis)
      return transformClientModule(code, id, moduleAnalysis)
    }

    if (moduleAnalysis.topLevelUseClient) {
      setComponentType(id, 'client')
      addTrackedClientComponent(id)
      trackClientImportsFromAnalysis(id, moduleAnalysis)
      return transformClientModuleForClient(code, id, moduleAnalysis)
    }

    if (
      environmentName !== 'rsc' &&
      environmentName !== 'ssr' &&
      (getComponentType(id) === 'client' || hasTrackedClientComponent(id))
    ) {
      return transformClientModuleForClient(code, id, moduleAnalysis)
    }

    if (isServerComponent(id)) {
      setComponentType(id, 'server')
      return transformServerComponentForEnv(code, id, moduleAnalysis, environmentName)
    }

    const cachedType = getComponentType(id)
    if (cachedType === 'server') {
      if (environmentName === 'rsc' || environmentName === 'ssr') {
        return rewriteClientImportsForServerEnvironment(
          transformServerModule(code, id, moduleAnalysis),
          id,
        )
      }
      return transformClientModule(code, id, moduleAnalysis)
    }
    if (cachedType === 'client') {
      return transformClientModuleForClient(code, id, moduleAnalysis)
    }

    setComponentType(id, 'unknown')
    return transformUnknownModuleClientRefs(code, id, environmentName)
  }

  const mainPlugin: Plugin = {
    name: 'rari',

    config(config: UserConfig, { command }) {
      // Layout owns <html>/<body>; client entry is virtual:rari-entry-client (no index.html).
      config.appType = 'custom'
      config.define ??= {}

      if (shouldDefineRariServerUrl(command)) {
        config.define['import.meta.env.RARI_SERVER_URL'] = JSON.stringify(
          resolveConfiguredRariServerUrl(),
        )
      }

      const existingCssModules = typeof config.css?.modules === 'object' ? config.css.modules : {}
      config.css = {
        ...config.css,
        transformer: config.css?.transformer ?? ('lightningcss' as const),
        modules: { ...existingCssModules, pattern: RARI_CSS_MODULES_PATTERN } as CSSModulesOptions,
      }

      config.resolve ??= {}
      const resolveOptions = config.resolve
      const existingDedupe = Array.isArray(resolveOptions.dedupe) ? resolveOptions.dedupe : []
      resolveOptions.dedupe = [...new Set(existingDedupe).union(new Set(['react', 'react-dom']))]

      const existingAlias = normalizeResolveAliases(resolveOptions)
      const aliasFinds = new Set(existingAlias.map(a => String(a.find)))
      const hasExactReactAlias = existingAlias.some(entry => isExactReactAliasFind(entry.find))
      try {
        resolveOptions.alias = [
          ...existingAlias,
          ...collectReactAliasAppendages(aliasFinds, hasExactReactAlias),
        ]
      } catch (err) {
        if (!isMissingPackageExportError(err)) {
          console.warn('[rari] Unexpected error configuring React aliases:', err)
        }
      }

      applyRariEnvironments(config)

      config.builder ??= {}
      config.builder.sharedPlugins = true
      config.builder.sharedConfigBuild = false

      applyRariOptimizeDeps(config)

      if (command === 'build') {
        for (const envName of ['rsc', 'ssr', 'client']) {
          const env = config.environments![envName]
          if (env.build != null) env.build.rolldownOptions ??= {}
        }
      }

      applyRariServerProxy(config)

      if (command === 'build') applyRariBuildCodeSplitting(config)

      applyRariClientBuildInput(config)

      return config
    },

    async buildStart() {
      resetClientHeadExtras()
      if (options.experimental?.useCache || options.experimental?.useCacheRemote) {
        try {
          await getUseCacheTransform()
        } catch (error) {
          this.error(error instanceof Error ? error : new Error(String(error)))
        }
      }
    },

    configResolved(config) {
      const paths = resolvePluginPaths(config)
      resolvedAssetsDir = paths.assetsDir
      resolvedOutDir = paths.outDir
      Object.assign(resolvedAlias, readViteAliases(config))
    },

    async transform(code, id) {
      if (id.endsWith('.mdx')) invalidateMdxRegistryModuleCache()

      if (/\.(?:tsx?|jsx?|mts|mjs)$/.test(id) && code.includes('defineMdxComponents')) {
        const mdxTransformed = transformDefineMdxComponents({
          code,
          id,
          projectRoot:
            options.projectRoot != null && options.projectRoot !== ''
              ? options.projectRoot
              : process.cwd(),
          resolvedAlias,
        })
        if (mdxTransformed != null && mdxTransformed !== '')
          return emitTransformed(mdxTransformed, code, id)
      }

      if (!TSX_EXT_REGEX.test(id)) return null

      const originalCode = code
      const useCache = await applyOptionalUseCacheTransform(code, id)
      code = useCache.code

      const transformed = transformByModuleKind(
        code,
        id,
        resolveModuleAnalysisForId(code, id),
        this.environment.name,
      )
      if (transformed != null) return emitTransformed(transformed, originalCode, id)
      if (useCache.wasTransformed) return emitTransformed(code, originalCode, id)
      return null
    },

    async configureServer(server) {
      const projectRoot = path.resolve(
        options.projectRoot != null && options.projectRoot !== ''
          ? options.projectRoot
          : process.cwd(),
      )
      const srcDir = path.join(projectRoot, 'src')
      await writeImageConfig(projectRoot, options, resolvedAssetsDir, resolvedOutDir)

      const reactDevtoolsStubFiles = new Map([
        ['/installHook.js.map', 'installHook.js'],
        ['/react_devtools_backend_compact.js.map', 'react_devtools_backend_compact.js'],
        ['/installHook.js', null],
        ['/react_devtools_backend_compact.js', null],
      ])
      server.middlewares.use((req, res, next) => {
        const pathname = pathnameFromUrl(req.url ?? '')
        if (!reactDevtoolsStubFiles.has(pathname)) {
          next()
          return
        }
        const stubFile = reactDevtoolsStubFiles.get(pathname)
        res.statusCode = 200
        res.setHeader('Cache-Control', 'no-store')
        if (stubFile != null) {
          res.setHeader('Content-Type', 'application/json')
          res.end(
            JSON.stringify({
              version: 3,
              file: stubFile,
              sources: [stubFile],
              sourcesContent: ['/* react-devtools extension stub */'],
              names: [],
              mappings: 'AAAA',
            }),
          )
          return
        }
        res.setHeader('Content-Type', 'application/javascript; charset=utf-8')
        res.end('/* react-devtools extension stub */\n')
      })

      const discoverAndRegisterComponents = async () => {
        try {
          const builder = new ServerComponentBuilder(projectRoot, {
            outDir: resolvedOutDir,
            rscDir: 'server',
            manifestPath: 'server/manifest.json',
            serverConfigPath: 'server/config.json',
            alias: resolvedAlias,
            assetsDir: resolvedAssetsDir,
            csp: options.csp,
            cacheControl: options.cacheControl,
            cache: options.cache,
            action: options.action,
            jsPoolSize: options.jsPoolSize,
            origin: options.origin,
            htmlLimitedBots: options.htmlLimitedBots,
            experimental: options.experimental,
            moduleAnalysisCache,
          })

          builder.setViteBuilder(
            await getOrCreateViteEmitBuilder({
              root: projectRoot,
              configFile: server.config.configFile,
              logLevel: 'error',
            }),
          )

          devServerComponentBuilder = builder

          hmrCoordinator ??= new HMRCoordinator(builder, getRariServerPort())

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

          const components = await builder.getTransformedComponentsForDevelopment()

          const baseUrl = `http://localhost:${getRariServerPort()}`

          await Promise.all(
            components.map(async component => {
              try {
                const isAppRouterComponent = component.id.startsWith('app/')
                if (isAppRouterComponent) return

                if (component.isAction) return

                const registerResponse = await fetch(`${baseUrl}/_rari/register`, {
                  method: 'POST',
                  headers: {
                    'Content-Type': 'application/json',
                  },
                  body: JSON.stringify({
                    component_id: component.id,
                    component_code: component.code,
                  }),
                })

                if (!registerResponse.ok) {
                  const errorText = await registerResponse.text()
                  throw new Error(`HTTP ${registerResponse.status}: ${errorText}`)
                }
              } catch (error) {
                console.error(
                  `[rari] Runtime: Failed to register component ${component.id}:`,
                  errorMessage(error, String(error)),
                )
              }
            }),
          )
        } catch (error) {
          console.error(
            '[rari] Runtime: Component discovery failed:',
            errorMessage(error, String(error)),
          )
        }
      }

      const ensureClientComponentsRegistered = async () => {
        try {
          const baseUrl = `http://localhost:${getRariServerPort()}`

          const clientComponentFiles = getKnownClientComponentPaths()

          await Promise.all(
            [...clientComponentFiles].map(async componentPath => {
              const relativePath = path.relative(process.cwd(), componentPath)
              const componentName = path.basename(componentPath).replace(EXTENSION_REGEX, '')

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
              } catch (error) {
                console.error(
                  `[rari] Runtime: Failed to pre-register client component ${componentName}:`,
                  error,
                )
              }
            }),
          )
        } catch (error) {
          console.error('[rari] Runtime: Failed to pre-register client components:', error)
        }
      }

      const startRustServer = async () => {
        if (rustServerProcess) return

        const { getBinaryPath, getInstallationInstructions } = await import('@/cli/platform')

        let binaryPath: string
        try {
          binaryPath = getBinaryPath()
        } catch (error) {
          console.error('rari binary not found')
          console.error(`   ${errorMessage(error, String(error))}`)
          console.error(getInstallationInstructions())
          return
        }

        const serverPort = getRariServerPort()
        const mode = process.env.NODE_ENV === 'production' ? 'production' : 'development'

        const vitePort = server.config.server.port
        const origin = options.origin?.trim().replace(/\/+$/, '')
        const envOrigin = process.env.RARI_ORIGIN?.trim().replace(/\/+$/, '')
        const layoutCssHrefs = collectLayoutCssDevHrefs(projectRoot, resolvedAlias)

        const args = ['--mode', mode, '--port', serverPort.toString(), '--host', '127.0.0.1']

        rustServerProcess = spawn(binaryPath, args, {
          stdio: ['ignore', 'pipe', 'pipe'],
          cwd: projectRoot,
          env: {
            ...process.env,
            RUST_LOG:
              process.env.RUST_LOG != null && process.env.RUST_LOG !== ''
                ? process.env.RUST_LOG
                : 'error',
            RARI_VITE_PORT: vitePort.toString(),
            ...(layoutCssHrefs.length > 0 ? { RARI_DEV_LAYOUT_CSS: layoutCssHrefs.join(',') } : {}),
            // Dev starts the binary before config.json is written; pass pool size / origin / bots via env.
            ...(options.jsPoolSize != null &&
            (process.env.RARI_JS_POOL_SIZE == null || process.env.RARI_JS_POOL_SIZE === '')
              ? { RARI_JS_POOL_SIZE: String(options.jsPoolSize) }
              : {}),
            ...(origin != null && origin !== '' && (envOrigin == null || envOrigin === '')
              ? { RARI_ORIGIN: origin }
              : {}),
            ...(options.htmlLimitedBots != null &&
            (process.env.RARI_HTML_LIMITED_BOTS == null ||
              process.env.RARI_HTML_LIMITED_BOTS === '')
              ? { RARI_HTML_LIMITED_BOTS: options.htmlLimitedBots }
              : {}),
          },
        })

        rustServerProcess.stdout?.on('data', (data: Buffer) => {
          const output = data.toString().trim()
          if (output) console.error(output)
        })

        rustServerProcess.stderr?.on('data', (data: Buffer) => {
          const output = data.toString().trim()
          if (output && !output.includes('warning')) console.error(output)
        })

        rustServerProcess.on('error', (error: Error) => {
          rustServerReady = false
          console.error('Failed to start rari server:', error.message)
          if (error.message.includes('ENOENT')) {
            console.error('   Binary not found. Please ensure rari is properly installed.')
          }
        })

        rustServerProcess.on('exit', (code: number, signal: string) => {
          rustServerProcess = null
          rustServerReady = false
          if (signal) console.error(`rari server stopped by signal ${signal}`)
          else if (code === 0) console.error('rari server stopped successfully')
          else if (code) console.error(`rari server exited with code ${code}`)
        })

        const serverReady = await waitForRustServerReady(10000)

        if (serverReady) {
          await discoverAndRegisterComponents()
          await ensureClientComponentsRegistered()
        } else {
          console.error('Server failed to become ready for component registration')
        }
      }

      const handleServerComponentHMR = async (filePath: string) => {
        try {
          if (!isServerComponent(filePath)) return

          if (devServerComponentBuilder == null) await discoverAndRegisterComponents()

          const builder = devServerComponentBuilder
          if (builder == null) return

          const code = moduleAnalysisCache.getSource(filePath)
          builder.addServerComponent(filePath, code)

          const affectedFiles = walkImporters(builder.getImportGraph(), [filePath])
          affectedFiles.add(filePath)
          const components = await builder.getTransformedComponentsForDevelopment(file =>
            affectedFiles.has(file),
          )

          if (components.length === 0) return

          const baseUrl = `http://localhost:${getRariServerPort()}`

          await Promise.all(
            components.map(async component => {
              try {
                const registerResponse = await fetch(`${baseUrl}/_rari/register`, {
                  method: 'POST',
                  headers: {
                    'Content-Type': 'application/json',
                  },
                  body: JSON.stringify({
                    component_id: component.id,
                    component_code: component.code,
                  }),
                })

                if (!registerResponse.ok) {
                  const errorText = await registerResponse.text()
                  throw new Error(`HTTP ${registerResponse.status}: ${errorText}`)
                }
              } catch (error) {
                console.error(
                  '[rari] Failed to register component',
                  `${component.id}:`,
                  errorMessage(error, String(error)),
                )
              }
            }),
          )
        } catch (error) {
          console.error(
            '[rari] Targeted HMR failed for',
            `${filePath}:`,
            errorMessage(error, String(error)),
          )
        }
      }

      startRustServer().catch((error: unknown) => {
        console.error('[rari] Failed to start Rust server:', error)
      })

      server.middlewares.use((req, res, next) => {
        void (async () => {
          const acceptHeader = req.headers.accept
          const method = req.method ?? 'GET'
          const url = req.url ?? ''
          const pathname = pathnameFromUrl(url)
          const isRscRequest =
            acceptHeader != null && acceptHeader !== '' && acceptHeader.includes('text/x-component')
          const isDocumentRequest = isDocumentHtmlRequest(method, acceptHeader, pathname)

          if (shouldSkipDevProxy(isRscRequest, isDocumentRequest, url, pathname)) {
            next()
            return
          }

          if (!rustServerReady) {
            const ready = await waitForRustServerReady(10000)
            if (!ready) {
              console.error(
                `[rari] Rust server not ready, cannot proxy ${isRscRequest ? 'RSC' : 'HTML'} request`,
              )
              if (!res.headersSent) {
                res.statusCode = 503
                res.end('Server not ready')
              }
              return
            }
          }

          const serverPort = getRariServerPort()
          try {
            const headers = copyProxyRequestHeaders(req.headers, serverPort)
            const hasBody = method !== 'GET' && method !== 'HEAD'
            const body = hasBody
              ? await readRequestBodyAsBlob(req, PROXY_BODY_MAX_BYTES)
              : undefined
            const response = await fetch(`http://localhost:${serverPort}${url}`, {
              method,
              headers,
              ...(body != null ? { body } : {}),
            })
            res.statusCode = response.status
            response.headers.forEach((value, key) => {
              if (key.toLowerCase() !== 'content-encoding') res.setHeader(key, value)
            })
            await streamProxyResponseBody(response, res, method)
          } catch (error) {
            handleDevProxyError(error, res, isRscRequest)
          }
        })()
      })

      server.watcher.on('change', filePath => {
        void (async () => {
          if (TSX_EXT_REGEX.test(filePath)) {
            deleteComponentType(filePath)
            removeTrackedClientComponent(filePath)
            moduleAnalysisCache.invalidate(filePath)
          }

          if (
            TSX_EXT_REGEX.test(filePath) &&
            isPathInside(filePath, srcDir) &&
            isServerComponent(filePath)
          ) {
            server.ws.send({
              type: 'custom',
              event: 'rari:register-server-component',
              data: { filePath },
            })
            await handleServerComponentHMR(filePath)
          }
        })()
      })

      server.middlewares.use('/api/vite/hmr-transform', (req, res) => {
        if (req.method !== 'POST') {
          res.statusCode = 405
          res.end('Method Not Allowed')
          return
        }

        void (async () => {
          try {
            const body = await readRequestText(req)
            const bodyRecord = parseJsonRecord(body)
            const filePath =
              bodyRecord && typeof bodyRecord.filePath === 'string'
                ? bodyRecord.filePath
                : undefined

            if (filePath == null || filePath === '') {
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
          } catch (error) {
            res.statusCode = 500
            res.setHeader('Content-Type', 'application/json')
            res.end(
              JSON.stringify({
                success: false,
                error: errorMessage(error, String(error)),
              }),
            )
          }
        })()
      })

      server.httpServer?.on('close', () => {
        clearViteEmitBuilder(projectRoot)
        if (devServerComponentBuilder != null) {
          devServerComponentBuilder.setViteBuilder(null)
          devServerComponentBuilder = null
        }

        if (hmrCoordinator) {
          hmrCoordinator.dispose()
          hmrCoordinator = null
        }

        if (rustServerProcess) {
          const proc = rustServerProcess
          rustServerProcess = null
          rustServerReady = false

          let exited = false
          const forceKillTimer = setTimeout(() => {
            if (!exited) proc.kill('SIGKILL')
          }, 5000)
          forceKillTimer.unref()
          proc.once('exit', () => {
            exited = true
            clearTimeout(forceKillTimer)
          })
          proc.kill('SIGTERM')
          return
        }

        rustServerReady = false
      })
    },

    resolveId(id, importer) {
      const virtualId = resolveVirtualModuleId(id)
      if (virtualId != null) return virtualId

      if (
        id === 'react-server-dom-webpack/client' ||
        id === 'react-server-dom-webpack/client.browser'
      )
        return 'virtual:react-flight-client.ts'

      if (id === 'react-server-dom-rari/server') return id

      if (importer != null && importer !== '' && importer.startsWith('virtual:')) {
        const resolved = resolveVirtualRelativeImport(id, importer)
        if (resolved != null) return resolved
      }

      return tryResolveProductionServerComponent(id, isServerComponent)
    },

    async load(id) {
      if (TSX_EXT_REGEX.test(id) && this.environment.name === 'client') {
        try {
          const analysis = moduleAnalysisCache.get(id)
          if (analysis.topLevelUseServer) {
            return transformClientModule(moduleAnalysisCache.getSource(id), id, analysis)
          }
        } catch {
          // File doesn't exist or can't be read
        }
      }

      const virtual = await loadNamedVirtualModule(id, {
        buildMdxRegistryModule,
        resolveProjectRoot: () =>
          options.projectRoot != null && options.projectRoot !== ''
            ? options.projectRoot
            : process.cwd(),
        resolvedAlias,
        moduleAnalysisCache,
        getKnownClientComponentPaths,
        environmentMode: this.environment.mode,
      })
      if (virtual !== undefined) return virtual

      const projectRoot =
        options.projectRoot != null && options.projectRoot !== ''
          ? options.projectRoot
          : process.cwd()
      return tryLoadSafeMjsFile(id, projectRoot) ?? undefined
    },

    async handleHotUpdate({ file, server }) {
      clearFileResolverCache()

      if (file.endsWith('.mdx')) invalidateMdxRegistryModuleCache()

      const isReactFile = TSX_EXT_REGEX.test(file)

      if (!isReactFile) return undefined

      deleteComponentType(file)
      removeTrackedClientComponent(file)
      moduleAnalysisCache.invalidate(file)
      invalidateMdxRegistryModuleCache()

      if (file.includes('/dist/') || file.includes('\\dist\\')) return []

      const componentType = hmrCoordinator?.detectComponentType(file) ?? 'unknown'

      const isAppRouterFile = file.includes('/app/') || file.includes('\\app\\')
      const hasExtension = (fileName: string, baseName: string) =>
        fileName.endsWith(`${baseName}.tsx`) ||
        fileName.endsWith(`${baseName}.jsx`) ||
        fileName.endsWith(`${baseName}.ts`) ||
        fileName.endsWith(`${baseName}.js`)

      const SPECIAL_ROUTE_FILE_BASES = [
        'page',
        'layout',
        'template',
        'loading',
        'error',
        'not-found',
      ] as const
      const isSpecialRouteFile = SPECIAL_ROUTE_FILE_BASES.some(base => hasExtension(file, base))

      if (isAppRouterFile && isSpecialRouteFile) {
        if (hmrCoordinator) {
          try {
            await hmrCoordinator.rebuildAndNotifyNow(file, server)
          } catch (error) {
            console.error(
              '[rari] HMR: Failed to rebuild app router file',
              `${file}:`,
              errorMessage(error, String(error)),
            )
          }
        }
        return undefined
      }

      if (componentType === 'client') return undefined

      if (componentType === 'server') {
        if (hmrCoordinator) await hmrCoordinator.handleServerComponentUpdate(file, server)

        return []
      }

      return undefined
    },

    generateBundle(_options, bundle) {
      if (this.environment.name !== 'client') return

      const head = buildClientHeadFromBundle(bundle)

      this.emitFile({
        type: 'asset',
        fileName: CLIENT_HEAD_FILE,
        source: head,
      })
    },

    async writeBundle() {
      const projectRoot =
        options.projectRoot != null && options.projectRoot !== ''
          ? options.projectRoot
          : process.cwd()
      await writeImageConfig(projectRoot, options, resolvedAssetsDir, resolvedOutDir)
    },
  }

  const serverBuildPlugin = createServerBuildPlugin({
    ...options.serverBuild,
    csp: options.csp,
    cacheControl: options.cacheControl,
    cache: options.cache,
    action: options.action,
    jsPoolSize: options.jsPoolSize,
    origin: options.origin,
    htmlLimitedBots: options.htmlLimitedBots,
    experimental: options.experimental,
    moduleAnalysisCache,
    mdx: options.mdx,
  })

  const webpackRequirePatchPlugin: Plugin = {
    name: 'rari:patch-react-server-dom-webpack',
    transform(code) {
      if (
        !code.includes('__webpack_require__') &&
        !code.includes('__webpack_chunk_load__') &&
        !code.includes('__webpack_get_script_filename__')
      )
        return null

      let modifiedCode = code

      if (modifiedCode.includes('__webpack_chunk_load__'))
        modifiedCode = modifiedCode.replaceAll('__webpack_chunk_load__', '__rari_chunk_load__')

      if (modifiedCode.includes('__webpack_get_script_filename__'))
        modifiedCode = modifiedCode.replaceAll(
          '__webpack_get_script_filename__',
          '__rari_get_script_filename__',
        )

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

  const plugins: Plugin[] = [...createReactRefreshPlugins()]

  if (options.compiler != null && options.compiler !== false)
    plugins.push(createReactCompilerPlugin(options.compiler))

  plugins.push(
    mainPlugin,
    createEnvTypesPlugin(
      options.projectRoot != null && options.projectRoot !== ''
        ? options.projectRoot
        : process.cwd(),
    ),
    createSilenceReactDirectiveLogsPlugin(),
    createStaticImagePlugin(),
    createFontPlugin(),
    webpackRequirePatchPlugin,
    serverBuildPlugin,
    createFindSourceMapURLPlugin(
      options.projectRoot != null && options.projectRoot !== ''
        ? options.projectRoot
        : process.cwd(),
    ),
  )

  if (options.proxy !== false) plugins.push(rariProxy(options.proxy ?? {}))

  if (options.router !== false) plugins.push(rariRouter(options.router ?? {}))

  return toRariPlugins(plugins)
}

export function defineRariConfig(
  config: UserConfig & { readonly plugins?: readonly RariPlugin[] },
): UserConfig {
  return {
    ...config,
    plugins: [rari(), ...(config.plugins ?? [])],
  }
}

export type { RariPlugin } from './plugin/types'

export type Request = globalThis.Request
export type Response = globalThis.Response

export type {
  ServerActionConfig,
  ServerCacheConfig,
  ServerCacheControlConfig,
  ServerCacheLayerConfig,
  ServerConfig,
  ServerCSPConfig,
  ServerUseCacheConfig,
} from './server/config'

export type { RariCompilerOption, ReactCompilerOptions } from './transform/react-compiler'
// oxlint-disable-next-line typescript/no-useless-empty-export side-effect import of ambient declarations
export type {} from '@/ambient'

export { rariProxy } from '@/proxy/build/vite-plugin'

export type { ProxyPluginOptions } from '@/proxy/build/vite-plugin'

export type {
  CookieOptions,
  ProxyConfig,
  ProxyFunction,
  ProxyMatcher,
  ProxyModule,
  ProxyResult,
  RariFetchEvent,
  RariURL,
  RequestCookies,
  ResponseCookies,
} from '@/proxy/http/types'

export type { ApiRouteHandlers, RouteContext, RouteHandler } from '@/router/build/api-routes'

export { ApiResponse } from '@/router/build/api-routes'

export {
  clearPropsCache,
  clearPropsCacheForComponent,
  extractMetadata,
  extractServerProps,
  extractServerPropsWithCache,
  extractStaticParams,
  hasServerSideDataFetching,
} from '@/router/build/props-extractor'

export type {
  MetadataResult,
  ServerSidePropsResult,
  StaticParamsResult,
} from '@/router/build/props-extractor'

export { generateAppRouteManifest } from '@/router/build/routes'

export type {
  AppRouteEntry,
  AppRouteManifest,
  AppRouteMatch,
  ErrorEntry,
  ErrorProps,
  GenerateMetadata,
  GenerateStaticParams,
  LayoutEntry,
  LayoutProps,
  LoadingEntry,
  NotFoundEntry,
  PageProps,
  RouteSegment,
  RouteSegmentType,
  TemplateEntry,
} from '@/router/build/types'

export type { Metadata } from '@/router/build/types'

export { rariRouter } from '@/router/build/vite-plugin'

export type {
  Robots,
  RobotsRule,
  Sitemap,
  SitemapEntry,
  SitemapImage,
  SitemapVideo,
} from '@/router/metadata/types'
