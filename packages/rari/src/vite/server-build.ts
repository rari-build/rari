import type { Plugin } from 'rolldown-vite'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { build } from 'esbuild'

interface ServerComponentManifest {
  components: Record<
    string,
    {
      id: string
      filePath: string
      relativePath: string
      bundlePath: string
      dependencies: string[]
      hasNodeImports: boolean
    }
  >
  version: string
  buildTime: string
}

export interface ServerBuildOptions {
  outDir?: string
  serverDir?: string
  manifestPath?: string
  minify?: boolean
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

  private options: Required<ServerBuildOptions>
  private projectRoot: string

  getComponentCount(): number {
    return this.serverComponents.size
  }

  constructor(projectRoot: string, options: ServerBuildOptions = {}) {
    this.projectRoot = projectRoot
    this.options = {
      outDir: options.outDir || path.join(projectRoot, 'dist'),
      serverDir: options.serverDir || 'server',
      manifestPath: options.manifestPath || 'server-manifest.json',
      minify: options.minify ?? process.env.NODE_ENV === 'production',
    }
  }

  isServerComponent(filePath: string): boolean {
    if (filePath.includes('node_modules')) {
      return false
    }

    if (filePath.includes('/.rari/') || filePath.includes('\\.rari\\')) {
      return false
    }

    const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || ''
    if (fileName === 'entry-client.tsx' || fileName === 'entry-client.ts' ||
        fileName === 'entry-client.jsx' || fileName === 'entry-client.js' ||
        fileName === 'main.tsx' || fileName === 'main.ts') {
      return false
    }

    try {
      if (!fs.existsSync(filePath)) {
        return false
      }
      const code = fs.readFileSync(filePath, 'utf-8')

      const isInFunctionsDir
        = filePath.includes('/functions/') || filePath.includes('\\\\functions\\\\')

      if (isInFunctionsDir) {
        return false
      }

      const lines = code.split('\n')
      let hasClientDirective = false
      for (const line of lines) {
        const trimmed = line.trim()
        if (trimmed.startsWith('//') || trimmed.startsWith('/*') || !trimmed) {
          continue
        }
        if (trimmed === '\'use client\'' || trimmed === '"use client"' ||
            trimmed === '\'use client\';' || trimmed === '"use client";') {
          hasClientDirective = true
          break
        }
        if (trimmed) {
          break
        }
      }
      return !hasClientDirective
    }
    catch {
      return false
    }
  }

