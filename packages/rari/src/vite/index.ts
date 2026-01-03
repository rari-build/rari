import type { Plugin, UserConfig } from 'rolldown-vite'
import type { ServerBuildOptions } from './server-build'
import { Buffer } from 'node:buffer'
import { spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import * as acorn from 'acorn'
import { HMRCoordinator } from './hmr-coordinator'
import { createLoadingComponentPlugin } from './loading-component-bundler'
import { createServerBuildPlugin } from './server-build'

interface RariOptions {
  projectRoot?: string
  serverBuild?: ServerBuildOptions
  serverHandler?: boolean
}

async function loadRuntimeFile(filename: string): Promise<string> {
  const currentFileUrl = import.meta.url
  const currentFilePath = fileURLToPath(currentFileUrl)
  const currentDir = path.dirname(currentFilePath)

  const possiblePaths = [
    path.join(currentDir, '../runtime', filename),
    path.join(currentDir, '../src/runtime', filename),
  ]

  for (const filePath of possiblePaths) {
    try {
      return await fs.promises.readFile(filePath, 'utf-8')
    }
    catch {
      // Try next path
    }
  }

  throw new Error(`Could not find ${filename}. Tried: ${possiblePaths.join(', ')}`)
}

async function loadRscClientRuntime(): Promise<string> {
  return loadRuntimeFile('rsc-client-runtime.js')
}

async function loadEntryClient(imports: string, registrations: string): Promise<string> {
  const template = await loadRuntimeFile('entry-client.js')
  return template
    .replace('// CLIENT_COMPONENT_IMPORTS_PLACEHOLDER', imports)
    .replace('// CLIENT_COMPONENT_REGISTRATIONS_PLACEHOLDER', registrations)
}

async function loadReactServerDomShim(): Promise<string> {
  return loadRuntimeFile('react-server-dom-shim.js')
}

function scanForClientComponents(srcDir: string): Set<string> {
  const clientComponents = new Set<string>()

  function scanDirectory(dir: string) {
    if (!fs.existsSync(dir))
      return

    const entries = fs.readdirSync(dir, { withFileTypes: true })

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name)

      if (entry.isDirectory()) {
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
        catch {
          // Skip files that can't be read
        }
      }
    }
  }

  scanDirectory(srcDir)
  return clientComponents
}

export function defineRariOptions(config: RariOptions): RariOptions {
  return config
}

