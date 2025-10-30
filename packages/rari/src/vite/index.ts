import type { Buffer } from 'node:buffer'
import type { Plugin, UserConfig } from 'rolldown-vite'
import type { ServerBuildOptions } from './server-build'
import { spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import * as acorn from 'acorn'
import { HMRCoordinator } from './hmr-coordinator'
import { createServerBuildPlugin } from './server-build'

interface RariOptions {
  projectRoot?: string
  serverBuild?: ServerBuildOptions
  serverHandler?: boolean
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

  const mainPlugin: Plugin = {
    name: 'rari',

    config(config: UserConfig, { command }) {
      config.resolve = config.resolve || {}
      const existingDedupe = Array.isArray((config.resolve as any).dedupe)
        ? ((config.resolve as any).dedupe as string[])
        : []
      const toAdd = ['react', 'react-dom'];
      (config.resolve as any).dedupe = Array.from(
        new Set([...(existingDedupe || []), ...toAdd]),
      )

      const existingAlias: Array<{
        find: string | RegExp
        replacement: string
      }> = Array.isArray((config.resolve as any).alias)
        ? (config.resolve as any).alias
        : []
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
            env.build.rollupOptions = env.build.rollupOptions || {}
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
        config.build.rollupOptions = config.build.rollupOptions || {}

        if (!config.build.rollupOptions.input) {
          config.build.rollupOptions.input = {
            main: './index.html',
          }
        }
      }

      if (config.environments && config.environments.client) {
        if (!config.environments.client.build) {
          config.environments.client.build = {}
        }
        if (!config.environments.client.build.rollupOptions) {
          config.environments.client.build.rollupOptions = {}
        }
        if (!config.environments.client.build.rollupOptions.input) {
          config.environments.client.build.rollupOptions.input = {}
        }
      }

      return config
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
  globalThis.__rari_server_components = globalThis.__rari_server_components || new Set();
  globalThis.__rari_server_components.add(${JSON.stringify(id)});
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

      let serverComponentBuilder: any = null

      const discoverAndRegisterComponents = async () => {
        try {
          const { ServerComponentBuilder, scanDirectory } = await import(
            './server-build',
          )

          const builder = new ServerComponentBuilder(projectRoot, {
            outDir: 'dist',
            serverDir: 'server',
            manifestPath: 'server-manifest.json',
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

      if (id === 'virtual:app-router-hmr-provider') {
        return `${id}.tsx`
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

    load(id) {
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
globalThis.__clientComponents["${relativePath}"] = {
  id: "${componentId}",
  path: "${relativePath}",
  type: "client",
  component: ${componentName},
  registered: true
};
globalThis.__clientComponents["${componentId}"] = globalThis.__clientComponents["${relativePath}"];
globalThis.__clientComponentPaths["${relativePath}"] = "${componentId}";`
        }).join('\n')

        const isDevelopment = process.env.NODE_ENV !== 'production'

        return `
import React from 'react';
import { createRoot, hydrateRoot } from 'react-dom/client';
import 'virtual:rsc-integration';
${isDevelopment ? 'import { AppRouterHMRProvider } from \'virtual:app-router-hmr-provider\';' : ''}

${imports}

if (typeof globalThis.__clientComponents === 'undefined') {
  globalThis.__clientComponents = {};
}
if (typeof globalThis.__clientComponentPaths === 'undefined') {
  globalThis.__clientComponentPaths = {};
}

${registrations}

let isInitialHydration = true;

export async function renderApp() {
  const rootElement = document.getElementById('root');
  if (!rootElement) {
    console.error('[Rari] Root element not found');
    return;
  }

  try {
    const hasSSRContent = rootElement.innerHTML.trim().length > 0 && isInitialHydration;

    const rariServerUrl = window.location.origin.includes(':5173')
      ? 'http://localhost:3000'
      : window.location.origin;
    const url = rariServerUrl + window.location.pathname + window.location.search;

    const response = await fetch(url, {
      headers: {
        'Accept': 'text/x-component',
      },
    });

    if (!response.ok) {
      throw new Error(\`Failed to fetch RSC data: \${response.status}\`);
    }

    const rscWireFormat = await response.text();

    const { element, isFullDocument } = parseRscWireFormat(rscWireFormat);

    let contentToRender;
    if (isFullDocument) {
      const bodyContent = extractBodyContent(element, false);
      if (bodyContent) {
        contentToRender = bodyContent;
      } else {
        console.error('[Rari] Could not extract body content, falling back to full element');
        contentToRender = element;
      }
    } else {
      contentToRender = element;
    }

    ${isDevelopment
      ? `
    const wrappedContent = React.createElement(
      AppRouterHMRProvider,
      { initialPayload: { element, rscWireFormat } },
      contentToRender
    );
    `
      : 'const wrappedContent = contentToRender;'}

    if (hasSSRContent) {
      hydrateRoot(rootElement, wrappedContent);
      isInitialHydration = false;
    } else {
      const root = createRoot(rootElement);
      root.render(wrappedContent);
    }
  } catch (error) {
    console.error('[Rari] Error rendering app:', error);
    rootElement.innerHTML = \`
      <div style="padding: 20px; background: #fee; border: 2px solid #f00; margin: 20px;">
        <h2>Error Loading App</h2>
        <p>\${error instanceof Error ? error.message : String(error)}</p>
      </div>
    \`;
  }
}

function extractBodyContent(element, skipHeadInjection = false) {
  console.log('[Rari] extractBodyContent - element:', element);
  console.log('[Rari] extractBodyContent - element.type:', element?.type);
  console.log('[Rari] extractBodyContent - element.props:', element?.props);

  if (element && element.type === 'html' && element.props && element.props.children) {
    const children = Array.isArray(element.props.children)
      ? element.props.children
      : [element.props.children];

    console.log('[Rari] extractBodyContent - children:', children);

    let headElement = null;
    let bodyElement = null;

    for (const child of children) {
      console.log('[Rari] extractBodyContent - checking child:', child, 'type:', child?.type);
      if (child && child.type === 'head') {
        headElement = child;
      } else if (child && child.type === 'body') {
        bodyElement = child;
      }
    }

    if (bodyElement) {
      console.log('[Rari] extractBodyContent - found body');

      if (!skipHeadInjection && headElement && headElement.props && headElement.props.children) {
        console.log('[Rari] extractBodyContent - found head, extracting styles');
        injectHeadContent(headElement);
      } else if (skipHeadInjection) {
        console.log('[Rari] extractBodyContent - skipping head injection (SSR hydration)');
      }

      const bodyChildren = bodyElement.props?.children;
      console.log('[Rari] extractBodyContent - body children:', bodyChildren);

      if (bodyChildren &&
          typeof bodyChildren === 'object' &&
          !Array.isArray(bodyChildren) &&
          bodyChildren.type === 'div' &&
          bodyChildren.props?.id === 'root') {
        console.log('[Rari] extractBodyContent - found root div in body, returning its children to avoid nesting');
        return bodyChildren.props?.children || null;
      }

      console.log('[Rari] extractBodyContent - returning body children as-is');
      return bodyChildren || null;
    }
  }

  console.log('[Rari] extractBodyContent - no body found, returning null');
  return null;
}

function injectHeadContent(headElement) {
  const headChildren = Array.isArray(headElement.props.children)
    ? headElement.props.children
    : [headElement.props.children];

  for (const child of headChildren) {
    if (!child) continue;

    if (child.type === 'style' && child.props && child.props.children) {
      console.log('[Rari] Injecting style tag');
      const styleElement = document.createElement('style');

      const styleContent = Array.isArray(child.props.children)
        ? child.props.children.join('')
        : child.props.children;

      styleElement.textContent = styleContent;
      document.head.appendChild(styleElement);
    }
    else if (child.type === 'meta' && child.props) {
      console.log('[Rari] Injecting meta tag');
      const metaElement = document.createElement('meta');
      Object.keys(child.props).forEach(key => {
        if (key !== 'children') {
          metaElement.setAttribute(key, child.props[key]);
        }
      });
      document.head.appendChild(metaElement);
    }
    else if (child.type === 'title' && child.props && child.props.children) {
      console.log('[Rari] Setting document title');
      document.title = Array.isArray(child.props.children)
        ? child.props.children.join('')
        : child.props.children;
    }
  }
}

function parseRscWireFormat(wireFormat) {
  const lines = [];
  let currentLine = '';
  let inString = false;
  let escapeNext = false;

  for (let i = 0; i < wireFormat.length; i++) {
    const char = wireFormat[i];

    if (escapeNext) {
      currentLine += char;
      escapeNext = false;
      continue;
    }

    if (char === '\\\\') {
      currentLine += char;
      escapeNext = true;
      continue;
    }

    if (char === '"' && !escapeNext) {
      inString = !inString;
      currentLine += char;
      continue;
    }

    if (char === '\\n' && !inString) {
      if (currentLine.trim()) {
        lines.push(currentLine);
      }
      currentLine = '';
      continue;
    }

    currentLine += char;
  }

  if (currentLine.trim()) {
    lines.push(currentLine);
  }

  let rootElement = null;
  let isFullDocument = false;
  const modules = new Map();

  for (const line of lines) {
    const colonIndex = line.indexOf(':');
    if (colonIndex === -1) continue;

    const rowId = line.substring(0, colonIndex);
    const content = line.substring(colonIndex + 1);

    try {
      if (content.startsWith('I[')) {
        const importData = JSON.parse(content.substring(1));
        if (Array.isArray(importData) && importData.length >= 3) {
          const [path, chunks, exportName] = importData;
          modules.set(\`$L\${rowId}\`, {
            id: path,
            chunks: Array.isArray(chunks) ? chunks : [chunks],
            name: exportName || 'default',
          });
        }
      } else if (content.startsWith('[')) {
        const elementData = JSON.parse(content);
        if (!rootElement && Array.isArray(elementData) && elementData[0] === '$') {
          rootElement = rscToReact(elementData, modules);
          if (elementData[1] === 'html') {
            isFullDocument = true;
          }
        }
      }
    } catch (e) {
      console.error('[Rari] Failed to parse RSC line:', line, e);
    }
  }

  if (!rootElement) {
    throw new Error('No root element found in RSC wire format');
  }

  return { element: rootElement, modules, isFullDocument };
}

function rscToReact(rsc, modules) {
  if (!rsc) return null;

  if (typeof rsc === 'string' || typeof rsc === 'number' || typeof rsc === 'boolean') {
    return rsc;
  }

  if (Array.isArray(rsc)) {
    if (rsc.length >= 4 && rsc[0] === '$') {
      const [, type, key, props] = rsc;

      if (typeof type === 'string' && type.startsWith('$L')) {
        const moduleInfo = modules.get(type);
        if (moduleInfo) {
          const Component = globalThis.__clientComponents[moduleInfo.id]?.component;
          if (Component) {
            const childProps = {
              ...props,
              children: props.children ? rscToReact(props.children, modules) : undefined,
            };
            return React.createElement(Component, { key, ...childProps });
          }
        }
        return null;
      }

      const processedProps = processProps(props, modules);
      return React.createElement(type, key ? { ...processedProps, key } : processedProps);
    }
    return rsc.map((child) => rscToReact(child, modules));
  }

  return rsc;
}

function processProps(props, modules) {
  if (!props || typeof props !== 'object') return props;

  const processed = {};
  for (const key in props) {
    if (Object.prototype.hasOwnProperty.call(props, key)) {
      if (key.startsWith('$$') || key === 'ref') {
        continue;
      }
      if (key === 'children') {
        processed[key] = props.children ? rscToReact(props.children, modules) : undefined;
      } else {
        processed[key] = props[key];
      }
    }
  }
  return processed;
}

renderApp().catch((err) => {
  console.error('[Rari] Fatal error:', err);
});
`
      }

      if (id === 'react-server-dom-rari/server') {
        return `
let clientReferenceRegistry = new Map();
let serverReferenceRegistry = new Map();

let rustBridge = null;

if (typeof globalThis.__rari_bridge !== 'undefined') {
  rustBridge = globalThis.__rari_bridge;
}

export function registerClientReference(clientReference, id, exportName) {
  const key = id + '#' + exportName;
  clientReferenceRegistry.set(key, {
    id,
    exportName,
    chunks: [id],
    name: exportName,
    async: false
  });

  Object.defineProperty(clientReference, '$$typeof', {
    value: Symbol.for('react.client.reference'),
    enumerable: false
  });

  Object.defineProperty(clientReference, '$$id', {
    value: key,
    enumerable: false
  });

  Object.defineProperty(clientReference, '$$async', {
    value: false,
    enumerable: false
  });

  try {
    if (rustBridge && typeof rustBridge.registerClientReference === 'function') {
      rustBridge.registerClientReference(key, id, exportName);
    }
  } catch (error) {
  }

  return clientReference;
}

const clientComponentRegistry = new Map();

export function registerClientComponent(componentFunction, id, exportName) {
  const key = id + '#' + exportName;
  clientComponentRegistry.set(key, componentFunction);
  clientReferenceRegistry.set(key, {
    id,
    exportName,
    chunks: [id],
    name: exportName,
    async: false
  });
}

export function getClientComponent(id) {
  return clientComponentRegistry.get(id);
}

export function registerServerReference(serverReference, id, exportName) {
  const key = id + '#' + exportName;
  serverReferenceRegistry.set(key, {
    id,
    exportName,
    bound: false
  });

  Object.defineProperty(serverReference, '$$typeof', {
    value: Symbol.for('react.server.reference'),
    enumerable: false
  });

  Object.defineProperty(serverReference, '$$id', {
    value: key,
    enumerable: false
  });

  Object.defineProperty(serverReference, '$$bound', {
    value: false,
    enumerable: false
  });

  try {
    if (rustBridge && typeof rustBridge.registerServerReference === 'function') {
      rustBridge.registerServerReference(key, id, exportName);
    }
  } catch (error) {
  }

  return serverReference;
}

export function createClientModuleProxy(id) {
  return new Proxy({}, {
    get(target, prop) {
      const key = id + '#' + String(prop);

      function clientProxy() {
        throw new Error(
          \`Attempted to call \${String(prop)}() from the server but \${String(prop)} is on the client. \` +
          \`It's not possible to invoke a client function from the server, it can only be rendered as a Component or passed to props of a Client Component.\`
        );
      }

      return registerClientReference(clientProxy, id, String(prop));
    }
  });
}

export const __CLIENT_REFERENCE_REGISTRY__ = clientReferenceRegistry;
export const __SERVER_REFERENCE_REGISTRY__ = serverReferenceRegistry;
export const __CLIENT_COMPONENT_REGISTRY__ = clientComponentRegistry;

export function createClientModuleMap() {
  const moduleMap = {};

  for (const [key, componentData] of clientReferenceRegistry) {
    const component = clientComponentRegistry.get(key);
    if (component) {
      moduleMap[key] = {
        id: componentData.id,
        chunks: componentData.chunks,
        name: componentData.name,
        async: componentData.async,
        default: component
      };
    }
  }

  return moduleMap;
}
`
      }

      if (id === 'virtual:app-router-hmr-provider.tsx') {
        const possiblePaths = [
          path.join(process.cwd(), 'packages/rari/src/runtime/AppRouterHMRProvider.tsx'),
          path.join(process.cwd(), 'src/runtime/AppRouterHMRProvider.tsx'),
          path.join(process.cwd(), 'node_modules/rari/src/runtime/AppRouterHMRProvider.tsx'),
        ]

        for (const providerSourcePath of possiblePaths) {
          if (fs.existsSync(providerSourcePath)) {
            return fs.readFileSync(providerSourcePath, 'utf-8')
          }
        }

        return 'export function AppRouterHMRProvider({ children }) { return children; }'
      }

      if (id === 'virtual:rsc-integration') {
        const isDevelopment = process.env.NODE_ENV !== 'production'
        return `
import { useState, useEffect, Suspense, createElement, isValidElement, cloneElement } from 'react';
import * as ReactDOMClient from 'react-dom/client';

if (typeof globalThis.__rari === 'undefined') {
  globalThis.__rari = {};
}
globalThis.__rari.isDevelopment = ${isDevelopment};

if (typeof globalThis.__clientComponents === 'undefined') {
  globalThis.__clientComponents = {};
}
if (typeof globalThis.__clientComponentNames === 'undefined') {
  globalThis.__clientComponentNames = {};
}
if (typeof globalThis.__clientComponentPaths === 'undefined') {
  globalThis.__clientComponentPaths = {};
}

export function registerClientComponent(componentFunction, id, exportName) {
  const componentName = exportName === 'default' ?
    (componentFunction.name || id.split('/').pop()?.replace(/\.[^/.]+$/, '') || 'DefaultComponent') :
    exportName;

  const componentId = componentName;

  globalThis.__clientComponents[componentId] = {
    id: componentId,
    path: id,
    type: 'client',
    component: componentFunction,
    registered: true
  };

  globalThis.__clientComponentPaths[id] = componentId;

  globalThis.__clientComponentNames[componentName] = componentId;

  if (componentFunction) {
    componentFunction.__isClientComponent = true;
    componentFunction.__clientComponentId = componentId;
  }

  if (typeof window !== 'undefined') {
    fetch('/api/rsc/register-client', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        component_id: componentId,
        file_path: id,
        export_name: exportName
      })
    }).catch(error => {
      console.error('[RARI] Failed to register client component with server:', error);
    });
  }
}

export function getClientComponent(id) {
  if (globalThis.__clientComponents[id]?.component) {
    return globalThis.__clientComponents[id].component;
  }

  if (id.includes('#')) {
    const [path, exportName] = id.split('#');

    const componentId = globalThis.__clientComponentPaths[path];
    if (componentId && globalThis.__clientComponents[componentId]) {
      const componentInfo = globalThis.__clientComponents[componentId];
      if (exportName === 'default' || !exportName) {
        return componentInfo.component;
      }
    }

    const normalizedPath = path.startsWith('./') ? path.slice(2) : path;
    const componentIdByNormalizedPath = globalThis.__clientComponentPaths[normalizedPath];
    if (componentIdByNormalizedPath && globalThis.__clientComponents[componentIdByNormalizedPath]) {
      return globalThis.__clientComponents[componentIdByNormalizedPath].component;
    }
  }

  const componentId = globalThis.__clientComponentNames[id];
  if (componentId && globalThis.__clientComponents[componentId]) {
    return globalThis.__clientComponents[componentId].component;
  }

  return null;
}

export function createClientModuleMap() {
  const moduleMap = {};
  for (const [componentId, componentInfo] of Object.entries(globalThis.__clientComponents)) {
    moduleMap[componentId] = {
      id: componentId,
      chunks: [componentInfo.path],
      name: componentId,
      async: false,
      default: componentInfo.component
    };
  }
  return moduleMap;
}

let createFromFetch = ReactDOMClient.createFromFetch || null;
let createFromReadableStream = ReactDOMClient.createFromReadableStream || null;
let rscClientLoadPromise = null;

async function loadRscClient() {
  if (rscClientLoadPromise) {
    return rscClientLoadPromise;
  }

  rscClientLoadPromise = (async () => {
    try {
      createFromFetch = ReactDOMClient.createFromFetch;
      createFromReadableStream = ReactDOMClient.createFromReadableStream;

      if (typeof createFromReadableStream !== 'function') {
        createFromReadableStream = null;
      }
      if (typeof createFromFetch !== 'function') {
        createFromFetch = null;
      }

      return ReactDOMClient;
    } catch (error) {
      console.error('Failed to load react-dom/client RSC functions:', error);
      createFromFetch = null;
      createFromReadableStream = null;
      return null;
    }
  })()

  return rscClientLoadPromise;
}

class RscClient {
  constructor() {
    this.componentCache = new Map();
    this.moduleCache = new Map();
    this.inflightRequests = new Map();
    this.config = {
      enableStreaming: true,
      maxRetries: 5,
      retryDelay: 500,
      timeout: 10000,
    };
  }

  configure(config) {
    this.config = { ...this.config, ...config };
  }

  clearCache() {
    this.componentCache.clear();
    this.moduleCache.clear();
  }

  async fetchServerComponent(componentId, props = {}) {
    const hmrCounter = (typeof window !== 'undefined' && window.__rscRefreshCounters && window.__rscRefreshCounters[componentId]) || 0;
    const cacheKey = componentId + ':' + JSON.stringify(props) + ':hmr:' + hmrCounter;


    if (this.componentCache.has(cacheKey)) {
      return this.componentCache.get(cacheKey);
    }

    if (this.inflightRequests.has(cacheKey)) {
      return this.inflightRequests.get(cacheKey);
    }

    let requestPromise;
    if (this.config.enableStreaming) {
      requestPromise = this.fetchServerComponentStreamV2(componentId, props);
    } else {
      requestPromise = (async () => {
        const encodedProps = encodeURIComponent(JSON.stringify(props));
        const cacheBuster = Date.now();
        const fetchUrl = '/rsc/render/' + componentId + '?props=' + encodedProps + '&_t=' + cacheBuster;
        await this.waitForServerReady();
        const response = await this.fetchWithTimeout(fetchUrl, {
          method: 'GET',
          headers: {
            ...this.buildRequestHeaders(),
            'Cache-Control': 'no-cache, no-store, must-revalidate',
            'Pragma': 'no-cache',
            'Expires': '0'
          },
        });
        if (!response.ok) {
          throw new Error('Server responded with ' + response.status + ': ' + response.statusText);
        }
        try {
          return await this.processRscResponseManually(response);
        } catch (manualError) {
          const fallback = await this.processRscResponse(response);
          return fallback;
        }
      })();
    }

    this.inflightRequests.set(cacheKey, requestPromise);
    try {
      const result = await requestPromise;
      this.componentCache.set(cacheKey, result);
      return result;
    } finally {
      this.inflightRequests.delete(cacheKey);
    }

  }

  async fetchServerComponentStreamV2(componentId, props = {}) {
    await loadRscClient();

    const endpoints = (() => {
      const list = ['/api/rsc/stream'];
      try {
        const isLocalHost = typeof window !== 'undefined' && (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1');
        if (isLocalHost) {
          list.push('http://127.0.0.1:3000/api/rsc/stream', 'http://localhost:3000/api/rsc/stream');
        }
      } catch {}
      return list;
    })();
    let response = null;
    let lastError = null;
    const attempt = async () => {
      for (const url of endpoints) {
        try {
          const r = await this.fetchWithTimeout(url, {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              ...this.buildRequestHeaders(),
            },
            body: JSON.stringify({ component_id: componentId, props }),
          });
          if (r.ok) {
            return r;
          }
          lastError = new Error('HTTP ' + r.status + ': ' + (await r.text()));
        } catch (e) {
          lastError = e;
        }
      }
      return null;
    };

    const abortController = new AbortController();
    const abortTimeout = setTimeout(() => abortController.abort(), this.config.timeout);

    response = await attempt();
    if (!response) {
      await new Promise(r => setTimeout(r, 150));
      response = await attempt();
    }
    if (!response) {
      throw lastError || new Error('Failed to reach stream endpoint');
    }

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error('Server responded with ' + response.status + ': ' + errorText);
    }

    const stream = response.body;
    if (!stream) {
      throw new Error('No ReadableStream from stream response');
    }

    if (false && createFromReadableStream) {
      try {
        const rscPromise = createFromReadableStream(stream);
        return {
          _isRscResponse: true,
          _rscPromise: rscPromise,
          readRoot() {
            return rscPromise;
          }
        };
      } catch (error) {
        console.error('Failed to use createFromReadableStream:', error);
      }
    }

    const reader = stream.getReader();
    const decoder = new TextDecoder();
    let content = '';
    const modules = new Map();
    const boundaryRowMap = new Map();

    const convertRscToReact = (element) => {
      if (!createElement) {
        console.error('React not available for RSC conversion');
        return null;
      }

      if (!element) {
        return null;
      }

      if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {
        return element;
      }

      if (Array.isArray(element)) {
        if (element.length >= 3 && element[0] === '$') {
          const [, type, key, props] = element;

          if (type === 'react.suspense' || type === 'suspense') {

            const suspenseWrapper = createElement('div',
              {
                'data-boundary-id': props?.boundaryId,
                boundaryId: props?.boundaryId,
                'data-suspense-boundary': true
              },
              convertRscToReact(props?.fallback || props?.children)
            );

            return suspenseWrapper;
          }

          let processedProps = props ? { ...props } : {};
          if (props?.children) {
            const child = convertRscToReact(props.children);
            if (Array.isArray(child)) {
              processedProps.children = child.map((c, i) => isValidElement(c) ? cloneElement(c, { key: (c.key ?? i) }) : c);
            } else {
              processedProps.children = child;
            }
          }

           if (typeof type === 'string') {
            if (type.startsWith('$L')) {
              const mod = modules.get(type);
              if (mod) {
                const clientKey = mod.id + '#' + (mod.name || 'default');
                const clientComponent = getClientComponent(clientKey);
                if (clientComponent) {
                  const reactElement = createElement(clientComponent, key ? { ...processedProps, key } : processedProps);
                  return reactElement;
                }
              }
              return processedProps && processedProps.children ? processedProps.children : null;
            }
            if (type.includes('.tsx#') || type.includes('.jsx#')) {
              const clientComponent = getClientComponent(type);
              if (clientComponent) {
                const reactElement = createElement(clientComponent, key ? { ...processedProps, key } : processedProps);
                return reactElement;
              } else {
                console.error('Failed to resolve client component:', type);
                return null;
              }
            } else {
              if (processedProps && Object.prototype.hasOwnProperty.call(processedProps, 'boundaryId')) {
                const { boundaryId, ...rest } = processedProps;
                processedProps = rest;
              }
              const reactElement = createElement(type, key ? { ...processedProps, key } : processedProps);
              return reactElement;
            }
          } else {
            console.error('Unknown RSC element type:', type);
          }
        }

        return element.map((child, index) => {
          const converted = convertRscToReact(child);
          return converted;
        });
      }

      if (typeof element === 'object') {
        console.error('Unexpected object in RSC conversion:', element);
        return null;
      }

      return element;
    };

    let initialContent = null;
    let boundaryUpdates = new Map();
    let isComplete = false;
    let buffered = '';

    const processStream = async () => {
      const newlineChar = String.fromCharCode(10);

      try {
        while (true) {
          const { value, done } = await reader.read();
          if (done) {
            isComplete = true;
            break;
          }

          const chunk = decoder.decode(value, { stream: true });
          buffered += chunk;

          const lines = buffered.split(newlineChar);
          const completeLines = lines.slice(0, -1);
          buffered = lines[lines.length - 1];

          for (const line of completeLines) {
              if (!line.trim()) continue;

              try {
                const colonIndex = line.indexOf(':');
                if (colonIndex === -1) continue;

                const rowId = line.substring(0, colonIndex);
                const content = line.substring(colonIndex + 1);


              if (content.includes('STREAM_COMPLETE')) {
                isComplete = true;
              } else if (content.startsWith('I[')) {
                try {
                  const importData = JSON.parse(content.substring(1));
                  if (Array.isArray(importData) && importData.length >= 3) {
                    const [path, chunks, exportName] = importData;
                    modules.set('$L' + rowId, {
                      id: path,
                      chunks: Array.isArray(chunks) ? chunks : [chunks],
                      name: exportName || 'default'
                    });
                  }
                } catch (e) {
                  console.error('Failed to parse import row:', content, e);
                }
              } else if (content.startsWith('E{')) {
                try {
                  const err = JSON.parse(content.substring(1));
                  console.error('RSC stream error:', err);
                } catch (e) {
                  console.error('Failed to parse error row:', content, e);
                }
              } else if (content.startsWith('Symbol.for(')) {
                continue;
              } else if (content.startsWith('[')) {
                const parsed = JSON.parse(content);
                if (Array.isArray(parsed) && parsed.length >= 4) {
                  const [marker, selector, key, props] = parsed;
                   if (marker === '$' && (selector === 'react.suspense' || selector === 'suspense') && props && props.boundaryId) {
                    boundaryRowMap.set('$L' + rowId, props.boundaryId);
                  }
                  if (marker === '$' && props && Object.prototype.hasOwnProperty.call(props, 'children')) {
                    if (typeof selector === 'string' && selector.startsWith('$L')) {
                      const target = boundaryRowMap.get(selector) || null;
                      if (target) {
                        const resolvedContent = convertRscToReact(props.children);
                        boundaryUpdates.set(target, resolvedContent);
                        if (streamingComponent) {
                          streamingComponent.updateBoundary(target, resolvedContent);
                        }
                        continue;
                      }
                    }
                  }
                }
                if (!initialContent) {
                  let canUseAsRoot = true;
                  if (Array.isArray(parsed) && parsed.length >= 4 && parsed[0] === '$') {
                    const sel = parsed[1];
                    const p = parsed[3];
                     if (typeof sel === 'string' && (sel === 'react.suspense' || sel === 'suspense') && p && p.boundaryId) {
                      canUseAsRoot = false;
                    }
                  }
                  if (canUseAsRoot) {
                    initialContent = convertRscToReact(parsed);
                    if (streamingComponent && typeof streamingComponent.updateRoot === 'function') {
                      streamingComponent.updateRoot();
                    }
                  }
                }
              }
            } catch (e) {
              console.error('Failed to parse stream line:', line, e);
            }
          }
        }
      } catch (error) {
        console.error('Error processing stream:', error);
        isComplete = true;
      }
    };

    let streamingComponent = null;

    const StreamingWrapper = () => {
      const [renderTrigger, setRenderTrigger] = useState(0);

      useEffect(() => {
        streamingComponent = {
          updateBoundary: (boundaryId, resolvedContent) => {
            boundaryUpdates.set(boundaryId, resolvedContent);
            setRenderTrigger(prev => prev + 1);
          },
          updateRoot: () => {
            setRenderTrigger(prev => prev + 1);
          }
        };

        return () => {
          streamingComponent = null;
        };
      }, []);

      const renderWithBoundaryUpdates = (element) => {
        if (!element) return null;

        if (isValidElement(element)) {
          if (element.props && element.props.boundaryId) {
            const boundaryId = element.props.boundaryId;
            const resolvedContent = boundaryUpdates.get(boundaryId);
            if (resolvedContent) {
              return resolvedContent;
            }
          }

          if (element.props && element.props.children) {
            const updatedChildren = renderWithBoundaryUpdates(element.props.children);
            if (updatedChildren !== element.props.children) {
              return cloneElement(element, { ...element.props, children: updatedChildren });
            }
          }

          return element;
        }

        if (Array.isArray(element)) {
          return element.map((child, index) => renderWithBoundaryUpdates(child));
        }

        return element;
      };

      const renderedContent = renderWithBoundaryUpdates(initialContent);
      return renderedContent;
    };

    processStream();

    return {
      _isRscResponse: true,
      _rscPromise: Promise.resolve(createElement(StreamingWrapper)),
      readRoot() {
        return Promise.resolve(createElement(StreamingWrapper));
      }
    };
  }



  buildRequestHeaders() {
    const headers = {
      'Accept': 'text/x-component',
      'Cache-Control': 'no-cache, no-transform',
    };

    if (this.config.enableStreaming) {
      headers['X-RSC-Streaming'] = 'enabled';
    }

    return headers;
  }

  async fetchWithTimeout(url, options) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);

    try {
      const response = await fetch(url, {
        ...options,
        signal: controller.signal,
      });
      clearTimeout(timeoutId);
      return response;
    } catch (error) {
      clearTimeout(timeoutId);
      throw error;
    }
  }

  async processRscResponse(response) {
    await loadRscClient();

    if (createFromFetch) {
      try {
        const rscPromise = createFromFetch(Promise.resolve(response));
        return {
          _isRscResponse: true,
          _rscPromise: rscPromise,
          readRoot() {
            return rscPromise;
          }
        };
      } catch (error) {
        throw new Error('React Server DOM client not available');
      }
    } else {
      throw new Error('React Server DOM client not available');
    }
  }

  async processRscResponseManually(response) {
    const rscPayload = await response.text();
    const result = this.parseRscResponse(rscPayload);
    return result;
  }

  parseRscResponse(rscPayload) {
    const lines = rscPayload.trim().split('\\n');
    const modules = new Map();
    const elements = new Map();
    const errors = [];

    for (const line of lines) {
      const colonIndex = line.indexOf(':');
      if (colonIndex === -1) continue;

      const rowId = line.substring(0, colonIndex);
      const rest = line.substring(colonIndex + 1);

      if (!rest) continue;

      try {
        if (rest.startsWith('I[')) {
          const data = rest.substring(1);
          const importData = JSON.parse(data);
          if (Array.isArray(importData) && importData.length >= 3) {
            const [path, chunks, exportName] = importData;
            modules.set('$L' + rowId, {
              id: path,
              chunks: Array.isArray(chunks) ? chunks : [chunks],
              name: exportName || 'default'
            });
          }
        } else if (rest.startsWith('E{')) {
          const data = rest.substring(1);
          const errorData = JSON.parse(data);
          errors.push(errorData);
          console.error('RSC: Server error', errorData);
        } else if (rest.startsWith('[')) {
          const elementData = JSON.parse(rest);
          elements.set(rowId, elementData);
        } else if (rest.startsWith('Symbol.for(')) {
          continue;
        } else {
          console.error('Unknown RSC row format:', line);
        }
      } catch (e) {
        console.error('Failed to parse RSC row:', line, e);
      }
    }

    if (errors.length > 0) {
      throw new Error('RSC Server Error: ' + errors.map(e => e.message || e).join(', '));
    }

    let rootElement = null;

    const elementKeys = Array.from(elements.keys()).sort((a, b) => parseInt(a) - parseInt(b));
    for (const key of elementKeys) {
      const element = elements.get(key);
      if (Array.isArray(element) && element.length >= 2 && element[0] === '$') {
        const [, type, , props] = element;
        if (type === 'react.suspense' && props && props.boundaryId) {
          continue;
        }
        rootElement = element;
        break;
      }
    }

    if (!rootElement) {
      console.error('No valid root element found in RSC payload', { elements, modules });
      return null;
    }

    return this.reconstructElementFromRscData(rootElement, modules);
  }

  reconstructElementFromRscData(elementData, modules) {
    if (elementData === null || elementData === undefined) {
      return null;
    }

    if (typeof elementData === 'string' || typeof elementData === 'number' || typeof elementData === 'boolean') {
      return elementData;
    }

    if (Array.isArray(elementData)) {
      if (elementData.length >= 2 && elementData[0] === '$') {
        const [marker, type, key, props] = elementData;

        let actualType = type;

        if (typeof type === 'string' && type.includes('#')) {
          const clientComponent = getClientComponent(type);
          if (clientComponent) {
            actualType = clientComponent;
          } else {
            actualType = ({ children, ...restProps }) => createElement(
              'div',
              {
                ...restProps,
                'data-client-component': type,
                style: {
                  border: '2px dashed #f00',
                  padding: '8px',
                  margin: '4px',
                  backgroundColor: '#fff0f0'
                }
              },
              createElement('small', { style: { color: '#c00' } },
                'Missing Client Component: ' + type
              ),
              children
            );
          }
        } else if (typeof type === 'string' && type.startsWith('$L')) {
          if (modules.has(type)) {
            const moduleData = modules.get(type);
            const clientComponentKey = moduleData.id + '#' + moduleData.name;

            const clientComponent = getClientComponent(clientComponentKey);

            if (clientComponent) {
              actualType = clientComponent;
            } else {
              actualType = ({ children, ...restProps }) => createElement(
                'div',
                {
                  ...restProps,
                  'data-client-component': type,
                  style: {
                    border: '2px dashed #f00',
                    padding: '8px',
                    margin: '4px',
                    backgroundColor: '#fff0f0'
                  }
                },
                createElement('small', { style: { color: '#c00' } },
                  'Missing Client Component: ' + moduleData.name + ' (' + moduleData.id + ')'
                ),
                children
              );
            }
          }
        }

        const processedProps = props ? this.processPropsRecursively(props, modules) : {};

        return createElement(actualType, { key, ...processedProps });
      } else {
        return elementData.map((item, index) => this.reconstructElementFromRscData(item, modules));
      }
    }

    if (typeof elementData === 'object') {
      return null;
    }

    return elementData;
  }

  processPropsRecursively(props, modules) {
    if (!props || typeof props !== 'object') return props;

    const processed = {};

    for (const [key, value] of Object.entries(props)) {
            if (key === 'children') {
        if (Array.isArray(value)) {
          if (value.length >= 2 && value[0] === '$') {
            const result = this.reconstructElementFromRscData(value, modules);
            processed[key] = result;
          } else {
            const processedChildren = value.map((child, index) => {
              const result = this.reconstructElementFromRscData(child, modules);
              return result;
            }).filter(child => child !== null && child !== undefined);

            if (processedChildren.length === 0) {
              processed[key] = null;
            } else if (processedChildren.length === 1) {
              processed[key] = processedChildren[0];
            } else {
              processed[key] = processedChildren;
            }
          }
        } else {
          const processedChild = this.reconstructElementFromRscData(value, modules);
          processed[key] = processedChild;
        }
      } else if (key === 'dangerouslySetInnerHTML') {
        processed[key] = value;
      } else {
        processed[key] = this.reconstructElementFromRscData(value, modules);
      }
    }

    return processed;
  }

  async waitForServerReady() {
    let serverReady = false;
    let retries = 0;

    while (!serverReady && retries < this.config.maxRetries) {
      try {
        const statusResponse = await fetch('/_rsc_status');
        if (statusResponse.ok) {
          const statusData = await statusResponse.json();
          if (statusData.status === 'ready') {
            serverReady = true;
          } else {
            throw new Error('Server status: ' + statusData.status);
          }
        } else {
          throw new Error('Status check failed: ' + statusResponse.status);
        }
      } catch (err) {
        retries++;
        if (retries < this.config.maxRetries) {
          await new Promise(resolve => setTimeout(resolve, this.config.retryDelay));
        }
      }
    }

    if (!serverReady) {
      throw new Error('RSC server is not ready after multiple attempts');
    }
  }
}

const rscClient = new RscClient();

function RscErrorComponent({ error, details }) {
  return createElement('div',
    {
      className: 'rsc-error',
      style: {
        padding: '16px',
        backgroundColor: '#fee',
        border: '1px solid #fcc',
        borderRadius: '4px',
        margin: '8px 0',
        fontFamily: 'monospace'
      }
    },
    createElement('h3', { style: { margin: '0 0 8px 0', color: '#c00' } }, 'RSC Error'),
    createElement('p', { style: { margin: '0 0 8px 0' } }, error),
    details && createElement('details',
      { style: { marginTop: '8px' } },
      createElement('summary', { style: { cursor: 'pointer' } }, 'Error Details'),
      createElement('pre',
        { style: { fontSize: '12px', overflow: 'auto', backgroundColor: '#f5f5f5', padding: '8px' } },
        JSON.stringify(details, null, 2)
      )
    )
  );
}

function ServerComponentWrapper({
  componentId,
  props,
  fallback
}) {
  const [data, setData] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  useEffect(() => {
    let mounted = true;
    setLoading(true);
    setError(null);

    rscClient.fetchServerComponent(componentId, props)
      .then(result => {
        if (mounted) {
          setData(result);
          setLoading(false);
        }
      })
      .catch(err => {
        if (mounted) {
          setError(err);
          setLoading(false);
        }
      });

    return () => {
      mounted = false;
    };
  }, [componentId, JSON.stringify(props)]);

  if (loading) {
    return fallback || null;
  }

  if (error) {
    return createElement(RscErrorComponent, {
      error: 'Error loading component',
      details: { message: error.message, componentId }
    });
  }

    if (data) {
    if (data._isRscResponse) {
      return createElement(Suspense,
        { fallback: fallback || null },
        data.readRoot()
      );
    } else if (data) {
      return data;
    }
  }

  return createElement(RscErrorComponent, {
    error: 'No data received for component: ' + componentId,
    details: { componentId, dataType: typeof data, hasData: !!data }
  });
}

function createServerComponentWrapper(componentName, importPath) {
  let globalRefreshCounter = 0;

  if (typeof window !== 'undefined') {
    window.__rscRefreshCounters = window.__rscRefreshCounters || {};
    if (window.__rscRefreshCounters[componentName] === undefined) {
      window.__rscRefreshCounters[componentName] = 0;
    }
    globalRefreshCounter = window.__rscRefreshCounters[componentName];
  }

  const ServerComponent = (props) => {
    const [mountKey, setMountKey] = useState(globalRefreshCounter);

    useEffect(() => {
      const handleRscInvalidate = (event) => {
        const detail = event.detail;
        if (detail && detail.filePath && isServerComponent(detail.filePath)) {

          rscClient.clearCache();

          if (typeof window !== 'undefined') {
            window.__rscRefreshCounters[componentName] = (window.__rscRefreshCounters[componentName] || 0) + 1;
            setMountKey(window.__rscRefreshCounters[componentName]);
          }
        }
      };

      if (typeof window !== 'undefined') {
        window.addEventListener('rari:rsc-invalidate', handleRscInvalidate);
        return () => window.removeEventListener('rari:rsc-invalidate', handleRscInvalidate);
      }
    }, []);

    return createElement(Suspense, {
      fallback: null
    }, createElement(ServerComponentWrapper, {
      key: componentName + '-' + mountKey,
      componentId: componentName,
      props: props,
      fallback: null
    }));
  };

  ServerComponent.displayName = 'ServerComponent(' + componentName + ')';

  return function(props) {
    return createElement(ServerComponent, props);
  };
}

export const fetchServerComponent = (componentId, props) =>
  rscClient.fetchServerComponent(componentId, props);

function isServerComponent(filePath) {
  if (!filePath) {
    return false;
  }

  try {
    if (typeof globalThis !== 'undefined' && globalThis.__rari_server_components) {
      return globalThis.__rari_server_components.has(filePath);
    }

    return false;
  } catch (error) {
    console.error('Error checking if file is server component:', error);
    return false;
  }
}

if (import.meta.hot) {
  import.meta.hot.on('rari:register-server-component', (data) => {
    if (data?.filePath) {
      if (typeof globalThis !== 'undefined') {
        globalThis.__rari_server_components = globalThis.__rari_server_components || new Set();
        globalThis.__rari_server_components.add(data.filePath);
      }
    }
  });

  import.meta.hot.on('rari:server-components-registry', (data) => {
    if (data?.serverComponents && Array.isArray(data.serverComponents)) {
      if (typeof globalThis !== 'undefined') {
        globalThis.__rari_server_components = globalThis.__rari_server_components || new Set();
        data.serverComponents.forEach(path => {
          globalThis.__rari_server_components.add(path);
        });
      }
    }
  });

  import.meta.hot.on('vite:beforeFullReload', async (data) => {
    if (data?.path && isServerComponent(data.path)) {
      await invalidateRscCache({ filePath: data.path, forceReload: true });
    }
  });

  import.meta.hot.on('rari:server-component-updated', async (data) => {
    console.warn('[HMR] ⚡ Received rari:server-component-updated event!', data);

    const componentId = data?.id || data?.componentId;
    const timestamp = data?.t || data?.timestamp;

    if (componentId) {
      console.warn('[HMR] Server component updated: ' + componentId);

      if (typeof window !== 'undefined') {
        console.warn('[HMR] Dispatching window event rari:rsc-invalidate');
        const event = new CustomEvent('rari:rsc-invalidate', {
          detail: {
            componentId: componentId,
            filePath: data.filePath || data.file,
            type: 'server-component',
            timestamp: timestamp
          }
        });
        window.dispatchEvent(event);
        console.warn('[HMR] Window event dispatched');
      }
    }
    else if (data?.path && isServerComponent(data.path)) {
      console.warn('[HMR] Legacy format, invalidating cache for:', data.path);
      await invalidateRscCache({ filePath: data.path, forceReload: false });
    }
  });

  import.meta.hot.on('rari:app-router-updated', async (data) => {
    console.log('[HMR] Received app-router-updated event:', data);
    try {
      if (!data) return;

      await handleAppRouterUpdate(data);
    } catch (error) {
      console.error('[HMR] App router update failed:', error);
    }
  });

  import.meta.hot.on('rari:server-action-updated', async (data) => {
    if (data?.filePath) {
      console.log('[HMR] Server action updated:', data.filePath);
      rscClient.clearCache();

      if (typeof window !== 'undefined') {
        const event = new CustomEvent('rari:rsc-invalidate', {
          detail: { filePath: data.filePath, type: 'server-action' }
        });
        window.dispatchEvent(event);
      }
    }
  });

  async function handleAppRouterUpdate(data) {
    const fileType = data.fileType;
    const filePath = data.filePath;
    const routePath = data.routePath;
    const affectedRoutes = data.affectedRoutes;
    const manifestUpdated = data.manifestUpdated;
    const metadata = data.metadata;
    const metadataChanged = data.metadataChanged;

    console.log('[HMR] App router ' + fileType + ' updated: ' + filePath);
    console.log('[HMR] Affected routes:', affectedRoutes);

    if (metadataChanged && metadata) {
      updateDocumentMetadata(metadata);
    }

    try {
      const rariServerUrl = window.location.origin;
      const reloadUrl = rariServerUrl + '/api/rsc/hmr-register';

      console.log('[HMR] Reloading component:', filePath, '(from dist/server)');

      let componentId = filePath;
      if (componentId.startsWith('src/')) {
        componentId = componentId.substring(4);
      }
      componentId = componentId.replace(/\.(tsx|ts|jsx|js)$/, '');

      const reloadResponse = await fetch(reloadUrl, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          file_path: filePath
        })
      });

      if (reloadResponse.ok) {
        const result = await reloadResponse.json();
        console.log('[HMR] Reload response:', result);
      } else {
        console.warn('[HMR] Component reload failed:', reloadResponse.status);
      }
    } catch (error) {
      console.error('[HMR] Failed to reload component:', error);
    }

    let routes = [routePath];
    switch (fileType) {
      case 'page':
        routes = [routePath];
        break;
      case 'layout':
      case 'loading':
      case 'error':
      case 'not-found':
        routes = affectedRoutes;
        break;
      default:
        console.warn('[HMR] Unknown file type: ' + fileType);
    }

    await invalidateAppRouterCache({ routes, fileType, filePath, componentId: routePath });

    if (manifestUpdated) {
      await reloadAppRouterManifest();
    }

    await triggerAppRouterRerender({ routePath, affectedRoutes });
  }

  function updateDocumentMetadata(metadata) {
    if (typeof document === 'undefined') return;

    if (metadata.title) {
      document.title = metadata.title;
    }

    if (metadata.description) {
      let metaDesc = document.querySelector('meta[name="description"]');
      if (!metaDesc) {
        metaDesc = document.createElement('meta');
        metaDesc.setAttribute('name', 'description');
        document.head.appendChild(metaDesc);
      }
      metaDesc.setAttribute('content', metadata.description);
    }
  }

  function clearCacheForRoutes(routes) {
    if (!routes || routes.length === 0) {
      rscClient.clearCache();
      return;
    }

    const keysToDelete = [];
    for (const key of rscClient.componentCache.keys()) {
      for (const route of routes) {
        if (key.includes('route:' + route + ':') || key.startsWith(route + ':')) {
          keysToDelete.push(key);
          break;
        }
        if (route !== '/' && key.includes('route:' + route + '/')) {
          keysToDelete.push(key);
          break;
        }
      }
    }

    for (const key of keysToDelete) {
      rscClient.componentCache.delete(key);
    }

    console.log('[HMR] Cleared cache for ' + keysToDelete.length + ' entries across ' + routes.length + ' route(s)');
  }

  async function invalidateAppRouterCache(data) {
    const routes = data.routes || [];
    const fileType = data.fileType;
    const filePath = data.filePath;
    const componentId = data.componentId;

    console.log('[HMR] Invalidating cache for routes:', routes);

    if (componentId || filePath) {
      try {
        const rariServerUrl = window.location.origin.includes(':5173')
          ? 'http://localhost:3000'
          : window.location.origin;

        const invalidateUrl = rariServerUrl + '/api/rsc/hmr-invalidate';

        console.log('[HMR] Calling server invalidation endpoint for:', componentId || filePath);

        const invalidateResponse = await fetch(invalidateUrl, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            componentId: componentId || filePath,
            filePath: filePath
          }),
        });

        if (invalidateResponse.ok) {
          const result = await invalidateResponse.json();
          console.log('[HMR] Server cache invalidated:', result);
        } else {
          console.warn('[HMR] Server cache invalidation failed:', invalidateResponse.status);
        }
      } catch (error) {
        console.error('[HMR] Failed to call server invalidation endpoint:', error);
      }
    }

    clearCacheForRoutes(routes);

    if (typeof window !== 'undefined') {
      const event = new CustomEvent('rari:rsc-invalidate', {
        detail: { routes, fileType }
      });
      window.dispatchEvent(event);

      const currentPath = window.location.pathname;
      if (routes.includes(currentPath) || routes.includes('/')) {
        console.log('[HMR] Re-fetching current route:', currentPath);

        try {
          const rariServerUrl = window.location.origin.includes(':5173')
            ? 'http://localhost:3000'
            : window.location.origin;
          const url = rariServerUrl + currentPath + window.location.search;

          const response = await fetch(url, {
            headers: {
              'Accept': 'text/x-component',
            },
            cache: 'no-cache',
          });

          if (response.ok) {
            console.log('[HMR] Successfully re-fetched route, triggering re-render');
          }
        } catch (error) {
          console.error('[HMR] Failed to re-fetch route:', error);
        }
      }
    }
  }

  async function triggerAppRouterRerender(data) {
    const routePath = data.routePath;
    const affectedRoutes = data.affectedRoutes || [routePath];

    if (typeof window === 'undefined') {
      return;
    }

    try {
      const currentPath = window.location.pathname;

      const isCurrentRouteAffected = affectedRoutes.some(route => {
        if (route === currentPath) return true;
        if (currentPath.startsWith(route + '/')) return true;
        return false;
      });

      console.log('[HMR] App router HMR triggered for routes:', affectedRoutes);
      console.log('[HMR] Current route affected:', isCurrentRouteAffected);

      const event = new CustomEvent('rari:app-router-rerender', {
        detail: {
          routePath,
          affectedRoutes,
          currentPath,
          preserveParams: true
        }
      });
      window.dispatchEvent(event);

      console.log('[HMR] Re-render triggered successfully');
    } catch (error) {
      console.error('[HMR] Failed to trigger re-render:', error);
      throw error;
    }
  }

  async function reloadAppRouterManifest() {
    if (typeof window === 'undefined') {
      return;
    }

    console.log('[HMR] Reloading app router manifest');

    try {
      const response = await fetch('/app-routes.json', {
        cache: 'no-cache',
        headers: {
          'Cache-Control': 'no-cache',
          'Pragma': 'no-cache'
        }
      });

      if (!response.ok) {
        throw new Error('Failed to fetch manifest: ' + response.status);
      }

      const manifest = await response.json();

      console.log('[HMR] Loaded updated manifest with ' + (manifest.routes?.length || 0) + ' routes');

      if (typeof globalThis !== 'undefined') {
        globalThis.__rari_app_routes_manifest = manifest;
      }

      const event = new CustomEvent('rari:app-router-manifest-updated', {
        detail: { manifest }
      });
      window.dispatchEvent(event);

      const currentPath = window.location.pathname;
      const currentRoute = manifest.routes?.find(r => {
        if (r.path === currentPath) return true;
        if (r.path.includes('[')) {
          const pattern = r.path.replace(/\\[([^\\]]+)\\]/g, '([^/]+)');
          const regex = new RegExp("^" + pattern + "$");
          return regex.test(currentPath);
        }
        return false;
      });

      if (currentRoute) {
        console.log('[HMR] Current route found in updated manifest:', currentRoute.path);
      } else {
        console.warn('[HMR] Current route not found in updated manifest, may need navigation update');
      }

    } catch (error) {
      console.error('[HMR] Failed to reload app router manifest:', error);
      throw error;
    }
  }

  async function invalidateRscCache(data) {
    const filePath = data?.filePath || data;

    rscClient.clearCache();

    if (typeof window !== 'undefined') {
      const event = new CustomEvent('rari:rsc-invalidate', {
        detail: { filePath }
      });
      window.dispatchEvent(event);
    }
  }

  function invalidateRSCCache(componentId) {
    rscClient.clearCache();
    console.log('[HMR] Cleared RSC cache for component: ' + componentId);
  }

  async function refetchCurrentRoute() {
    try {
      if (typeof window === 'undefined') {
        return;
      }

      const currentPath = window.location.pathname;

      const response = await fetch('/rsc' + currentPath, {
        headers: {
          'Accept': 'text/x-component',
          'X-RSC-Refetch': 'true',
        },
      });

      if (!response.ok) {
        throw new Error('Failed to refetch RSC: ' + response.status);
      }

      console.log('[HMR] Re-fetched RSC for route: ' + currentPath);

      if (typeof window !== 'undefined') {
        const event = new CustomEvent('rari:rsc-refetch-complete', {
          detail: { path: currentPath, timestamp: Date.now() }
        });
        window.dispatchEvent(event);
      }
    } catch (error) {
      console.error('[HMR] Failed to refetch RSC:', error);
    }
  }
}

class HMRErrorOverlay {
  constructor() {
    this.overlay = null;
    this.currentError = null;
  }

  show(error) {
    this.currentError = error;
    if (this.overlay) {
      this.updateOverlay(error);
    } else {
      this.createOverlay(error);
    }
  }

  hide() {
    if (this.overlay) {
      this.overlay.remove();
      this.overlay = null;
      this.currentError = null;
    }
  }

  isVisible() {
    return this.overlay !== null;
  }

  createOverlay(error) {
    this.overlay = document.createElement('div');
    this.overlay.id = 'rari-hmr-error-overlay';
    this.updateOverlay(error);
    document.body.appendChild(this.overlay);
  }

  updateOverlay(error) {
    if (!this.overlay) return;

    const fileInfo = error.filePath
      ? '<div style="margin-bottom: 1rem; padding: 0.75rem; background: rgba(0, 0, 0, 0.2); border-radius: 0.375rem; font-family: monospace; font-size: 0.875rem;"><strong>File:</strong> ' + this.escapeHtml(error.filePath) + '</div>'
      : '';

    const stackTrace = error.stack
      ? '<details style="margin-top: 1rem; cursor: pointer;"><summary style="font-weight: 600; margin-bottom: 0.5rem; user-select: none;">Stack Trace</summary><pre style="margin: 0; padding: 0.75rem; background: rgba(0, 0, 0, 0.2); border-radius: 0.375rem; overflow-x: auto; font-size: 0.875rem; line-height: 1.5; white-space: pre-wrap; word-break: break-word;">' + this.escapeHtml(error.stack) + '</pre></details>'
      : '';

    this.overlay.innerHTML = '<div style="position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.85); z-index: 999999; display: flex; align-items: center; justify-content: center; padding: 2rem; backdrop-filter: blur(4px);"><div style="background: #1e1e1e; color: #e0e0e0; border-radius: 0.5rem; padding: 2rem; max-width: 50rem; width: 100%; max-height: 90vh; overflow-y: auto; box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 10px 10px -5px rgba(0, 0, 0, 0.4); border: 1px solid #ef4444;"><div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 1.5rem;"><div style="display: flex; align-items: center; gap: 0.75rem;"><svg style="width: 2rem; height: 2rem; color: #ef4444;" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path></svg><h1 style="margin: 0; font-size: 1.5rem; font-weight: 700; color: #ef4444;">Build Error</h1></div><button onclick="document.getElementById(' + "'" + 'rari-hmr-error-overlay' + "'" + ').remove()" style="background: transparent; border: none; color: #9ca3af; cursor: pointer; padding: 0.5rem; border-radius: 0.25rem; transition: all 0.2s; font-size: 1.5rem; line-height: 1; width: 2rem; height: 2rem; display: flex; align-items: center; justify-content: center;" onmouseover="this.style.background=' + "'" + 'rgba(255,255,255,0.1)' + "'" + '; this.style.color=' + "'" + '#e0e0e0' + "'" + '" onmouseout="this.style.background=' + "'" + 'transparent' + "'" + '; this.style.color=' + "'" + '#9ca3af' + "'" + '">×</button></div>' + fileInfo + '<div style="margin-bottom: 1.5rem;"><h2 style="margin: 0 0 0.75rem 0; font-size: 1rem; font-weight: 600; color: #fca5a5;">Error Message:</h2><pre style="margin: 0; padding: 1rem; background: rgba(239, 68, 68, 0.1); border-left: 4px solid #ef4444; border-radius: 0.375rem; overflow-x: auto; font-family: monospace; font-size: 0.875rem; line-height: 1.5; white-space: pre-wrap; word-break: break-word; color: #fca5a5;">' + this.escapeHtml(error.message) + '</pre></div>' + stackTrace + '<div style="margin-top: 1.5rem; padding-top: 1.5rem; border-top: 1px solid #374151; display: flex; gap: 0.75rem; align-items: center;"><button onclick="window.location.reload()" style="padding: 0.625rem 1.25rem; background: #ef4444; color: white; border: none; border-radius: 0.375rem; cursor: pointer; font-weight: 600; font-size: 0.875rem; transition: all 0.2s;" onmouseover="this.style.background=' + "'" + '#dc2626' + "'" + '" onmouseout="this.style.background=' + "'" + '#ef4444' + "'" + '">Reload Page</button><button onclick="document.getElementById(' + "'" + 'rari-hmr-error-overlay' + "'" + ').remove()" style="padding: 0.625rem 1.25rem; background: #374151; color: #e0e0e0; border: none; border-radius: 0.375rem; cursor: pointer; font-weight: 600; font-size: 0.875rem; transition: all 0.2s;" onmouseover="this.style.background=' + "'" + '#4b5563' + "'" + '" onmouseout="this.style.background=' + "'" + '#374151' + "'" + '">Dismiss</button><span style="margin-left: auto; font-size: 0.75rem; color: #9ca3af;">' + new Date(error.timestamp).toLocaleTimeString() + '</span></div></div></div>';
  }

  escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }
}

let hmrErrorOverlay = null;

function getErrorOverlay() {
  if (!hmrErrorOverlay) {
    hmrErrorOverlay = new HMRErrorOverlay();
  }
  return hmrErrorOverlay;
}

if (import.meta.hot) {
  const overlay = getErrorOverlay();

  import.meta.hot.on('rari:hmr-error', (data) => {
    const message = data.msg || data.message;
    const filePath = data.file || data.filePath;
    const timestamp = data.t || data.timestamp;
    const errorCount = data.count || data.errorCount;
    const maxErrors = data.max || data.maxErrors;

    console.error('[HMR] Build error:', message);

    if (filePath) {
      console.error('[HMR] File:', filePath);
    }

    if (data.stack) {
      console.error('[HMR] Stack:', data.stack);
    }

    overlay.show({
      message: message,
      stack: data.stack,
      filePath: filePath,
      timestamp: timestamp,
    });

    if (errorCount && maxErrors) {
      if (errorCount >= maxErrors) {
        console.error('[HMR] Maximum error count (' + maxErrors + ') reached. Consider restarting the dev server if issues persist.');
      } else if (errorCount >= maxErrors - 2) {
        console.warn('[HMR] Error count: ' + errorCount + '/' + maxErrors + '. Approaching maximum error threshold.');
      }
    }
  });

  import.meta.hot.on('rari:hmr-error-cleared', (data) => {
    console.log('[HMR] Error cleared, build successful');
    overlay.hide();
  });

  import.meta.hot.on('vite:error', (data) => {
    console.error('[HMR] Vite error:', data);

    overlay.show({
      message: data.err?.message || 'Unknown Vite error',
      stack: data.err?.stack,
      filePath: data.err?.file,
      timestamp: Date.now(),
    });
  });

  console.log('[HMR] Error handling initialized');
}

export {
  createServerComponentWrapper,
  RscErrorComponent,
  rscClient
};
`
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

        server.hot.send('rari:app-router-updated', {
          type: 'rari-hmr',
          filePath: file,
          fileType,
        })
        return []
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

  return [mainPlugin, serverBuildPlugin]
}

export function defineRariConfig(
  config: UserConfig & { plugins?: Plugin[] },
): UserConfig {
  return {
    plugins: [rari(), ...(config.plugins || [])],
    ...config,
  }
}
