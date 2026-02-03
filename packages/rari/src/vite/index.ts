import type { Plugin, UserConfig } from 'rolldown-vite'
import type { ProxyPluginOptions } from '../proxy/vite-plugin'
import type { ServerBuildOptions } from './server-build'
import { Buffer } from 'node:buffer'
import { spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import { DEFAULT_DEVICE_SIZES, DEFAULT_FORMATS, DEFAULT_IMAGE_SIZES, DEFAULT_MAX_CACHE_SIZE, DEFAULT_MINIMUM_CACHE_TTL, DEFAULT_QUALITY_LEVELS } from '../image/constants'
import { rariProxy } from '../proxy/vite-plugin'
import { rariRouter } from '../router/vite-plugin'
import { HMRCoordinator } from './hmr-coordinator'
import { scanForImageUsage } from './image-scanner'
import { createServerBuildPlugin } from './server-build'

interface RouterPluginOptions {
  appDir?: string
  extensions?: string[]
}

interface RariOptions {
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
  rateLimit?: {
    enabled?: boolean
    requestsPerSecond?: number
    burstSize?: number
    revalidateRequestsPerMinute?: number
  }
  spamBlocker?: {
    enabled?: boolean
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

async function loadRuntimeFile(filename: string): Promise<string> {
  const currentFileUrl = import.meta.url
  const currentFilePath = fileURLToPath(currentFileUrl)
  const currentDir = path.dirname(currentFilePath)

  const possiblePaths = [
    path.join(currentDir, 'runtime', filename),
    path.join(currentDir, '../runtime', filename),
    path.join(currentDir, '../src/runtime', filename.replace('.mjs', '.ts')),
  ]

  for (const filePath of possiblePaths) {
    try {
      let content = await fs.promises.readFile(filePath, 'utf-8')

      if (filePath.endsWith('.ts')) {
        content = content.replace(
          /import\s+type\s+(\{[^}]+\})\s+from\s+["']\.\.\/([^"']+)["'];?/g,
          (match, specifier, modulePath) => `import type ${specifier} from "rari/${modulePath}";`,
        )

        content = content.replace(
          /import\s+type\s+(\*\s+as\s+\w+)\s+from\s+["']\.\.\/([^"']+)["'];?/g,
          (match, specifier, modulePath) => `import type ${specifier} from "rari/${modulePath}";`,
        )

        content = content.replace(
          /import\s+type\s+(\w+)\s+from\s+["']\.\.\/([^"']+)["'];?/g,
          (match, specifier, modulePath) => `import type ${specifier} from "rari/${modulePath}";`,
        )

        content = content.replace(
          /import\s+(\{[^}]+\})\s+from\s+["']\.\.\/([^"']+)["'];?/g,
          (match, specifier, modulePath) => `import ${specifier} from "rari/${modulePath}";`,
        )

        content = content.replace(
          /import\s+(\*\s+as\s+\w+)\s+from\s+["']\.\.\/([^"']+)["'];?/g,
          (match, specifier, modulePath) => `import ${specifier} from "rari/${modulePath}";`,
        )

        content = content.replace(
          /import\s+(\w+)\s+from\s+["']\.\.\/([^"']+)["'];?/g,
          (match, specifier, modulePath) => `import ${specifier} from "rari/${modulePath}";`,
        )

        content = content.replace(
          /import\s+["']\.\.\/([^"']+)["'];?/g,
          (match, modulePath) => `import "rari/${modulePath}";`,
        )
      }

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

async function loadRscClientRuntime(): Promise<string> {
  return loadRuntimeFile('rsc-client-runtime.mjs')
}

async function loadEntryClient(imports: string, registrations: string): Promise<string> {
  const template = await loadRuntimeFile('entry-client.mjs')
  return template
    .replace('/*! @preserve CLIENT_COMPONENT_IMPORTS_PLACEHOLDER */', imports)
    .replace('/*! @preserve CLIENT_COMPONENT_REGISTRATIONS_PLACEHOLDER */', registrations)
}

async function loadReactServerDomShim(): Promise<string> {
  return loadRuntimeFile('react-server-dom-shim.mjs')
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

function scanForClientComponents(srcDir: string, additionalDirs: string[] = []): Set<string> {
  const clientComponents = new Set<string>()

  function scanDirectory(dir: string) {
    if (!fs.existsSync(dir))
      return

    const entries = fs.readdirSync(dir, { withFileTypes: true })

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name)

      if (entry.isDirectory()) {
        if (entry.name === 'node_modules')
          continue
        scanDirectory(fullPath)
      }
      else if (entry.isFile() && /\.(?:tsx?|jsx?)$/.test(entry.name)) {
        try {
          const content = fs.readFileSync(fullPath, 'utf8')
          if (
            content.includes('\'use client\'')
            || content.includes('"use client"')
          ) {
            clientComponents.add(fullPath)
          }
        }
        catch {}
      }
    }
  }

  scanDirectory(srcDir)

  for (const dir of additionalDirs) {
    if (fs.existsSync(dir))
      scanDirectory(dir)
  }

  return clientComponents
}

export function defineRariOptions(config: RariOptions): RariOptions {
  return config
}

export function rari(options: RariOptions = {}): Plugin[] {
  const componentTypeCache = new Map<string, 'client' | 'server' | 'unknown'>()
  const clientComponents = new Set<string>()
  let rustServerProcess: any = null

  let hmrCoordinator: HMRCoordinator | null = null
  const resolvedAlias: Record<string, string> = {}

  function isServerComponent(filePath: string): boolean {
    if (filePath.includes('node_modules') || (filePath.includes('/rari/dist/') || filePath.includes('\\rari\\dist\\')))
      return false

    const projectRoot = options.projectRoot || process.cwd()
    const indexHtmlPath = path.join(projectRoot, 'index.html')

    if (fs.existsSync(indexHtmlPath)) {
      try {
        const htmlContent = fs.readFileSync(indexHtmlPath, 'utf-8')
        const importRegex = /import\s*(?:\(\s*)?["']([^"']+)["']\)?/g

        for (const match of htmlContent.matchAll(importRegex)) {
          const importPath = match[1]
          if (importPath.startsWith('/src/')) {
            const absolutePath = path.join(projectRoot, importPath.slice(1))
            if (absolutePath === filePath)
              return false
          }
        }
      }
      catch {}
    }

    let pathForFsOperations
    try {
      pathForFsOperations = fs.realpathSync(filePath)
    }
    catch {
      return false
    }

    try {
      if (!fs.existsSync(pathForFsOperations))
        return false

      const code = fs.readFileSync(pathForFsOperations, 'utf-8')
      const hasClientDirective = hasTopLevelDirective(code, 'use client')
      const hasServerDirective = hasTopLevelDirective(code, 'use server')

      if (hasServerDirective)
        return false

      return !hasClientDirective
    }
    catch {
      return false
    }
  }

  function parseExportedNames(code: string): string[] {
    try {
      const exportedNames: string[] = []
      const namedExportMatch = code.matchAll(/export\s*\{([^}]+)\}/g)
      for (const match of namedExportMatch) {
        const exports = match[1].split(',')
        for (const exp of exports) {
          const trimmed = exp.trim()
          const parts = trimmed.split(/\s+as\s+/)
          const exportedName = parts.at(-1)?.trim()
          if (exportedName)
            exportedNames.push(exportedName)
        }
      }

      if (/export\s+default\s+(?:function|class)\s+\w+/.test(code))
        exportedNames.push('default')
      else if (/export\s+default\s+/.test(code))
        exportedNames.push('default')

      const declarationExports = code.matchAll(/export\s+(?:const|let|var|function|class)\s+(\w+)/g)
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

  function hasTopLevelDirective(
    code: string,
    directive: 'use client' | 'use server',
  ): boolean {
    try {
      const lines = code.split('\n')

      for (const line of lines) {
        const trimmed = line.trim()

        if (!trimmed || trimmed.startsWith('//') || trimmed.startsWith('/*'))
          continue

        if (trimmed === `'${directive}'` || trimmed === `"${directive}"`
          || trimmed === `'${directive}';` || trimmed === `"${directive}";`) {
          return true
        }

        break
      }

      return false
    }
    catch {
      return false
    }
  }

  function transformServerModule(code: string, id: string): string {
    const hasUseServer = hasTopLevelDirective(code, 'use server')

    if (!hasUseServer)
      return code

    const exportedNames = parseExportedNames(code)
    if (exportedNames.length === 0)
      return code

    let newCode = code
    newCode
      += '\n\nimport {registerServerReference} from "react-server-dom-rari/server";\n'

    for (const name of exportedNames) {
      if (name === 'default') {
        const functionDeclRegex
          = /export\s+default\s+(?:async\s+)?function\s+(\w+)/
        const functionDeclMatch = code.match(functionDeclRegex)

        if (functionDeclMatch) {
          const functionName = functionDeclMatch[1]
          newCode += `\n// Register server reference for default export\n`
          newCode += `registerServerReference(${functionName}, ${JSON.stringify(id)}, ${JSON.stringify(name)});\n`
        }
        else {
          const defaultExportRegex = /export\s+default\s+([^;]+)/
          const match = code.match(defaultExportRegex)
          if (match) {
            const exportedValue = match[1].trim()
            const tempVarName = '__default_export__'
            newCode = newCode.replace(
              defaultExportRegex,
              `const ${tempVarName} = ${exportedValue};\nexport default ${tempVarName}`,
            )
            newCode += `\n// Register server reference for default export\n`
            newCode += `if (typeof ${tempVarName} === "function") {\n`
            newCode += `  registerServerReference(${tempVarName}, ${JSON.stringify(id)}, ${JSON.stringify(name)});\n`
            newCode += `}\n`
          }
        }
      }
      else {
        newCode += `\n// Register server reference for ${name}\n`
        newCode += `if (typeof ${name} === "function") {\n`
        newCode += `  registerServerReference(${name}, ${JSON.stringify(id)}, ${JSON.stringify(name)});\n`
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

  function transformClientModule(code: string, id: string): string {
    const isServerFunction = hasTopLevelDirective(code, 'use server')
    const isServerComp = isServerComponent(id)

    if (isServerFunction) {
      const exportedNames = parseExportedNames(code)
      if (exportedNames.length === 0)
        return ''

      const relativePath = path.relative(process.cwd(), id)
      const moduleId = relativePath
        .replace(/\\/g, '/')
        .replace(/\.(tsx?|jsx?)$/, '')
        .replace(/[^\w/-]/g, '_')
        .replace(/^src\//, '')

      let newCode = 'import { createServerReference } from "rari/runtime/actions";\n'

      for (const name of exportedNames) {
        if (name === 'default')
          newCode += `export default createServerReference("default", ${JSON.stringify(moduleId)}, "default");\n`
        else
          newCode += `export const ${name} = createServerReference("${name}", ${JSON.stringify(moduleId)}, "${name}");\n`
      }

      return newCode
    }

    if (isServerComp) {
      const exportedNames = parseExportedNames(code)
      if (exportedNames.length === 0)
        return ''

      const relativePath = path.relative(process.cwd(), id)
      const componentId = relativePath
        .replace(/\\/g, '/')
        .replace(/\.(tsx?|jsx?)$/, '')
        .replace(/[^\w/-]/g, '_')
        .replace(/^src\//, '')
        .replace(/^components\//, '')

      let newCode
        = 'import { createServerComponentWrapper } from "virtual:rsc-integration.ts";\n'

      for (const name of exportedNames) {
        if (name === 'default')
          newCode += `export default createServerComponentWrapper("${componentId}", ${JSON.stringify(id)});\n`
        else
          newCode += `export const ${name} = createServerComponentWrapper("${componentId}_${name}", ${JSON.stringify(id)});\n`
      }

      return newCode
    }

    if (!hasTopLevelDirective(code, 'use client'))
      return code

    const exportedNames = parseExportedNames(code)
    if (exportedNames.length === 0)
      return ''

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
      newCode += `${JSON.stringify(id)},`
      newCode += `${JSON.stringify(name)});\n`
    }

    return newCode
  }

  function transformClientModuleForClient(code: string, _id: string): string {
    if (!hasTopLevelDirective(code, 'use client'))
      return code

    const exportedNames = parseExportedNames(code)
    if (exportedNames.length === 0)
      return code

    return code.replace(/^['"]use client['"];?\s*$/gm, '')
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
    for (const ext of extensions) {
      const pathWithExt = `${resolvedPath}${ext}`
      if (fs.existsSync(pathWithExt))
        return pathWithExt
    }

    if (fs.existsSync(resolvedPath)) {
      for (const ext of extensions) {
        const indexPath = path.join(resolvedPath, `index${ext}`)
        if (fs.existsSync(indexPath))
          return indexPath
      }
    }

    return `${resolvedPath}.tsx`
  }

  function getComponentName(importPath: string): string {
    const lastSegment = importPath.split('/').pop() || importPath
    return lastSegment.replace(/\.[^.]*$/, '')
  }

  const serverComponentBuilder: any = null
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
      if (command === 'build') {
        const projectRoot = options.projectRoot || process.cwd()
        const indexHtmlPath = path.join(projectRoot, 'index.html')

        if (fs.existsSync(indexHtmlPath)) {
          try {
            const htmlContent = fs.readFileSync(indexHtmlPath, 'utf-8')
            const importRegex = /import\s+["']([^"']+)["']/g
            const htmlImports: Array<{ path: string, name: string }> = []

            for (const match of htmlContent.matchAll(importRegex)) {
              const importPath = match[1]
              if (importPath.startsWith('/src/') && /\.(?:tsx?|jsx?)$/.test(importPath)) {
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
        catch {}
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
      catch {}

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
      }

      if (config.environments && config.environments.client) {
        if (!config.environments.client.build)
          config.environments.client.build = {}
        if (!config.environments.client.build.rolldownOptions)
          config.environments.client.build.rolldownOptions = {}
        if (!config.environments.client.build.rolldownOptions.input)
          config.environments.client.build.rolldownOptions.input = {}
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

    transform(code, id) {
      if (!/\.(?:tsx?|jsx?)$/.test(id))
        return null

      const environment = (this as any).environment

      if (hasTopLevelDirective(code, 'use client')) {
        componentTypeCache.set(id, 'client')
        clientComponents.add(id)

        const importRegex
          = /^\s*import\s+(?:(\w+)(?:\s*,\s*\{\s*(?:(\w+(?:\s*,\s*\w+)*)\s*)?\})?|\{\s*(\w+(?:\s*,\s*\w+)*)\s*\})\s+from\s+['"]([./@][^'"]+)['"].*$/
        const lines = code.split('\n')

        for (const line of lines) {
          const importMatch = line.match(importRegex)
          if (!importMatch)
            continue

          const importPath = importMatch[4]
          if (!importPath)
            continue

          const resolvedImportPath = resolveImportToFilePath(importPath, id)

          if (fs.existsSync(resolvedImportPath)) {
            componentTypeCache.set(resolvedImportPath, 'client')
            clientComponents.add(resolvedImportPath)
          }
        }

        return transformClientModuleForClient(code, id)
      }

      if (componentTypeCache.get(id) === 'client' || clientComponents.has(id))
        return transformClientModuleForClient(code, id)

      if (isServerComponent(id)) {
        componentTypeCache.set(id, 'server')

        if (
          environment
          && (environment.name === 'rsc' || environment.name === 'ssr')
        ) {
          return transformServerModule(code, id)
        }
        else {
          let clientTransformedCode = transformClientModule(code, id)

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

      if (hasTopLevelDirective(code, 'use server')) {
        componentTypeCache.set(id, 'server')

        if (
          environment
          && (environment.name === 'rsc' || environment.name === 'ssr')
        ) {
          return transformServerModule(code, id)
        }
        else {
          return transformClientModule(code, id)
        }
      }

      const cachedType = componentTypeCache.get(id)
      if (cachedType === 'server')
        return transformServerModule(code, id)
      if (cachedType === 'client')
        return transformClientModuleForClient(code, id)

      componentTypeCache.set(id, 'unknown')

      const lines = code.split('\n')
      let modifiedCode = code
      let hasServerImports = false
      let needsReactImport = false
      let needsWrapperImport = false
      const serverComponentReplacements: string[] = []

      const importRegex
        = /^\s*import\s+(\w+)(?:\s*,\s*\{\s*(?:(\w+(?:\s*,\s*\w+)*)\s*)?\})?\s+from\s+['"]([./@][^'"]+)['"].*$/

      for (const line of lines) {
        const importMatch = line.match(importRegex)
        if (!importMatch)
          continue

        const importedDefault = importMatch[1]
        const importPath = importMatch[3]
        const componentName = getComponentName(importPath)
        const resolvedImportPath = resolveImportToFilePath(importPath, id)

        const importingFileIsClient = hasTopLevelDirective(code, 'use client')
          || componentTypeCache.get(id) === 'client'
          || id.includes('entry-client')

        const isClientComponent
          = componentTypeCache.get(resolvedImportPath) === 'client'
            || (fs.existsSync(resolvedImportPath)
              && fs
                .readFileSync(resolvedImportPath, 'utf-8')
                .includes('\'use client\''))

        if (isClientComponent) {
          componentTypeCache.set(resolvedImportPath, 'client')
          clientComponents.add(resolvedImportPath)
        }

        if (
          isClientComponent
          && environment
          && (environment.name === 'rsc' || environment.name === 'ssr')
        ) {
          if (!importingFileIsClient) {
            const originalImport = line
            const componentName = importedDefault || 'default'

            const clientRefReplacement = `
import { registerClientReference } from "react-server-dom-rari/server";
const ${componentName} = registerClientReference(
  function() {
    throw new Error("Attempted to call ${componentName} from the server but it's on the client. It can only be rendered as a Component or passed to props of a Client Component.");
  },
  ${JSON.stringify(resolvedImportPath)},
  ${JSON.stringify(importedDefault || 'default')}
);`

            modifiedCode = modifiedCode.replace(
              originalImport,
              clientRefReplacement,
            )
            hasServerImports = true
            needsReactImport = true
          }
        }
        else if (!importingFileIsClient && isServerComponent(resolvedImportPath)) {
          hasServerImports = true
          needsReactImport = true
          needsWrapperImport = true

          const originalImport = line

          if (importedDefault && importedDefault !== '_') {
            serverComponentReplacements.push(
              `const ${importedDefault} = createServerComponentWrapper('${componentName}', '${importPath}');`,
            )
          }

          modifiedCode = modifiedCode.replace(originalImport, '')
        }
      }

      if (hasServerImports) {
        const hasReactImport
          = modifiedCode.includes('import React')
            || modifiedCode.match(/import\s+\{[^}]*\}\s+from\s+['"]react['"]/)
            || modifiedCode.match(
              /import\s+[^,\s]+\s*,\s*\{[^}]*\}\s+from\s+['"]react['"]/,
            )

        const hasWrapperImport = modifiedCode.includes(
          'createServerComponentWrapper',
        )

        let importsToAdd = ''

        if (needsReactImport && !hasReactImport)
          importsToAdd += `import React from 'react';\n`
        if (needsWrapperImport && !hasWrapperImport)
          importsToAdd += `import { createServerComponentWrapper } from 'virtual:rsc-integration.ts';\n`
        if (serverComponentReplacements.length > 0)
          importsToAdd += `${serverComponentReplacements.join('\n')}\n`
        if (importsToAdd)
          modifiedCode = importsToAdd + modifiedCode
        if (!modifiedCode.includes('Suspense')) {
          const reactImportMatch = modifiedCode.match(
            /import React(,\s*\{([^}]*)\})?\s+from\s+['"]react['"];?/,
          )
          if (reactImportMatch) {
            if (
              reactImportMatch[1]
              && !reactImportMatch[2].includes('Suspense')
            ) {
              modifiedCode = modifiedCode.replace(
                reactImportMatch[0],
                reactImportMatch[0].replace(/\{([^}]*)\}/, `{ Suspense, $1 }`),
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
            || /\bJSX\b/.test(modifiedCode)

        if (
          !modifiedCode.includes('\'use client\'')
          && !modifiedCode.includes('"use client"')
          && hasJsx
        ) {
          if (isDevMode) {
            modifiedCode = `'use client';\n\n${modifiedCode}`
            componentTypeCache.set(id, 'client')
          }
        }

        return modifiedCode
      }

      return null
    },

    async configureServer(server) {
      const projectRoot = options.projectRoot || process.cwd()
      const srcDir = path.join(projectRoot, 'src')
      await writeImageConfig(projectRoot, options)

      let serverComponentBuilder: any = null

      const discoverAndRegisterComponents = async () => {
        try {
          const { ServerComponentBuilder, scanDirectory } = await import(
            './server-build',
          )

          const builder = new ServerComponentBuilder(projectRoot, {
            outDir: 'dist',
            rscDir: 'server',
            manifestPath: 'server/manifest.json',
            serverConfigPath: 'server/config.json',
            alias: resolvedAlias,
            csp: options.csp,
            rateLimit: options.rateLimit,
            spamBlocker: options.spamBlocker,
          })

          serverComponentBuilder = builder

          if (!hmrCoordinator && serverComponentBuilder) {
            const serverPort = process.env.SERVER_PORT
              ? Number(process.env.SERVER_PORT)
              : Number(process.env.PORT || process.env.RSC_PORT || 3000)
            hmrCoordinator = new HMRCoordinator(serverComponentBuilder, serverPort)
          }

          const srcDir = path.join(projectRoot, 'src')
          const serverComponentPaths: string[] = []

          if (fs.existsSync(srcDir)) {
            const collectServerComponents = (dir: string) => {
              const entries = fs.readdirSync(dir, { withFileTypes: true })
              for (const entry of entries) {
                const fullPath = path.join(dir, entry.name)
                if (entry.isDirectory()) {
                  collectServerComponents(fullPath)
                }
                else if (entry.isFile() && /\.(?:tsx?|jsx?)$/.test(entry.name)) {
                  try {
                    if (isServerComponent(fullPath))
                      serverComponentPaths.push(fullPath)
                  }
                  catch (error) {
                    console.error(`[rari] Error checking ${fullPath}:`, error)
                  }
                }
              }
            }

            collectServerComponents(srcDir)
            scanDirectory(srcDir, builder)
          }

          if (serverComponentPaths.length > 0) {
            server.ws.send({
              type: 'custom',
              event: 'rari:server-components-registry',
              data: { serverComponents: serverComponentPaths },
            })
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

              if (component.code.includes('"use server"') || component.code.includes('\'use server\''))
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

          const clientComponentFiles = scanForClientComponents(srcDir, Object.values(resolvedAlias))

          for (const componentPath of clientComponentFiles) {
            const relativePath = path.relative(process.cwd(), componentPath)
            const componentName = path
              .basename(componentPath)
              .replace(/\.[^.]+$/, '')

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
          '../platform',
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
          await ensureClientComponentsRegistered()
          await discoverAndRegisterComponents()
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

          const { ServerComponentBuilder } = await import('./server-build')
          const builder = new ServerComponentBuilder(projectRoot, {
            outDir: 'dist',
            rscDir: 'server',
            manifestPath: 'server/manifest.json',
            serverConfigPath: 'server/config.json',
            alias: resolvedAlias,
            csp: options.csp,
            rateLimit: options.rateLimit,
            spamBlocker: options.spamBlocker,
          })

          builder.addServerComponent(filePath)

          const components
            = await builder.getTransformedComponentsForDevelopment()

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
        if (/\.(?:tsx?|jsx?)$/.test(filePath))
          componentTypeCache.delete(filePath)

        if (/\.(?:tsx?|jsx?)$/.test(filePath) && filePath.includes(srcDir)) {
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
      if (id === 'virtual:react-server-dom-rari-client' || id === 'virtual:react-server-dom-rari-client.ts')
        return 'virtual:react-server-dom-rari-client.ts'
      if (id === 'virtual:app-router-provider' || id === 'virtual:app-router-provider.tsx')
        return 'virtual:app-router-provider.tsx'
      if (id === './DefaultLoadingIndicator' || id === './DefaultLoadingIndicator.tsx')
        return 'virtual:default-loading-indicator.tsx'
      if (id === './LoadingErrorBoundary' || id === './LoadingErrorBoundary.tsx')
        return 'virtual:loading-error-boundary.tsx'
      if (id === '../router/LoadingComponentRegistry' || id === '../router/LoadingComponentRegistry.ts')
        return 'virtual:loading-component-registry.ts'
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
          path.join(currentDir, '../src/runtime'),
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
        catch {}
      }

      return null
    },

    async load(id) {
      if (id === 'virtual:rari-entry-client.ts') {
        const srcDir = path.join(process.cwd(), 'src')
        const scannedClientComponents = scanForClientComponents(srcDir, Object.values(resolvedAlias))

        const allClientComponents = new Set([
          ...clientComponents,
          ...scannedClientComponents,
        ])

        const externalClientComponents = [
          { path: 'rari/image', exports: ['Image'] },
        ]

        const clientComponentsArray = [...allClientComponents].filter((componentPath) => {
          try {
            const code = fs.readFileSync(componentPath, 'utf-8')
            return hasTopLevelDirective(code, 'use client')
          }
          catch {
            return false
          }
        })

        const imports = clientComponentsArray.map((componentPath, index) => {
          const relativePath = path.relative(process.cwd(), componentPath).replace(/\\/g, '/')
          const componentName = `ClientComponent${index}`

          try {
            const code = fs.readFileSync(componentPath, 'utf-8')
            const hasDefaultExport = /export\s+default\s+/.test(code)
            const namedExportMatch = code.match(/export\s+(?:function|const|class)\s+(\w+)/)

            if (!hasDefaultExport && namedExportMatch) {
              const exportName = namedExportMatch[1]
              return `import { ${exportName} as ${componentName} } from '/${relativePath}';`
            }
          }
          catch {}

          return `import ${componentName} from '/${relativePath}';`
        }).join('\n')

        const registrations = clientComponentsArray.map((componentPath, index) => {
          const relativePath = path.relative(process.cwd(), componentPath).replace(/\\/g, '/')
          const componentId = path.basename(componentPath, path.extname(componentPath))
          const registrationPath = relativePath.startsWith('..') ? componentPath.replace(/\\/g, '/') : relativePath

          return `
globalThis['~clientComponents']["${registrationPath}"] = {
  id: "${componentId}",
  path: "${registrationPath}",
  type: "client",
  component: ${`ClientComponent${index}`},
  registered: true
};
globalThis['~clientComponents']["${componentId}"] = globalThis['~clientComponents']["${registrationPath}"];
globalThis['~clientComponentPaths']["${registrationPath}"] = "${componentId}";`
        }).join('\n')

        const externalImports = externalClientComponents.map((ext, index) => {
          const componentNames = ext.exports.map(exp => `${exp} as External${index}_${exp}`).join(', ')
          return `import { ${componentNames} } from '${ext.path}';`
        }).join('\n')

        const externalRegistrations = externalClientComponents.flatMap((ext, index) => {
          return ext.exports.map((exportName) => {
            const componentName = `External${index}_${exportName}`
            const fullId = `${ext.path}#${exportName}`
            return `
globalThis['~clientComponents'] = globalThis['~clientComponents'] || {};
globalThis['~clientComponents']["${fullId}"] = {
  id: "${exportName}",
  path: "${ext.path}",
  type: "client",
  component: ${componentName},
  registered: true
};
globalThis['~clientComponents']["${ext.path}"] = globalThis['~clientComponents']["${ext.path}"] || {};
globalThis['~clientComponents']["${ext.path}"].component = ${componentName};
globalThis['~clientComponentPaths'] = globalThis['~clientComponentPaths'] || {};
globalThis['~clientComponentPaths']["${ext.path}"] = "${exportName}";`
          })
        }).join('\n')

        const allImports = [imports, externalImports].filter(Boolean).join('\n')
        const allRegistrations = [registrations, externalRegistrations].filter(Boolean).join('\n')

        return await loadEntryClient(allImports, allRegistrations)
      }

      if (id === 'react-server-dom-rari/server')
        return await loadReactServerDomShim()

      if (id === 'virtual:app-router-provider.tsx') {
        const possiblePaths = [
          path.join(process.cwd(), 'packages/rari/src/runtime/AppRouterProvider.tsx'),
          path.join(process.cwd(), 'src/runtime/AppRouterProvider.tsx'),
          path.join(process.cwd(), 'node_modules/rari/src/runtime/AppRouterProvider.tsx'),
        ]

        for (const providerSourcePath of possiblePaths) {
          if (fs.existsSync(providerSourcePath))
            return fs.readFileSync(providerSourcePath, 'utf-8')
        }

        return 'export function AppRouterProvider({ children }) { return children; }'
      }

      if (id === 'virtual:default-loading-indicator.tsx') {
        const possiblePaths = [
          path.join(process.cwd(), 'packages/rari/src/runtime/DefaultLoadingIndicator.tsx'),
          path.join(process.cwd(), 'src/runtime/DefaultLoadingIndicator.tsx'),
          path.join(process.cwd(), 'node_modules/rari/src/runtime/DefaultLoadingIndicator.tsx'),
        ]

        for (const sourcePath of possiblePaths) {
          if (fs.existsSync(sourcePath))
            return fs.readFileSync(sourcePath, 'utf-8')
        }

        return 'export function DefaultLoadingIndicator() { return null; }'
      }

      if (id === 'virtual:loading-error-boundary.tsx') {
        const possiblePaths = [
          path.join(process.cwd(), 'packages/rari/src/runtime/LoadingErrorBoundary.tsx'),
          path.join(process.cwd(), 'src/runtime/LoadingErrorBoundary.tsx'),
          path.join(process.cwd(), 'node_modules/rari/src/runtime/LoadingErrorBoundary.tsx'),
        ]

        for (const sourcePath of possiblePaths) {
          if (fs.existsSync(sourcePath))
            return fs.readFileSync(sourcePath, 'utf-8')
        }

        return 'export class LoadingErrorBoundary extends React.Component { render() { return this.props.children; } }'
      }

      if (id === 'virtual:loading-component-registry.ts') {
        const possiblePaths = [
          path.join(process.cwd(), 'packages/rari/src/router/LoadingComponentRegistry.ts'),
          path.join(process.cwd(), 'src/router/LoadingComponentRegistry.ts'),
          path.join(process.cwd(), 'node_modules/rari/src/router/LoadingComponentRegistry.ts'),
        ]

        for (const sourcePath of possiblePaths) {
          if (fs.existsSync(sourcePath))
            return fs.readFileSync(sourcePath, 'utf-8')
        }

        return 'export class LoadingComponentRegistry { loadComponent() { return Promise.resolve(null); } }'
      }

      if (id === 'virtual:rsc-integration.ts') {
        const code = await loadRscClientRuntime()
        return code.replace(
          /from(\s*)(['"])\.\/react-server-dom-rari-client\.mjs\2/g,
          (match, whitespace, quote) => `from${whitespace}${quote}virtual:react-server-dom-rari-client.ts${quote}`,
        )
      }

      if (id === 'virtual:react-server-dom-rari-client.ts')
        return await loadRuntimeFile('react-server-dom-rari-client.mjs')

      if (id.endsWith('.mjs') && fs.existsSync(id)) {
        try {
          const projectRoot = options.projectRoot || process.cwd()
          const realId = fs.realpathSync(id)
          const relativeToRoot = path.relative(projectRoot, realId)

          const isInProjectRoot = !relativeToRoot.startsWith('..') && !path.isAbsolute(relativeToRoot)
          const isInNodeModules = realId.includes(`${path.sep}node_modules${path.sep}`)

          if (isInProjectRoot || isInNodeModules) {
            return fs.readFileSync(id, 'utf-8')
          }

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
      const isReactFile = /\.(?:tsx?|jsx?)$/.test(file)

      if (!isReactFile)
        return undefined

      if (file.includes('/dist/') || file.includes('\\dist\\'))
        return []

      const componentType = hmrCoordinator?.detectComponentType(file) || 'unknown'

      const isAppRouterFile = file.includes('/app/') || file.includes('\\app\\')
      const isPageFile = file.endsWith('page.tsx') || file.endsWith('page.jsx')
      const isLayoutFile = file.endsWith('layout.tsx') || file.endsWith('layout.jsx')
      const isLoadingFile = file.endsWith('loading.tsx') || file.endsWith('loading.jsx')
      const isErrorFile = file.endsWith('error.tsx') || file.endsWith('error.jsx')
      const isNotFoundFile = file.endsWith('not-found.tsx') || file.endsWith('not-found.jsx')
      const isSpecialRouteFile = isPageFile || isLayoutFile || isLoadingFile || isErrorFile || isNotFoundFile

      if (isAppRouterFile && isSpecialRouteFile) {
        let fileType = 'page'
        if (isLayoutFile)
          fileType = 'layout'
        else if (isLoadingFile)
          fileType = 'loading'
        else if (isErrorFile)
          fileType = 'error'
        else if (isNotFoundFile)
          fileType = 'not-found'

        if (serverComponentBuilder && componentType === 'server') {
          try {
            await (serverComponentBuilder as any).rebuildComponent(file)
          }
          catch (error) {
            console.error(`[rari] HMR: Failed to rebuild ${file}:`, error)
          }
        }

        server.hot.send('rari:app-router-updated', {
          type: 'rari-hmr',
          filePath: file,
          fileType,
        })

        return undefined
      }

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
        const importRegex = /import\s+["']([^"']+)["']/g
        const imports: string[] = []

        for (const match of html.matchAll(importRegex)) {
          const importPath = match[1]
          if (importPath.startsWith('/src/'))
            imports.push(importPath)
        }

        if (imports.length > 0) {
          const tags = imports.map(importPath => ({
            tag: 'script',
            attrs: {
              type: 'module',
              src: importPath,
            },
            injectTo: 'head-prepend' as const,
          }))
          return { html, tags }
        }

        return html
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
    rateLimit: options.rateLimit,
    spamBlocker: options.spamBlocker,
  })

  const plugins: Plugin[] = [mainPlugin, serverBuildPlugin]

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

export { RariRequest } from '../proxy/RariRequest'
export { RariResponse } from '../proxy/RariResponse'
export type { ProxyConfig, ProxyFunction, RariFetchEvent, RariURL } from '../proxy/types'
export { rariProxy } from '../proxy/vite-plugin'