export function rari(options: RariOptions = {}): Plugin[] {
  const componentTypeCache = new Map<string, 'client' | 'server' | 'unknown'>()
  const serverComponents = new Set<string>()
  const clientComponents = new Set<string>()
  let rustServerProcess: any = null

  const serverImportedClientComponents = new Set<string>()

  let hmrCoordinator: HMRCoordinator | null = null
  const resolvedAlias: Record<string, string> = {}

  function isServerComponent(filePath: string): boolean {
    if (filePath.includes('node_modules')) {
      return false
    }

    if (filePath.includes('/rari/dist/') || filePath.includes('\\rari\\dist\\')) {
      return false
    }

    let pathForFsOperations = filePath
    try {
      pathForFsOperations = fs.realpathSync(filePath)
    }
    catch {
      return false
    }

    try {
      if (!fs.existsSync(pathForFsOperations)) {
        return false
      }
      const code = fs.readFileSync(pathForFsOperations, 'utf-8')

      const hasClientDirective = hasTopLevelDirective(code, 'use client')
      const hasServerDirective = hasTopLevelDirective(code, 'use server')

      if (hasServerDirective) {
        return false
      }

      return !hasClientDirective
    }
    catch {
      return false
    }
  }

  function parseExportedNames(code: string): string[] {
    try {
      const ast = acorn.parse(code, {
        ecmaVersion: 2024,
        sourceType: 'module',
      }) as any

      const exportedNames: string[] = []

      for (const node of ast.body) {
        switch (node.type) {
          case 'ExportDefaultDeclaration':
            exportedNames.push('default')
            break
          case 'ExportNamedDeclaration':
            if (node.declaration) {
              if (node.declaration.type === 'VariableDeclaration') {
                for (const declarator of node.declaration.declarations) {
                  if (declarator.id.type === 'Identifier') {
                    exportedNames.push(declarator.id.name)
                  }
                }
              }
              else if (node.declaration.id) {
                exportedNames.push(node.declaration.id.name)
              }
            }
            if (node.specifiers) {
              for (const specifier of node.specifiers) {
                exportedNames.push(specifier.exported.name)
              }
            }
            break
          case 'ExportAllDeclaration':
            if (node.exported && node.exported.type === 'Identifier') {
              exportedNames.push(node.exported.name)
            }
            break
        }
      }

      return exportedNames
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
      const ast = acorn.parse(code, {
        ecmaVersion: 2024,
        sourceType: 'module',
        locations: false,
      }) as any

      for (const node of ast.body) {
        if (node.type !== 'ExpressionStatement' || !node.directive)
          break
        if (node.directive === directive)
          return true
      }
      return false
    }
    catch {
      return false
    }
  }

  function transformServerModule(code: string, id: string): string {
    const hasUseServer = hasTopLevelDirective(code, 'use server')

    if (!hasUseServer) {
      return code
    }

    const exportedNames = parseExportedNames(code)
    if (exportedNames.length === 0) {
      return code
    }

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
      if (exportedNames.length === 0) {
        return ''
      }

      const relativePath = path.relative(process.cwd(), id)
      const moduleId = relativePath
        .replace(/\\/g, '/')
        .replace(/\.(tsx?|jsx?)$/, '')
        .replace(/[^\w/-]/g, '_')
        .replace(/^src\//, '')

      let newCode = 'import { createServerReference } from "rari/runtime/actions";\n'

      for (const name of exportedNames) {
        if (name === 'default') {
          newCode += `export default createServerReference("default", ${JSON.stringify(moduleId)}, "default");\n`
        }
        else {
          newCode += `export const ${name} = createServerReference("${name}", ${JSON.stringify(moduleId)}, "${name}");\n`
        }
      }

      return newCode
    }

    if (isServerComp) {
      const exportedNames = parseExportedNames(code)
      if (exportedNames.length === 0) {
        return ''
      }

      const relativePath = path.relative(process.cwd(), id)
      const componentId = relativePath
        .replace(/\\/g, '/')
        .replace(/\.(tsx?|jsx?)$/, '')
        .replace(/[^\w/-]/g, '_')
        .replace(/^src\//, '')
        .replace(/^components\//, '')

      let newCode
        = 'import { createServerComponentWrapper } from "virtual:rsc-integration";\n'

      for (const name of exportedNames) {
        if (name === 'default') {
          newCode += `export default createServerComponentWrapper("${componentId}", ${JSON.stringify(id)});\n`
        }
        else {
          newCode += `export const ${name} = createServerComponentWrapper("${componentId}_${name}", ${JSON.stringify(id)});\n`
        }
      }

      return newCode
    }

    if (!hasTopLevelDirective(code, 'use client')) {
      return code
    }

    const exportedNames = parseExportedNames(code)
    if (exportedNames.length === 0) {
      return ''
    }

    let newCode
      = 'import {registerClientReference} from "react-server-dom-rari/server";\n'

    for (const name of exportedNames) {
      if (name === 'default') {
        newCode += 'export default '
        newCode += 'registerClientReference(function() {'
        newCode += `throw new Error(${JSON.stringify(`Attempted to call the default export of ${id} from the server but it's on the client. It's not possible to invoke a client function from the server, it can only be rendered as a Component or passed to props of a Client Component.`)});`
      }
      else {
        newCode += `export const ${name} = `
        newCode += 'registerClientReference(function() {'
        newCode += `throw new Error(${JSON.stringify(`Attempted to call ${name}() from the server but ${name} is on the client. It's not possible to invoke a client function from the server, it can only be rendered as a Component or passed to props of a Client Component.`)});`
      }
      newCode += '},'
      newCode += `${JSON.stringify(id)},`
      newCode += `${JSON.stringify(name)});\n`
    }

    return newCode
  }

  function transformClientModuleForClient(code: string, _id: string): string {
    if (!hasTopLevelDirective(code, 'use client')) {
      return code
    }

    const exportedNames = parseExportedNames(code)
    if (exportedNames.length === 0) {
      return code
    }

    return code.replace(/^['"]use client['"];?\s*$/gm, '')
  }

  function resolveImportToFilePath(
    importPath: string,
    importerPath: string,
  ): string {
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

  function getComponentName(importPath: string): string {
    const lastSegment = importPath.split('/').pop() || importPath
    return lastSegment.replace(/\.[^.]*$/, '')
  }

  let serverComponentBuilder: any = null

  const mainPlugin: Plugin = {
    name: 'rari',

    config(config: UserConfig, { command }) {
      // Suppress the esbuildOptions deprecation warning from @vitejs/plugin-react
      // The React plugin internally sets optimizeDeps.esbuildOptions, but Vite 7+ uses Rolldown
      // This warning is expected and can be safely ignored until the React plugin is updated for Vite 7
      const originalWarn = console.warn
      console.warn = (...args: any[]) => {
        const message = args[0]
        if (typeof message === 'string' && message.includes('optimizeDeps.esbuildOptions') && message.includes('deprecated')) {
          return
        }
        originalWarn.apply(console, args)
      }
      config.resolve = config.resolve || {}
      const existingDedupe = Array.isArray((config.resolve as any).dedupe)
        ? ((config.resolve as any).dedupe as string[])
        : []
      const toAdd = ['react', 'react-dom'];
      (config.resolve as any).dedupe = Array.from(
        new Set([...(existingDedupe || []), ...toAdd]),
      )

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
        const reactPath = require.resolve('react')
        const reactDomClientPath = require.resolve('react-dom/client')
        const reactJsxRuntimePath = require.resolve('react/jsx-runtime')
        const aliasesToAppend: Array<{ find: string, replacement: string }>
          = []
        if (!aliasFinds.has('react/jsx-runtime')) {
          aliasesToAppend.push({
            find: 'react/jsx-runtime',
            replacement: reactJsxRuntimePath,
          })
        }
        try {
          const reactJsxDevRuntimePath = require.resolve(
            'react/jsx-dev-runtime',
          )
          if (!aliasFinds.has('react/jsx-dev-runtime')) {
            aliasesToAppend.push({
              find: 'react/jsx-dev-runtime',
              replacement: reactJsxDevRuntimePath,
            })
          }
        }
        catch { }
        if (!aliasFinds.has('react')) {
          aliasesToAppend.push({ find: 'react', replacement: reactPath })
        }
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
      catch { }

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
      if (!config.optimizeDeps.include.includes('react-dom/server')) {
        config.optimizeDeps.include.push('react-dom/server')
      }

      if (command === 'build') {
        for (const envName of ['rsc', 'ssr', 'client']) {
          const env = config.environments[envName]
          if (env && env.build) {
            env.build.rolldownOptions = env.build.rolldownOptions || {}
          }
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
        rewrite: path => path.replace(/^\/api/, '/api'),
        ws: false,
      }

      config.server.proxy['/rsc'] = {
        target: `http://localhost:${serverPort}`,
        changeOrigin: true,
        secure: false,
        ws: false,
      }

      config.server.proxy['/_rsc_status'] = {
        target: `http://localhost:${serverPort}`,
        changeOrigin: true,
        secure: false,
        ws: false,
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
        if (!config.environments.client.build) {
          config.environments.client.build = {}
        }
        if (!config.environments.client.build.rolldownOptions) {
          config.environments.client.build.rolldownOptions = {}
        }
        if (!config.environments.client.build.rolldownOptions.input) {
          config.environments.client.build.rolldownOptions.input = {}
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
            if (typeof entry.find === 'string' && typeof entry.replacement === 'string' && !excludeAliases.has(entry.find)) {
              resolvedAlias[entry.find] = entry.replacement
            }
          })
        }
        else if (typeof aliasConfig === 'object') {
          Object.entries(aliasConfig).forEach(([key, value]) => {
            if (typeof value === 'string' && !excludeAliases.has(key)) {
              resolvedAlias[key] = value
            }
          })
        }
      }
    },

    transform(code, id) {
      if (!/\.(?:tsx?|jsx?)$/.test(id)) {
        return null
      }

      const environment = (this as any).environment

      if (hasTopLevelDirective(code, 'use client')) {
        componentTypeCache.set(id, 'client')
        clientComponents.add(id)

        return transformClientModuleForClient(code, id)
      }

      if (isServerComponent(id)) {
        componentTypeCache.set(id, 'server')
        serverComponents.add(id)

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
      if (cachedType === 'server') {
        return transformServerModule(code, id)
      }

      if (cachedType === 'client') {
        return transformClientModuleForClient(code, id)
      }

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
          const importingFileIsClient = hasTopLevelDirective(code, 'use client')
            || componentTypeCache.get(id) === 'client'
            || id.includes('entry-client')

          if (!importingFileIsClient) {
            serverImportedClientComponents.add(resolvedImportPath)

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
        else if (isServerComponent(resolvedImportPath)) {
          hasServerImports = true
          needsReactImport = true
          needsWrapperImport = true
          serverComponents.add(resolvedImportPath)

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

        if (needsReactImport && !hasReactImport) {
          importsToAdd += `import React from 'react';\n`
        }

        if (needsWrapperImport && !hasWrapperImport) {
          importsToAdd += `import { createServerComponentWrapper } from 'virtual:rsc-integration';\n`
        }

        if (serverComponentReplacements.length > 0) {
          importsToAdd += `${serverComponentReplacements.join('\n')}\n`
        }

        if (importsToAdd) {
          modifiedCode = importsToAdd + modifiedCode
        }

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

    configureServer(server) {
      const projectRoot = options.projectRoot || process.cwd()
      const srcDir = path.join(projectRoot, 'src')

      const discoverAndRegisterComponents = async () => {
        try {
          const { ServerComponentBuilder, scanDirectory } = await import(
            './server-build',
          )

          const builder = new ServerComponentBuilder(projectRoot, {
            outDir: 'dist',
            serverDir: 'server',
            manifestPath: 'server-manifest.json',
            alias: resolvedAlias,
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
                    if (isServerComponent(fullPath)) {
                      serverComponentPaths.push(fullPath)
                    }
                  }
                  catch (error) {
                    console.error(`[RARI] Error checking ${fullPath}:`, error)
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
              if (isAppRouterComponent) {
                continue
              }

              if (component.code.includes('"use server"') || component.code.includes('\'use server\'')) {
                continue
              }

              const registerResponse = await fetch(
                `${baseUrl}/api/rsc/register`,
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
                `Failed to register component ${component.id}:`,
                error instanceof Error ? error.message : String(error),
              )
            }
          }
        }
        catch (error) {
          console.error(
            'Component discovery failed:',
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

          const clientComponentFiles = scanForClientComponents(srcDir)

          for (const componentPath of clientComponentFiles) {
            const relativePath = path.relative(process.cwd(), componentPath)
            const componentName = path
              .basename(componentPath)
              .replace(/\.[^.]+$/, '')

            try {
              await fetch(`${baseUrl}/api/rsc/register-client`, {
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
                `Failed to pre-register client component ${componentName}:`,
                error,
              )
            }
          }
        }
        catch (error) {
          console.error('Failed to pre-register client components:', error)
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
          console.error('Rari binary not found')
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
          if (output) {
            console.error(`${output}`)
          }
        })

        rustServerProcess.stderr?.on('data', (data: Buffer) => {
          const output = data.toString().trim()
          if (output && !output.includes('warning')) {
            console.error(`${output}`)
          }
        })

        rustServerProcess.on('error', (error: Error) => {
          console.error('Failed to start Rari server:', error.message)
          if (error.message.includes('ENOENT')) {
            console.error(
              '   Binary not found. Please ensure Rari is properly installed.',
            )
          }
        })

        rustServerProcess.on('exit', (code: number, signal: string) => {
          rustServerProcess = null
          if (signal) {
            console.error(`Rari server stopped by signal ${signal}`)
          }
          else if (code === 0) {
            console.error('Rari server stopped successfully')
          }
          else if (code) {
            console.error(`Rari server exited with code ${code}`)
          }
        })

        setTimeout(async () => {
          try {
            const serverPort = process.env.SERVER_PORT
              ? Number(process.env.SERVER_PORT)
              : Number(process.env.PORT || process.env.RSC_PORT || 3000)
            const baseUrl = `http://localhost:${serverPort}`

            let serverReady = false
            for (let i = 0; i < 10; i++) {
              try {
                const healthResponse = await fetch(`${baseUrl}/api/rsc/health`)
                if (healthResponse.ok) {
                  serverReady = true
                  break
                }
              }
              catch {
                await new Promise(resolve => setTimeout(resolve, 500))
              }
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
          catch (error) {
            console.error('Failed during component registration:', error)
          }
        }, 1000)
      }

      const handleServerComponentHMR = async (filePath: string) => {
        try {
          if (!isServerComponent(filePath)) {
            return
          }

          const { ServerComponentBuilder } = await import('./server-build')
          const builder = new ServerComponentBuilder(projectRoot, {
            outDir: 'dist',
            serverDir: 'server',
            manifestPath: 'server-manifest.json',
            alias: resolvedAlias,
          })

          builder.addServerComponent(filePath)

          const components
            = await builder.getTransformedComponentsForDevelopment()

          if (components.length === 0) {
            return
          }

          const serverPort = process.env.SERVER_PORT
            ? Number(process.env.SERVER_PORT)
            : Number(process.env.PORT || process.env.RSC_PORT || 3000)
          const baseUrl = `http://localhost:${serverPort}`

          for (const component of components) {
            try {
              const registerResponse = await fetch(
                `${baseUrl}/api/rsc/register`,
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
                '[RARI HMR] Failed to register component',
                `${component.id}:`,
                error instanceof Error ? error.message : String(error),
              )
            }
          }
        }
        catch (error) {
          console.error(
            '[RARI HMR] Targeted HMR failed for',
            `${filePath}:`,
            error instanceof Error ? error.message : String(error),
          )
          setTimeout(discoverAndRegisterComponents, 1000)
        }
      }

      startRustServer()

      server.middlewares.use(async (req, res, next) => {
        const acceptHeader = req.headers.accept
        const isRscRequest = acceptHeader && acceptHeader.includes('text/x-component')

        if (isRscRequest && req.url && !req.url.startsWith('/api') && !req.url.startsWith('/rsc') && !req.url.includes('.')) {
          const serverPort = process.env.SERVER_PORT
            ? Number(process.env.SERVER_PORT)
            : Number(process.env.PORT || process.env.RSC_PORT || 3000)

          const targetUrl = `http://localhost:${serverPort}${req.url}`

          try {
            const headers: Record<string, string> = {}
            for (const [key, value] of Object.entries(req.headers)) {
              if (value && typeof value === 'string') {
                headers[key] = value
              }
            }
            headers.host = `localhost:${serverPort}`
            headers['accept-encoding'] = 'identity'

            const response = await fetch(targetUrl, {
              method: req.method,
              headers,
            })

            res.statusCode = response.status
            response.headers.forEach((value, key) => {
              if (key.toLowerCase() !== 'content-encoding') {
                res.setHeader(key, value)
              }
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
                console.error('[Rari] Stream error:', streamError)
                if (!res.headersSent) {
                  res.statusCode = 500
                }
                res.end()
              }
            }
            else {
              res.end()
            }
            return
          }
          catch (error) {
            console.error('[Rari] Failed to proxy RSC request:', error)
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
        if (/\.(?:tsx?|jsx?)$/.test(filePath)) {
          componentTypeCache.delete(filePath)
        }

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
      })
    },

    resolveId(id) {
      if (id === 'virtual:rsc-integration') {
        return id
      }

      if (id === 'virtual:rari-entry-client') {
        return id
      }

      if (id === 'virtual:react-server-dom-rari-client') {
        return id
      }

      if (id === 'virtual:app-router-provider') {
        return `${id}.tsx`
      }

      if (id === './DefaultLoadingIndicator' || id === './DefaultLoadingIndicator.tsx') {
        return 'virtual:default-loading-indicator.tsx'
      }

      if (id === './LoadingErrorBoundary' || id === './LoadingErrorBoundary.tsx') {
        return 'virtual:loading-error-boundary.tsx'
      }

      if (id === '../router/LoadingComponentRegistry' || id === '../router/LoadingComponentRegistry.ts') {
        return 'virtual:loading-component-registry.ts'
      }

      if (id === 'react-server-dom-rari/server') {
        return id
      }

      if (process.env.NODE_ENV === 'production') {
        try {
          const resolvedPath = path.resolve(id)
          if (fs.existsSync(resolvedPath) && isServerComponent(resolvedPath)) {
            return { id, external: true }
          }
        }
        catch { }
      }

      return null
    },

    async load(id) {
      if (id === 'virtual:rari-entry-client') {
        const srcDir = path.join(process.cwd(), 'src')
        const scannedClientComponents = scanForClientComponents(srcDir)

        const allClientComponents = new Set([
          ...clientComponents,
          ...scannedClientComponents,
        ])

        const clientComponentsArray = Array.from(allClientComponents).filter((componentPath) => {
          try {
            const code = fs.readFileSync(componentPath, 'utf-8')
            const lines = code.split('\n')
            for (const line of lines) {
              const trimmed = line.trim()
              if (!trimmed || trimmed.startsWith('//') || trimmed.startsWith('/*')) {
                continue
              }
              if (trimmed === '\'use client\'' || trimmed === '"use client"'
                || trimmed === '\'use client\';' || trimmed === '"use client";') {
                return true
              }
              break
            }
            return false
          }
          catch {
            return false
          }
        })

        const imports = clientComponentsArray.map((componentPath, index) => {
          const relativePath = path.relative(process.cwd(), componentPath)
          const componentName = `ClientComponent${index}`
          return `import ${componentName} from '/${relativePath}';`
        }).join('\n')

        const registrations = clientComponentsArray.map((componentPath, index) => {
          const relativePath = path.relative(process.cwd(), componentPath)
          const componentName = `ClientComponent${index}`
          const componentId = path.basename(componentPath, path.extname(componentPath))

          return `
globalThis['~clientComponents']["${relativePath}"] = {
  id: "${componentId}",
  path: "${relativePath}",
  type: "client",
  component: ${componentName},
  registered: true
};
globalThis['~clientComponents']["${componentId}"] = globalThis['~clientComponents']["${relativePath}"];
globalThis['~clientComponentPaths']["${relativePath}"] = "${componentId}";`
        }).join('\n')

        return await loadEntryClient(imports, registrations)
      }

      if (id === 'react-server-dom-rari/server') {
        return await loadReactServerDomShim()
      }

      if (id === 'virtual:app-router-provider.tsx') {
        const possiblePaths = [
          path.join(process.cwd(), 'packages/rari/src/runtime/AppRouterProvider.tsx'),
          path.join(process.cwd(), 'src/runtime/AppRouterProvider.tsx'),
          path.join(process.cwd(), 'node_modules/rari/src/runtime/AppRouterProvider.tsx'),
        ]

        for (const providerSourcePath of possiblePaths) {
          if (fs.existsSync(providerSourcePath)) {
            return fs.readFileSync(providerSourcePath, 'utf-8')
          }
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
          if (fs.existsSync(sourcePath)) {
            return fs.readFileSync(sourcePath, 'utf-8')
          }
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
          if (fs.existsSync(sourcePath)) {
            return fs.readFileSync(sourcePath, 'utf-8')
          }
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
          if (fs.existsSync(sourcePath)) {
            return fs.readFileSync(sourcePath, 'utf-8')
          }
        }

        return 'export class LoadingComponentRegistry { loadComponent() { return Promise.resolve(null); } }'
      }

      if (id === 'virtual:rsc-integration') {
        return await loadRscClientRuntime()
      }

      if (id === 'virtual:react-server-dom-rari-client') {
        return await loadRuntimeFile('react-server-dom-rari-client.js')
      }
    },

    async handleHotUpdate({ file, server }) {
      const isReactFile = /\.(?:tsx?|jsx?)$/.test(file)

      if (!isReactFile) {
        return undefined
      }

      if (file.includes('/dist/') || file.includes('\\dist\\')) {
        return []
      }

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
        if (isLayoutFile) {
          fileType = 'layout'
        }
        else if (isLoadingFile) {
          fileType = 'loading'
        }
        else if (isErrorFile) {
          fileType = 'error'
        }
        else if (isNotFoundFile) {
          fileType = 'not-found'
        }

        if (serverComponentBuilder && componentType === 'server') {
          try {
            await (serverComponentBuilder as any).rebuildComponent(file)
          }
          catch (error) {
            console.error(`[HMR] Failed to rebuild ${file}:`, error)
          }
        }

        server.hot.send('rari:app-router-updated', {
          type: 'rari-hmr',
          filePath: file,
          fileType,
        })

        return undefined
      }

      if (componentType === 'client') {
        if (hmrCoordinator) {
          await hmrCoordinator.handleClientComponentUpdate(file, server)
        }
        return undefined
      }

      if (componentType === 'server') {
        if (hmrCoordinator) {
          await hmrCoordinator.handleServerComponentUpdate(file, server)
        }
        return []
      }

      return undefined
    },
  }

  const serverBuildPlugin = createServerBuildPlugin(options.serverBuild)
  const loadingComponentPlugin = createLoadingComponentPlugin()

  return [mainPlugin, serverBuildPlugin, loadingComponentPlugin]
}

export function defineRariConfig(
  config: UserConfig & { plugins?: Plugin[] },
): UserConfig {
  return {
    plugins: [rari(), ...(config.plugins || [])],
    ...config,
  }
}
