import type { Buffer } from 'node:buffer'
import type { Plugin, UserConfig } from 'rolldown-vite'
import type { ServerBuildOptions } from './server-build'
import { spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'
import * as acorn from 'acorn'
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

  function isServerComponent(filePath: string): boolean {
    if (filePath.includes('node_modules')) {
      return false
    }

    if (filePath.includes('/.rari/') || filePath.includes('\\.rari\\')) {
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

      const isInFunctionsDir
        = filePath.includes('/functions/') || filePath.includes('\\\\functions\\\\')

      if (isInFunctionsDir) {
        return false
      }

      const hasClientDirective = hasTopLevelDirective(code, 'use client')
      if (hasClientDirective) {
        return false
      }

      const hasClientPatterns
        = code.includes('react-dom/client')
        || code.includes('ReactDOM.createRoot')
        || code.includes('document.')
        || code.includes('window.')
        || code.includes('localStorage')
        || code.includes('sessionStorage')
        || code.includes('navigator.')
        || code.includes('history.')
        || (code.includes('rari/client') && (code.includes('useRouter') || code.includes('RouterProvider')))
        || /addEventListener\s*\(/.test(code)
        || /removeEventListener\s*\(/.test(code)

      if (hasClientPatterns) {
        return false
      }

      const hasNodeImports
        = code.includes('from \'node:')
        || code.includes('from "node:')
        || code.includes('from \'fs\'')
        || code.includes('from "fs"')
        || code.includes('from \'path\'')
        || code.includes('from "path"')
        || code.includes('from \'crypto\'')
        || code.includes('from "crypto"')

      const hasAsyncDefaultExport = /export\s+default\s+async\s+function/.test(code)

      const hasServerOnlyPatterns
        = code.includes('readFileSync')
        || code.includes('writeFileSync')
        || code.includes('process.env')
        || code.includes('await fetch')

      const hasReactImport = code.includes('react') || code.includes('React')
      const hasJSX = /<[A-Z]/.test(code) || code.includes('jsx') || code.includes('tsx')

      const hasDefaultExport = /export\s+default/.test(code)
      const hasFunctionDeclaration = /function\s+\w+/.test(code) || /const\s+\w+\s*=\s*\([^)]*\)\s*=>/.test(code)
      const hasReactComponent = (hasReactImport || hasJSX) && (hasDefaultExport || hasFunctionDeclaration)

      const isServerComponent
        = hasNodeImports
        || hasAsyncDefaultExport
        || hasServerOnlyPatterns
        || (hasReactComponent && !hasClientDirective)

      return isServerComponent
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
    const isInFunctionsDir = id.includes('/functions/') || id.includes('\\\\functions\\\\')

    if (isInFunctionsDir && !hasTopLevelDirective(code, 'use server')) {
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

// HMR acceptance for server components
if (import.meta.hot) {
  import.meta.hot.accept(() => {
    // Server component updated, no need to reload
  });
}`

    return newCode
  }

  function transformClientModule(code: string, id: string): string {
    const isServerFunction = hasTopLevelDirective(code, 'use server')
    const isServerComp = isServerComponent(id)

    if (isServerFunction || isServerComp) {
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

  function transformClientModuleForClient(code: string, id: string): string {
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

      const discoverAndRegisterComponents = async () => {
        try {
          const { ServerComponentBuilder, scanDirectory } = await import(
            './server-build',
          )

          const builder = new ServerComponentBuilder(projectRoot, {
            outDir: 'temp',
            serverDir: 'server',
            manifestPath: 'server-manifest.json',
          })

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
            RUST_LOG: process.env.RUST_LOG || 'info',
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
          const { ServerComponentBuilder } = await import('./server-build')
          const builder = new ServerComponentBuilder(projectRoot, {
            outDir: 'temp',
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
              const isAppRouterComponent = component.id.startsWith('app/')
              if (isAppRouterComponent) {
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

      if (id === 'virtual:rsc-client-components') {
        return id
      }

      if (id === 'virtual:rari-entry-client') {
        return id
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


        const clientComponentsArray = Array.from(allClientComponents).filter(componentPath => {
          try {
            const code = fs.readFileSync(componentPath, 'utf-8')
            const lines = code.split('\n')
            for (const line of lines) {
              const trimmed = line.trim()
              if (!trimmed || trimmed.startsWith('//') || trimmed.startsWith('/*')) {
                continue
              }
              if (trimmed === "'use client'" || trimmed === '"use client"' ||
                trimmed === "'use client';" || trimmed === '"use client";') {
                return true
              }
              break
            }
            return false
          } catch {
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

        return `
import React from 'react';
import { createRoot } from 'react-dom/client';

${imports}

if (typeof globalThis.__clientComponents === 'undefined') {
  globalThis.__clientComponents = {};
}
if (typeof globalThis.__clientComponentPaths === 'undefined') {
  globalThis.__clientComponentPaths = {};
}

${registrations}

export async function renderApp() {
  const rootElement = document.getElementById('root');
  if (!rootElement) {
    console.error('[Rari] Root element not found');
    return;
  }

  try {
    const rariServerUrl = window.location.origin.includes(':3001')
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

    const { element } = parseRscWireFormat(rscWireFormat);

    const root = createRoot(rootElement);
    root.render(element);
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

function parseRscWireFormat(wireFormat) {
  const lines = wireFormat.trim().split('\\n');
  let rootElement = null;
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
        }
      }
    } catch (e) {
      console.error('[Rari] Failed to parse RSC line:', line, e);
    }
  }

  if (!rootElement) {
    throw new Error('No root element found in RSC wire format');
  }

  return { element: rootElement, modules };
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

      if (id === 'virtual:rsc-integration') {
        return `
import { useState, useEffect, Suspense, createElement, isValidElement, cloneElement } from 'react';

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

let createFromFetch = null;
let createFromReadableStream = null;
let rscClientLoadPromise = null;

async function loadRscClient() {
  if (rscClientLoadPromise) {
    return rscClientLoadPromise;
  }

  rscClientLoadPromise = (async () => {
    try {
      const rscModule = await import('react-dom/client');
      createFromFetch = rscModule.createFromFetch;
      createFromReadableStream = rscModule.createFromReadableStream;

      if (typeof createFromReadableStream !== 'function') {
        createFromReadableStream = null;
      }
      if (typeof createFromFetch !== 'function') {
        createFromFetch = null;
      }

      return rscModule;
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

    const hasServerPattern = (
      filePath.includes('/functions/') ||
      filePath.includes('\\\\functions\\\\')
    );

    return hasServerPattern;
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
    if (data?.path && isServerComponent(data.path)) {
      await invalidateRscCache({ filePath: data.path, forceReload: false });
    }
  });



  async function invalidateRscCache(data) {
    const filePath = data?.filePath || data;

    const waitForServerReady = async () => {
      for (let i = 0; i < 20; i++) {
        try {
          const response = await fetch('/_rsc_status');
          if (response.ok) {
            return true;
          }
        } catch (e) {
          // Server not ready yet
        }
        await new Promise(resolve => setTimeout(resolve, 100));
      }
      return false;
    };

    const serverReady = await waitForServerReady();
    if (serverReady) {
      rscClient.clearCache();

      try {
        await fetch('/api/rsc/hmr-register', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            file_path: filePath
          })
        });

        await new Promise(resolve => setTimeout(resolve, 300));
      } catch (error) {
        // Fallback to existing timeout-based approach
      }

      if (typeof window !== 'undefined') {
        const event = new CustomEvent('rari:rsc-invalidate', {
          detail: { filePath }
        });
        window.dispatchEvent(event);
      }
    } else {
      setTimeout(() => {
        rscClient.clearCache();
        if (typeof window !== 'undefined') {
          const event = new CustomEvent('rari:rsc-invalidate', {
            detail: { filePath }
          });
          window.dispatchEvent(event);
        }
      }, 1200);
    }
  }
}

export {
  createServerComponentWrapper,
  RscErrorComponent,
  rscClient
};
`
      }

      if (id === 'virtual:rsc-client-components') {
        return `
export {}
`
      }
    },

    handleHotUpdate({ file, server }) {
      const isReactFile = /\.(?:tsx?|jsx?)$/.test(file)
      const isServerComp = isServerComponent(file)

      if (isReactFile && isServerComp) {
        server.hot.send('rari:server-component-updated', {
          type: 'rari-hmr',
          path: file,
        })
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
