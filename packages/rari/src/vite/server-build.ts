import type { Plugin } from 'rolldown-vite'
import type { ServerConfig, ServerCSPConfig, ServerRateLimitConfig, ServerSpamBlockerConfig } from '../types/server-config'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { build } from 'rolldown'

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
    }
  >
  importMap: {
    imports: Record<string, string>
  }
  version: string
  buildTime: string
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
  rateLimit?: ServerRateLimitConfig
  spamBlocker?: ServerSpamBlockerConfig
}

export interface ComponentRebuildResult {
  componentId: string
  bundlePath: string
  success: boolean
  error?: string
}

type ResolvedServerBuildOptions = Required<Omit<ServerBuildOptions, 'csp' | 'rateLimit' | 'spamBlocker' | 'define' | 'serverConfigPath'>> & {
  serverConfigPath: string
  csp?: ServerBuildOptions['csp']
  rateLimit?: ServerBuildOptions['rateLimit']
  spamBlocker?: ServerBuildOptions['spamBlocker']
  define?: ServerBuildOptions['define']
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
    timestamp: number
    dependencies: string[]
  }>()

  private htmlOnlyImports = new Set<string>()

  getComponentCount(): number {
    return this.serverComponents.size + this.serverActions.size
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
      rateLimit: options.rateLimit,
      spamBlocker: options.spamBlocker,
    }

    this.parseHtmlImports()
  }

  private parseHtmlImports() {
    const indexHtmlPath = path.join(this.projectRoot, 'index.html')
    if (!fs.existsSync(indexHtmlPath))
      return

    try {
      const htmlContent = fs.readFileSync(indexHtmlPath, 'utf-8')
      const importRegex = /import\s+["']([^"']+)["']/g
      for (const match of htmlContent.matchAll(importRegex)) {
        const importPath = match[1]
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

  isServerComponent(filePath: string): boolean {
    if (filePath.includes('node_modules'))
      return false

    if (this.isHtmlOnlyImport(filePath))
      return false

    try {
      if (!fs.existsSync(filePath))
        return false

      const code = fs.readFileSync(filePath, 'utf-8')

      const lines = code.split('\n')
      let hasClientDirective = false
      let hasServerDirective = false
      for (const line of lines) {
        const trimmed = line.trim()
        if (trimmed.startsWith('//') || trimmed.startsWith('/*') || !trimmed)
          continue
        if (trimmed === '\'use client\'' || trimmed === '"use client"'
          || trimmed === '\'use client\';' || trimmed === '"use client";') {
          hasClientDirective = true
          break
        }
        if (trimmed === '\'use server\'' || trimmed === '"use server"'
          || trimmed === '\'use server\';' || trimmed === '"use server";') {
          hasServerDirective = true
          break
        }
        if (trimmed)
          break
      }

      return !hasClientDirective && !hasServerDirective
    }
    catch {
      return false
    }
  }

  private isClientComponent(filePath: string): boolean {
    try {
      if (!fs.existsSync(filePath))
        return false
      const code = fs.readFileSync(filePath, 'utf-8')

      const clientDirectives = [
        '\'use client\'',
        '"use client"',
        '/* @client */',
        '// @client',
      ]

      const trimmedCode = code.trim()

      const hasClientDirective = clientDirectives.some(
        directive =>
          trimmedCode.startsWith(directive) || code.includes(directive),
      )

      return hasClientDirective
    }
    catch {
      return false
    }
  }

  addServerComponent(filePath: string) {
    const code = fs.readFileSync(filePath, 'utf-8')

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

    if (!this.isServerComponent(filePath))
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
    const lines = code.split('\n')
    for (const line of lines) {
      const trimmed = line.trim()
      if (trimmed.startsWith('//') || trimmed.startsWith('/*') || !trimmed)
        continue
      if (trimmed === '\'use server\'' || trimmed === '"use server"'
        || trimmed === '\'use server\';' || trimmed === '"use server";') {
        return true
      }

      if (trimmed)
        break
    }

    return false
  }

  private extractDependencies(code: string): string[] {
    const dependencies: string[] = []
    const importRegex
      = /import(?:\s+(?:\w+|\{[^}]*\}|\*\s+as\s+\w+)(?:\s*,\s*(?:\w+|\{[^}]*\}|\*\s+as\s+\w+))*\s+from\s+)?['"]([^'"]+)['"]/g
    let match

    while (true) {
      match = importRegex.exec(code)
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
    const nodeBuiltins = [
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
    ]
    return nodeBuiltins.includes(moduleName)
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
    const importRegex = /import\s+(\w+)\s+from\s+['"]([^'"]+)['"]/g
    const replacements: Array<{ original: string, replacement: string }> = []

    for (const match of code.matchAll(importRegex)) {
      const [fullMatch, importName, importPath] = match

      if (!importPath.startsWith('.') && !importPath.startsWith('@') && !importPath.startsWith('~') && !importPath.startsWith('#'))
        continue

      let resolvedPath: string | null = null

      if (importPath.startsWith('.')) {
        if (importPath.includes('/components/')) {
          const componentMatch = importPath.match(/\/components\/(\w+)(?:\.tsx?|\.jsx?)?$/)
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
  let Component = globalThis['~rsc']?.components?.['components/${componentName}']
    || globalThis['~rsc']?.modules?.['components/${componentName}']?.default
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
        const componentMatch = resolvedPath.match(/[/\\]components[/\\](\w+)(?:\.tsx?|\.jsx?)?$/)
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

          const relativeFromRoot = path.relative(this.projectRoot, actualPath)
          const componentId = relativeFromRoot
            .replace(/\\/g, '/')
            .replace(/\.(tsx?|\.jsx?)$/, '')
            .replace(/[^\w/-]/g, '_')
            .replace(/^src\//, '')

          const replacement = `// Component reference: ${componentName}
const ${importName} = (props) => {
  let Component = globalThis['~rsc']?.components?.['${componentId}']
    || globalThis['~rsc']?.modules?.['${componentId}']?.default
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

  private createBuildPlugins(virtualModuleId: string, transformedCode: string, loader: 'tsx' | 'jsx' | 'ts' | 'js', inputPath: string, isPage = false) {
    const resolveDir = path.dirname(inputPath)
    const isProxyFile = path.basename(inputPath).match(/^proxy\.(?:tsx?|jsx?|mts|mjs)$/)
    const self = this

    const clientComponentRefs = new Map<string, string>()
    const serverActionRefs = new Map<string, string>()

    return [
      {
        name: 'virtual-module',
        resolveId(id: string, importer: string | undefined) {
          if (id === virtualModuleId)
            return id

          if (importer === virtualModuleId && (id.startsWith('./') || id.startsWith('../'))) {
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
          if (!importer || importer.includes('node_modules') || importer.includes('/packages/rari/dist'))
            return null

          if (source.startsWith('node:') || self.isNodeBuiltin(source)
            || source === 'react' || source === 'react-dom'
            || source === 'react/jsx-runtime' || source === 'react/jsx-dev-runtime') {
            return null
          }

          let resolvedPath: string | null = null
          const aliases = self.options.alias || {}

          for (const [alias, replacement] of Object.entries(aliases)) {
            if (source.startsWith(`${alias}/`) || source === alias) {
              const relativePath = source.slice(alias.length)
              const newPath = path.join(replacement, relativePath)
              resolvedPath = path.isAbsolute(newPath) ? newPath : path.resolve(self.projectRoot, newPath)
              break
            }
          }

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
                  const componentId = relativePath.startsWith('..') ? pathWithExt : relativePath
                  clientComponentRefs.set(pathWithExt, componentId)
                  return { id: `\0client-ref:${pathWithExt}` }
                }

                try {
                  const content = fs.readFileSync(pathWithExt, 'utf-8')
                  const lines = content.split('\n')
                  for (const line of lines) {
                    const trimmed = line.trim()
                    if (trimmed.startsWith('//') || trimmed.startsWith('/*') || !trimmed)
                      continue
                    if (trimmed === '\'use server\'' || trimmed === '"use server"'
                      || trimmed === '\'use server\';' || trimmed === '"use server";') {
                      const relActionPath = path.relative(self.projectRoot, pathWithExt)
                      const actionId = relActionPath.startsWith('..') ? pathWithExt : relActionPath
                      serverActionRefs.set(pathWithExt, actionId)
                      return { id: `\0server-action:${pathWithExt}` }
                    }
                    if (trimmed)
                      break
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
            const componentId = clientComponentRefs.get(filePath) || (relativePath.startsWith('..') ? filePath : relativePath)

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
  try {
    if (typeof globalThis['~rari']?.bridge !== 'undefined' &&
        typeof globalThis['~rari'].bridge.registerClientReference === 'function') {
      globalThis['~rari'].bridge.registerClientReference(key, id, exportName);
    }
  } catch (error) {
    console.error('[rari] Build: Failed to register client reference:', error);
  }
  return clientProxy;
}

export default registerClientReference(null, ${JSON.stringify(componentId)}, "default");
`,
              moduleType: 'js',
            }
          }

          if (id.startsWith('\0server-action:')) {
            const filePath = id.slice('\0server-action:'.length)

            return {
              code: `export * from ${JSON.stringify(filePath)};`,
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
            const filePath = source.replace(/^file:\/\//, '')
            if (fs.existsSync(filePath))
              return { id: `\0transformed:${filePath}` }

            return null
          }

          let resolvedPath: string | null = null
          const aliases = self.options.alias || {}

          for (const [alias, replacement] of Object.entries(aliases)) {
            if (source.startsWith(`${alias}/`) || source === alias) {
              const relativePath = source.slice(alias.length)
              const newPath = path.join(replacement, relativePath)
              resolvedPath = path.isAbsolute(newPath) ? newPath : path.resolve(self.projectRoot, newPath)
              break
            }
          }

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

              const relativePath = path.relative(srcDir, pathWithExt)
              const distPath = path.join(self.options.outDir, self.options.rscDir, relativePath.replace(/\.(tsx?|jsx?)$/, '.js'))

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
            const rariResponsePath = path.resolve(self.projectRoot, 'node_modules/rari/dist/proxy/RariResponse.mjs')
            if (fs.existsSync(rariResponsePath))
              return rariResponsePath

            const rariResponseSrcPath = path.resolve(self.projectRoot, 'node_modules/rari/src/proxy/RariResponse.ts')
            if (fs.existsSync(rariResponseSrcPath))
              return rariResponseSrcPath
          }

          return null
        },
      },
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

          if (source === 'rari' || source === 'rari/client')
            return null

          if (!source.startsWith('.') && !source.startsWith('/'))
            return { id: source, external: true }

          return null
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
      plugins: this.createBuildPlugins(virtualModuleId, transformedCode, loader, inputPath, isPage),
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

  async buildServerComponents(): Promise<ServerComponentManifest> {
    const serverOutDir = path.join(this.options.outDir, this.options.rscDir)

    await fs.promises.mkdir(serverOutDir, { recursive: true })

    const importMapImports: Record<string, string> = {
      'react': 'npm:react@19',
      'react-dom': 'npm:react-dom@19',
      'react/jsx-runtime': 'npm:react@19/jsx-runtime',
      'react/jsx-dev-runtime': 'npm:react@19/jsx-dev-runtime',
    }

    const aliases = this.options.alias || {}
    for (const [alias, replacement] of Object.entries(aliases)) {
      const absolutePath = path.isAbsolute(replacement)
        ? replacement
        : path.resolve(this.projectRoot, replacement)
      importMapImports[`${alias}/`] = `file://${absolutePath}/`
    }

    const manifest: ServerComponentManifest = {
      components: {},
      importMap: {
        imports: importMapImports,
      },
      version: '1.0.0',
      buildTime: new Date().toISOString(),
    }

    for (const [filePath, component] of this.serverComponents) {
      if (this.isPageComponent(filePath))
        continue

      const relativePath = path.relative(this.projectRoot, filePath)
      const componentId = this.getComponentId(relativePath)
      const bundlePath = path.join(this.options.rscDir, `${componentId}.js`)
      const fullBundlePath = path.join(this.options.outDir, bundlePath)

      const bundleDir = path.dirname(fullBundlePath)
      await fs.promises.mkdir(bundleDir, { recursive: true })

      await this.buildSingleComponent(filePath, fullBundlePath)

      const moduleSpecifier = `file://${path.resolve(this.projectRoot, fullBundlePath)}`

      manifest.components[componentId] = {
        id: componentId,
        filePath,
        relativePath,
        bundlePath,
        moduleSpecifier,
        dependencies: component.dependencies,
        hasNodeImports: component.hasNodeImports,
      }
    }

    for (const [filePath, component] of this.serverComponents) {
      if (!this.isPageComponent(filePath))
        continue

      const relativePath = path.relative(this.projectRoot, filePath)
      const componentId = this.getComponentId(relativePath)
      const bundlePath = path.join(this.options.rscDir, `${componentId}.js`)
      const fullBundlePath = path.join(this.options.outDir, bundlePath)

      const bundleDir = path.dirname(fullBundlePath)
      await fs.promises.mkdir(bundleDir, { recursive: true })

      await this.buildSingleComponent(filePath, fullBundlePath)

      const moduleSpecifier = `file://${path.resolve(this.projectRoot, fullBundlePath)}`

      manifest.components[componentId] = {
        id: componentId,
        filePath,
        relativePath,
        bundlePath,
        moduleSpecifier,
        dependencies: component.dependencies,
        hasNodeImports: component.hasNodeImports,
      }
    }

    for (const [filePath, action] of this.serverActions) {
      const relativePath = path.relative(this.projectRoot, filePath)
      const actionId = this.getComponentId(relativePath)
      const bundlePath = path.join(this.options.rscDir, `${actionId}.js`)
      const fullBundlePath = path.join(this.options.outDir, bundlePath)

      const bundleDir = path.dirname(fullBundlePath)
      await fs.promises.mkdir(bundleDir, { recursive: true })

      await this.buildSingleComponent(filePath, fullBundlePath)

      const moduleSpecifier = `file://${path.resolve(this.projectRoot, fullBundlePath)}`

      manifest.components[actionId] = {
        id: actionId,
        filePath,
        relativePath,
        bundlePath,
        moduleSpecifier,
        dependencies: action.dependencies,
        hasNodeImports: action.hasNodeImports,
      }
    }

    const manifestPath = path.join(
      this.options.outDir,
      this.options.manifestPath,
    )
    await fs.promises.writeFile(
      manifestPath,
      JSON.stringify(manifest, null, 2),
      'utf-8',
    )

    const serverConfig: ServerConfig = {}
    if (this.options.csp)
      serverConfig.csp = this.options.csp
    if (this.options.rateLimit)
      serverConfig.rateLimit = this.options.rateLimit
    if (this.options.spamBlocker)
      serverConfig.spamBlocker = this.options.spamBlocker

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

  private async buildSingleComponent(
    inputPath: string,
    outputPath: string,
    returnCode = false,
  ): Promise<string | void> {
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
      plugins: this.createBuildPlugins(virtualModuleId, transformedCode, loader, inputPath, isPage),
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

    if (returnCode)
      return code
  }

  private createSelfRegisteringModule(
    code: string,
    componentId: string,
  ): string {
    if (code.includes('Self-registering Production Component'))
      return code

    let transformedCode = code

    let defaultExportName: string | null = null
    const namedExports: string[] = []

    transformedCode = transformedCode.replace(
      /^export\s+default\s+function\s+(\w+)/gm,
      (match, name) => {
        defaultExportName = name
        return `function ${name}`
      },
    )

    transformedCode = transformedCode.replace(
      /^export\s+default\s+async\s+function\s+(\w+)/gm,
      (match, name) => {
        defaultExportName = name
        return `async function ${name}`
      },
    )

    transformedCode = transformedCode.replace(
      /^export\s+default\s+(\w+);?\s*$/gm,
      (match, name) => {
        defaultExportName = name
        return `// Default export: ${name}`
      },
    )

    transformedCode = transformedCode.replace(
      /^export\s*\{\s*(\w+)\s+as\s+default\s*\};?\s*$/gm,
      (match, name) => {
        defaultExportName = name
        return `// Default export: ${name}`
      },
    )

    transformedCode = transformedCode.replace(
      /^export\s*\{([^}]+)\};?\s*$/gm,
      (match, exports) => {
        const exportList = exports.split(',').map((exp: string) => exp.trim())
        const exportNames: string[] = []
        exportList.forEach((exp: string) => {
          if (exp.includes('as default')) {
            const actualName = exp.replace('as default', '').trim()
            defaultExportName = actualName
            exportNames.push(`${actualName} (default)`)
          }
          else if (exp === 'default') {
            const possibleDefault = `${componentId}_default`
            if (transformedCode.includes(`var ${possibleDefault}`))
              defaultExportName = possibleDefault

            exportNames.push('default')
          }
          else {
            namedExports.push(exp)
            exportNames.push(exp)
          }
        })
        return `// Exports: ${exportNames.join(', ')}`
      },
    )

    transformedCode = transformedCode.replace(
      /^export\s+(?:async\s+)?function\s+(\w+)/gm,
      (match, name) => {
        namedExports.push(name)
        return match.replace('export ', '')
      },
    )

    transformedCode = transformedCode.replace(
      /^export\s+(const|let|var)\s+(\w+)/gm,
      (match, keyword, name) => {
        namedExports.push(name)
        return `${keyword} ${name}`
      },
    )

    if (!defaultExportName) {
      const possibleDefault = `${componentId}_default`
      if (transformedCode.includes(`var ${possibleDefault}`))
        defaultExportName = possibleDefault
    }

    const selfRegisteringCode = `// Self-registering Production Component: ${componentId}

if (!globalThis["${componentId}"]) {
    ${transformedCode}

        try {
            const moduleKey = "${componentId}";
            let mainExport = null;
            const exportedFunctions = {};

            if (!globalThis['~serverFunctions']) globalThis['~serverFunctions'] = {};
            if (!globalThis['~serverFunctions'].all) globalThis['~serverFunctions'].all = {};

            ${namedExports
              .map(
                name => `if (typeof ${name} !== 'undefined') {
                globalThis.${name} = ${name};
                globalThis['~serverFunctions'].all['${name}'] = ${name};
                exportedFunctions['${name}'] = ${name};
            }`,
              )
              .join('\n            ')}

            ${defaultExportName
              ? `if (typeof ${defaultExportName} !== 'undefined') {
                mainExport = ${defaultExportName};
            }`
              : ''}

            if (mainExport === null && Object.keys(exportedFunctions).length > 0) {
                if (Object.keys(exportedFunctions).length === 1) {
                    mainExport = exportedFunctions[Object.keys(exportedFunctions)[0]];
                } else {
                    let componentFunction = null;
                    let firstFunction = null;

                    for (const [name, value] of Object.entries(exportedFunctions)) {
                        if (typeof value === 'function') {
                            if (!firstFunction) firstFunction = value;
                            if (/^[A-Z]/.test(name)) {
                                componentFunction = value;
                                break;
                            }
                        }
                    }

                    mainExport = componentFunction || firstFunction;
                }
            }

            if (mainExport !== null) {
                if (!globalThis[moduleKey]) {
                    globalThis[moduleKey] = mainExport;
                }

                if (!globalThis['~rsc']) globalThis['~rsc'] = {};
                globalThis['~rsc'].components = globalThis['~rsc'].components || {};
                globalThis['~rsc'].components[moduleKey] = mainExport;

                globalThis['~rsc'].modules = globalThis['~rsc'].modules || {};
                globalThis['~rsc'].modules[moduleKey] = { default: mainExport, ...exportedFunctions };

                if (typeof globalThis.RscModuleManager !== 'undefined' && globalThis.RscModuleManager.register) {
                    globalThis.RscModuleManager.register(moduleKey, mainExport, exportedFunctions);
                }
            }
        } catch (error) {
            console.error('[rari] Build: Error in self-registration for ${componentId}:', error);
        }
}`

    return selfRegisteringCode
  }

  private transformClientImports(code: string, inputPath: string): string {
    let transformedCode = code

    const importRegex
      = /import\s+(?:(\w+)|\{([^}]+)\})\s+from\s+['"]([^'"]+)['"];?\s*$/gm
    let match

    const replacements: Array<{ original: string, replacement: string }> = []
    let hasClientComponents = false

    const externalClientComponents = ['rari/image']

    while (true) {
      match = importRegex.exec(code)
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
          componentId = path.relative(this.projectRoot, resolvedPath)
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
    if (typeof globalThis['~rari']?.bridge !== 'undefined' &&
        typeof globalThis['~rari'].bridge.registerClientReference === 'function') {
      globalThis['~rari'].bridge.registerClientReference(key, id, exportName);
    }
  } catch (error) {
    console.error('[rari] Build: Failed to register client reference with Rust bridge:', error);
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

  private getComponentId(relativePath: string): string {
    return relativePath
      .replace(/\\/g, '/')
      .replace(/\.(tsx?|jsx?)$/, '')
      .replace(/[^\w/-]/g, '_')
      .replace(/^src\//, '')
  }

  async rebuildComponent(filePath: string): Promise<ComponentRebuildResult> {
    const componentId = this.getComponentId(
      path.relative(this.projectRoot, filePath),
    )

    const code = await fs.promises.readFile(filePath, 'utf-8')
    const dependencies = this.extractDependencies(code)
    const hasNodeImports = this.hasNodeImports(code)

    const componentData = {
      filePath,
      originalCode: code,
      dependencies,
      hasNodeImports,
    }

    if (this.isServerAction(code))
      this.serverActions.set(filePath, componentData)
    else
      this.serverComponents.set(filePath, componentData)

    const relativeBundlePath = path.join(
      this.options.rscDir,
      `${componentId}.js`,
    )
    const fullBundlePath = path.join(this.options.outDir, relativeBundlePath)

    const cached = this.buildCache.get(filePath)
    const fileStats = await fs.promises.stat(filePath)
    const fileTimestamp = fileStats.mtimeMs

    if (cached
      && cached.timestamp >= fileTimestamp
      && JSON.stringify(cached.dependencies) === JSON.stringify(dependencies)) {
      await fs.promises.writeFile(fullBundlePath, cached.code, 'utf-8')

      await this.updateManifestForComponent(componentId, filePath, relativeBundlePath)

      return {
        componentId,
        bundlePath: path.join(this.options.outDir, relativeBundlePath),
        success: true,
      }
    }

    const bundleDir = path.dirname(fullBundlePath)
    await fs.promises.mkdir(bundleDir, { recursive: true })

    const builtCode = await this.buildSingleComponent(
      filePath,
      fullBundlePath,
      true,
    ) as string

    this.buildCache.set(filePath, {
      code: builtCode,
      timestamp: Date.now(),
      dependencies,
    })

    await this.updateManifestForComponent(componentId, filePath, relativeBundlePath)

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
        importMap: {
          imports: {
            'react': 'npm:react@19',
            'react-dom': 'npm:react-dom@19',
            'react/jsx-runtime': 'npm:react@19/jsx-runtime',
            'react/jsx-dev-runtime': 'npm:react@19/jsx-dev-runtime',
          },
        },
        version: '1.0.0',
        buildTime: new Date().toISOString(),
      }
      this.manifestCache = manifest
    }

    const componentData = this.serverComponents.get(filePath) || this.serverActions.get(filePath)
    const fullBundlePath = path.join(this.options.outDir, bundlePath)
    const moduleSpecifier = `file://${path.resolve(this.projectRoot, fullBundlePath)}`

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
      }
    }

    if (!manifest.importMap) {
      manifest.importMap = {
        imports: {
          'react': 'npm:react@19',
          'react-dom': 'npm:react-dom@19',
          'react/jsx-runtime': 'npm:react@19/jsx-runtime',
          'react/jsx-dev-runtime': 'npm:react@19/jsx-dev-runtime',
        },
      }
    }

    manifest.buildTime = new Date().toISOString()

    await fs.promises.writeFile(
      manifestPath,
      JSON.stringify(manifest, null, 2),
      'utf-8',
    )

    this.manifestCache = manifest
  }

  clearCache(): void {
    this.buildCache.clear()
    this.manifestCache = null
  }

  async getTransformedComponentCode(filePath: string): Promise<string> {
    return await this.buildComponentCodeOnly(filePath)
  }
}

export function scanDirectory(dir: string, builder: ServerComponentBuilder) {
  const entries = fs.readdirSync(dir, { withFileTypes: true })

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name)

    if (entry.isDirectory()) {
      scanDirectory(fullPath, builder)
    }
    else if (entry.isFile() && /\.(?:tsx?|jsx?)$/.test(entry.name)) {
      if (/^(?:robots|sitemap)\.(?:tsx?|jsx?)$/.test(entry.name))
        continue

      try {
        if (builder.isServerComponent(fullPath)) {
          builder.addServerComponent(fullPath)
        }
        else {
          const code = fs.readFileSync(fullPath, 'utf-8')
          const lines = code.split('\n')
          let hasServerDirective = false
          for (const line of lines) {
            const trimmed = line.trim()
            if (trimmed.startsWith('//') || trimmed.startsWith('/*') || !trimmed)
              continue
            if (trimmed === '\'use server\'' || trimmed === '"use server"'
              || trimmed === '\'use server\';' || trimmed === '"use server";') {
              hasServerDirective = true
              break
            }
            if (trimmed)
              break
          }
          if (hasServerDirective)
            builder.addServerComponent(fullPath)
        }
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

        try {
          const { generateRobotsFile } = await import('../router/robots-generator')
          await generateRobotsFile({
            appDir: path.join(projectRoot, 'src', 'app'),
            outDir: path.join(projectRoot, 'dist'),
            extensions: ['.ts', '.tsx', '.js', '.jsx'],
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
            extensions: ['.ts', '.tsx', '.js', '.jsx'],
          })
        }
        catch (error) {
          console.warn('[rari] Failed to generate sitemap:', error)
        }
      }
    },

    async handleHotUpdate({ file }) {
      if (!builder || !isDev)
        return

      const relativePath = path.relative(projectRoot, file).replace(/\\/g, '/')
      if (!relativePath.startsWith('src/') || !/\.(?:tsx?|jsx?)$/.test(relativePath))
        return

      try {
        const content = await fs.promises.readFile(file, 'utf-8')
        if (content.includes('use client'))
          return

        await builder.buildServerComponents()
      }
      catch (error) {
        console.error(`[rari] Build: Error rebuilding ${relativePath}:`, error)
      }
    },
  }
}