  private isClientComponent(filePath: string): boolean {
    try {
      if (!fs.existsSync(filePath)) {
        return false
      }
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
    if (!this.isServerComponent(filePath)) {
      return
    }

    const code = fs.readFileSync(filePath, 'utf-8')
    const dependencies = this.extractDependencies(code)
    const hasNodeImports = this.hasNodeImports(code)

    this.serverComponents.set(filePath, {
      filePath,
      originalCode: code,
      dependencies,
      hasNodeImports,
    })
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

    return Array.from(new Set(dependencies))
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

    for (const [filePath, component] of this.serverComponents) {
      const relativePath = path.relative(this.projectRoot, filePath)
      const componentId = this.getComponentId(relativePath)

      const transformedCode = await this.buildComponentCodeOnly(filePath, componentId, component)

      components.push({
        id: componentId,
        code: transformedCode,
      })
    }

    return components
  }

  private async buildComponentCodeOnly(
    inputPath: string,
    componentId: string,
    _component: { dependencies: string[], hasNodeImports: boolean },
  ): Promise<string> {
    const originalCode = await fs.promises.readFile(inputPath, 'utf-8')
    const clientTransformedCode = this.transformClientImports(
      originalCode,
      inputPath,
    )
    const transformedCode = this.transformNodeImports(clientTransformedCode)

    const ext = path.extname(inputPath)
    let loader: string
    if (ext === '.tsx') {
      loader = 'tsx'
    }
    else if (ext === '.ts') {
      loader = 'ts'
    }
    else if (ext === '.jsx') {
      loader = 'jsx'
    }
    else {
      loader = 'js'
    }

    try {
      const result = await build({
        stdin: {
          contents: transformedCode,
          resolveDir: path.dirname(inputPath),
          sourcefile: inputPath,
          loader: loader as any,
        },
        bundle: true,
        platform: 'neutral',
        target: 'es2022',
        format: 'esm',
        external: [],
        mainFields: ['module', 'main'],
        conditions: ['import', 'module', 'default'],
        define: {
          'global': 'globalThis',
          'process.env.NODE_ENV': '"production"',
        },
        loader: {
          '.ts': 'ts',
          '.tsx': 'tsx',
          '.js': 'js',
          '.jsx': 'jsx',
        },
        resolveExtensions: ['.ts', '.tsx', '.js', '.jsx'],
        minify: false,
        minifyIdentifiers: false,
        sourcemap: false,
        metafile: false,
        write: false,
        plugins: [
          {
            name: 'hmr-auto-external',
            setup(build) {
              build.onResolve({ filter: /^[^./]/ }, async (args) => {
                return { path: args.path, external: true }
              })
            },
          },
          {
            name: 'resolve-server-functions',
            setup(build) {
              build.onResolve(
                { filter: /^\.\.?\/.*functions/ },
                async (args) => {
                  const resolvedPath = path.resolve(
                    path.dirname(args.importer),
                    args.path,
                  )

                  const possibleExtensions = [
                    '.ts',
                    '.js',
                    '.tsx',
                    '.jsx',
                    '/index.ts',
                    '/index.js',
                  ]
                  for (const ext of possibleExtensions) {
                    const fullPath = resolvedPath + ext
                    if (fs.existsSync(fullPath)) {
                      return { path: fullPath }
                    }
                  }

                  return null
                },
              )
            },
          },
        ],
        banner: {
          js: `// Rari Server Component Bundle
// Generated at: ${new Date().toISOString()}
// Original file: ${path.relative(this.projectRoot, inputPath)}
`,
        },
      })

      if (result.outputFiles && result.outputFiles.length > 0) {
        const outputFile = result.outputFiles[0]

        const finalTransformedCode = this.createSelfRegisteringModule(
          outputFile.text,
          componentId,
        )

        return finalTransformedCode
      }

      if (result.errors.length > 0) {
        console.error('ESBuild errors:', result.errors)
        throw new Error(
          `ESBuild compilation failed with ${result.errors.length} errors`,
        )
      }

      throw new Error('No output generated from ESBuild')
    }
    catch (error) {
      console.error(`ESBuild failed for ${inputPath}:`, error)
      throw error
    }
  }

  async buildServerComponents(): Promise<ServerComponentManifest> {
    const serverOutDir = path.join(this.options.outDir, this.options.serverDir)

    await fs.promises.mkdir(serverOutDir, { recursive: true })

    const manifest: ServerComponentManifest = {
      components: {},
      version: '1.0.0',
      buildTime: new Date().toISOString(),
    }

    for (const [filePath, component] of this.serverComponents) {
      const relativePath = path.relative(this.projectRoot, filePath)
      const componentId = this.getComponentId(relativePath)
      const bundlePath = path.join(this.options.serverDir, `${componentId}.js`)
      const fullBundlePath = path.join(this.options.outDir, bundlePath)

      const bundleDir = path.dirname(fullBundlePath)
      await fs.promises.mkdir(bundleDir, { recursive: true })

      await this.buildSingleComponent(filePath, fullBundlePath, component)

      manifest.components[componentId] = {
        id: componentId,
        filePath,
        relativePath,
        bundlePath,
        dependencies: component.dependencies,
        hasNodeImports: component.hasNodeImports,
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

    return manifest
  }

  private async buildSingleComponent(
    inputPath: string,
    outputPath: string,
    _component: { dependencies: string[], hasNodeImports: boolean },
  ): Promise<void> {
    const componentId = this.getComponentId(
      path.relative(this.projectRoot, inputPath),
    )

    const originalCode = await fs.promises.readFile(inputPath, 'utf-8')
    const clientTransformedCode = this.transformClientImports(
      originalCode,
      inputPath,
    )
    const transformedCode = this.transformNodeImports(clientTransformedCode)

    const ext = path.extname(inputPath)
    let loader: string
    if (ext === '.tsx') {
      loader = 'tsx'
    }
    else if (ext === '.ts') {
      loader = 'ts'
    }
    else if (ext === '.jsx') {
      loader = 'jsx'
    }
    else {
      loader = 'js'
    }

    try {
      const result = await build({
        stdin: {
          contents: transformedCode,
          resolveDir: path.dirname(inputPath),
          sourcefile: inputPath,
          loader: loader as any,
        },
        bundle: true,
        platform: 'neutral',
        target: 'es2022',
        format: 'esm',
        outfile: outputPath,
        external: [],
        mainFields: ['module', 'main'],
        conditions: ['import', 'module', 'default'],
        define: {
          'global': 'globalThis',
          'process.env.NODE_ENV': '"production"',
        },
        loader: {
          '.ts': 'ts',
          '.tsx': 'tsx',
          '.js': 'js',
          '.jsx': 'jsx',
        },
        resolveExtensions: ['.ts', '.tsx', '.js', '.jsx'],
        minify: false,
        minifyIdentifiers: false,
        sourcemap: false,
        metafile: false,
        write: false,
        plugins: [
          {
            name: 'auto-external',
            setup(build) {
              build.onResolve({ filter: /^[^./]/ }, async (args) => {
                return { path: args.path, external: true }
              })
            },
          },
          {
            name: 'resolve-server-functions',
            setup(build) {
              build.onResolve(
                { filter: /^\.\.?\/.*functions/ },
                async (args) => {
                  const resolvedPath = path.resolve(
                    path.dirname(args.importer),
                    args.path,
                  )

                  const possibleExtensions = [
                    '.ts',
                    '.js',
                    '.tsx',
                    '.jsx',
                    '/index.ts',
                    '/index.js',
                  ]
                  for (const ext of possibleExtensions) {
                    const fullPath = resolvedPath + ext
                    if (fs.existsSync(fullPath)) {
                      return { path: fullPath }
                    }
                  }

                  return null
                },
              )
            },
          },
        ],
        banner: {
          js: `// Rari Server Component Bundle
// Generated at: ${new Date().toISOString()}
// Original file: ${path.relative(this.projectRoot, inputPath)}
`,
        },
      })

      if (result.outputFiles && result.outputFiles.length > 0) {
        const outputFile = result.outputFiles[0]

        const finalTransformedCode = this.createSelfRegisteringModule(
          outputFile.text,
          componentId,
        )

        await fs.promises.writeFile(outputPath, finalTransformedCode, 'utf-8')
      }

      if (result.errors.length > 0) {
        console.error('ESBuild errors:', result.errors)
        throw new Error(
          `ESBuild compilation failed with ${result.errors.length} errors`,
        )
      }

      if (result.warnings.length > 0) {
        console.warn('ESBuild warnings:', result.warnings)
      }
    }
    catch (error) {
      console.error(`ESBuild failed for ${inputPath}:`, error)
      throw error
    }
  }

  private createSelfRegisteringModule(
    code: string,
    componentId: string,
  ): string {
    if (code.includes('Self-registering Production Component')) {
      return code
    }

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
        exportList.forEach((exp: string) => {
          if (exp.includes('as default')) {
            const actualName = exp.replace('as default', '').trim()
            defaultExportName = actualName
          }
          else if (exp === 'default') {
            const possibleDefault = `${componentId}_default`
            if (transformedCode.includes(`var ${possibleDefault}`)) {
              defaultExportName = possibleDefault
            }
          }
          else {
            namedExports.push(exp)
          }
        })
        return `// Exports: ${exports}`
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
      if (transformedCode.includes(`var ${possibleDefault}`)) {
        defaultExportName = possibleDefault
      }
    }

    const mainExport = defaultExportName || componentId

    const selfRegisteringCode = `// Self-registering Production Component: ${componentId}
"use module";

// Original component code with exports removed for self-registration
${transformedCode}

// Self-registration logic
(function() {
    try {
        const moduleKey = "${componentId}";
        const registrationKey = "Component_${Math.random().toString(36).substr(2, 8)}";
        let mainExport = null;
        let exportedFunctions = {};

        globalThis.__rsc_functions = globalThis.__rsc_functions || {};

        // Register named exports
        ${namedExports
          .map(
            name => `
        if (typeof ${name} !== 'undefined') {
            globalThis.${name} = ${name};
            globalThis.__rsc_functions['${name}'] = ${name};
            exportedFunctions['${name}'] = ${name};
        }`,
          )
          .join('')}

        // Set main export
        if (typeof ${mainExport} !== 'undefined') {
            mainExport = ${mainExport};
        } else {
            const potentialExports = {};
            ${namedExports.map(name => `if (typeof ${name} !== 'undefined') potentialExports.${name} = ${name};`).join('\n            ')}

            if (Object.keys(potentialExports).length > 0) {
                if (Object.keys(potentialExports).length === 1) {
                    mainExport = potentialExports[Object.keys(potentialExports)[0]];
                } else {
                    mainExport = potentialExports;
                }
            }
        }

        if (mainExport !== null) {
            if (!globalThis[moduleKey]) {
                globalThis[moduleKey] = mainExport;
            }

            if (!globalThis[registrationKey]) {
                globalThis[registrationKey] = mainExport;
            }

            if (typeof globalThis.RscModuleManager !== 'undefined' && globalThis.RscModuleManager.register) {
                globalThis.RscModuleManager.register(moduleKey, mainExport, exportedFunctions);
            }
        }
    } catch (error) {
        console.error('Error in self-registration for ${componentId}:', error);
    }
})();`

    return selfRegisteringCode
  }

  private transformClientImports(code: string, inputPath: string): string {
    let transformedCode = code

    const importRegex
      = /import\s+(\w+)(?:\s*,\s*\{[^}]*\})?\s+from\s+['"]([./][^'"]+)['"];?\s*$/gm
    let match

    const replacements: Array<{ original: string, replacement: string }> = []
    let hasClientComponents = false

    while (true) {
      match = importRegex.exec(code)
      if (match === null)
        break

      const [fullMatch, defaultImport, importPath] = match

      const resolvedPath = this.resolveImportPath(importPath, inputPath)

      if (this.isClientComponent(resolvedPath)) {
        hasClientComponents = true
        const componentName = defaultImport || 'default'

        const replacement = `const ${componentName} = registerClientReference(
  null,
  ${JSON.stringify(path.relative(this.projectRoot, resolvedPath))},
  "default"
);`

        replacements.push({ original: fullMatch, replacement })
      }
    }

    if (hasClientComponents) {
      const functionDefinition = `
// registerClientReference function for client components
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
    if (typeof globalThis.__rari_bridge !== 'undefined' &&
        typeof globalThis.__rari_bridge.registerClientReference === 'function') {
      globalThis.__rari_bridge.registerClientReference(key, id, exportName);
    }
  } catch (error) {
    console.error('Failed to register client reference with Rust bridge:', error);
  }

  return clientProxy;
}

`
      transformedCode = functionDefinition + transformedCode
    }

    for (const { original, replacement } of replacements) {
      transformedCode = transformedCode.replace(original, replacement)
    }

    return transformedCode
  }

  private resolveImportPath(importPath: string, importerPath: string): string {
    const resolvedPath = path.resolve(path.dirname(importerPath), importPath)

    const extensions = ['.tsx', '.jsx', '.ts', '.js']
    for (const ext of extensions) {
      const pathWithExt = `${resolvedPath}${ext}`
      if (fs.existsSync(pathWithExt)) {
        return pathWithExt
      }
    }

    if (fs.existsSync(resolvedPath)) {
      for (const ext of extensions) {
        const indexPath = path.join(resolvedPath, `index${ext}`)
        if (fs.existsSync(indexPath)) {
          return indexPath
        }
      }
    }

    return `${resolvedPath}.tsx`
  }

  private transformNodeImports(code: string): string {
    let transformedCode = code

    transformedCode = transformedCode.replace(
      /import\s+(\w+)\s+from\s+['"]node:process['"];?\s*$/gm,
      (match, importName) => {
        return `const ${importName} = globalThis.process;`
      },
    )

    transformedCode = transformedCode.replace(
      /import\s+\{([^}]+)\}\s+from\s+['"]node:fs['"];?\s*$/gm,
      (match, imports) => {
        const importList = imports.split(',').map((imp: string) => imp.trim())
        return importList
          .map((imp: string) => {
            const cleanImp = imp.replace(/\s+as\s+\w+/, '')
            if (cleanImp === 'existsSync') {
              return `const ${cleanImp} = (path) => { try { if (globalThis.Deno?.statSync) { globalThis.Deno.statSync(path); return true; } return false; } catch (error) { return false; } };`
            }
            if (cleanImp === 'readFileSync') {
              return `const ${cleanImp} = (path, encoding = 'utf8') => globalThis.Deno.readTextFileSync(path);`
            }
            return `const ${cleanImp} = globalThis.Deno?.${cleanImp} || (() => { throw new Error('${cleanImp} not available'); });`
          })
          .join('\n')
      },
    )

    transformedCode = transformedCode.replace(
      /import\s+\{([^}]+)\}\s+from\s+['"]node:path['"];?\s*$/gm,
      (match, imports) => {
        const importList = imports.split(',').map((imp: string) => imp.trim())
        return importList
          .map((imp: string) => {
            const cleanImp = imp.replace(/\s+as\s+\w+/, '')
            if (cleanImp === 'join') {
              return `const ${cleanImp} = (...paths) => paths.filter(Boolean).join('/').replace(/\\/+/g, '/');`
            }
            return `const ${cleanImp} = globalThis.path?.${cleanImp} || (() => { throw new Error('${cleanImp} not available'); });`
          })
          .join('\n')
      },
    )

    transformedCode = transformedCode.replace(
      /import\s+\{([^}]+)\}\s+from\s+['"]node:process['"];?\s*$/gm,
      (match, imports) => {
        const importList = imports.split(',').map((imp: string) => imp.trim())
        return importList
          .map((imp: string) => {
            const cleanImp = imp.replace(/\s+as\s+\w+/, '')
            if (cleanImp === 'cwd') {
              return `const ${cleanImp} = () => globalThis.Deno?.cwd?.() || '.';`
            }
            return `const ${cleanImp} = globalThis.process?.${cleanImp} || (() => { throw new Error('${cleanImp} not available'); });`
          })
          .join('\n')
      },
    )

    return transformedCode
  }

  private getComponentId(relativePath: string): string {
    return relativePath
      .replace(/\\/g, '/')
      .replace(/\.(tsx?|jsx?)$/, '')
      .replace(/[^\w/-]/g, '_')
      .replace(/^src\//, '')
      .replace(/^components\//, '')
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
      try {
        if (builder.isServerComponent(fullPath)) {
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

  return {
    name: 'rari-server-build',

    configResolved(config) {
      projectRoot = config.root
      builder = new ServerComponentBuilder(projectRoot, options)
    },

    buildStart() {
      if (!builder)
        return

      const srcDir = path.join(projectRoot, 'src')
      if (fs.existsSync(srcDir)) {
        scanDirectory(srcDir, builder)
      }
    },

    async closeBundle() {
      if (builder) {
        await builder.buildServerComponents()
      }
    },
  }
}
