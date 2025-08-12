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

export function rari(options: RariOptions = {}): Plugin[] {
  const componentTypeCache = new Map<string, 'client' | 'server' | 'unknown'>()
  const serverComponents = new Set<string>()
  const clientComponents = new Set<string>()
  let rustServerProcess: any = null

  const serverImportedClientComponents = new Set<string>()

  function isServerComponent(filePath: string): boolean {
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

      const serverDirectives = ['\'use server\'', '"use server"']

      const trimmedCode = code.trim()

      const hasServerDirective = serverDirectives.some(
        directive =>
          trimmedCode.startsWith(directive) || code.includes(directive),
      )

      const isInFunctionsDir
        = filePath.includes('/functions/') || filePath.includes('\\functions\\')
      const hasServerFunctionSignature
        = (code.includes('export async function')
          || code.includes('export function'))
        && code.includes('\'use server\'')

      if (
        hasServerDirective
        || (isInFunctionsDir && hasServerFunctionSignature)
      ) {
        return true
      }

      return false
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
        }
      }

      return exportedNames
    }
    catch {
      return []
    }
  }

  function transformServerModule(code: string, id: string): string {
    if (!code.includes('use server')) {
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
          newCode += `\n// Register server reference for default export (function declaration)\n`
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
    if (code.includes('use server')) {
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

    if (!code.includes('use client')) {
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
    if (!code.includes('use client')) {
      return code
    }

    const exportedNames = parseExportedNames(code)
    if (exportedNames.length === 0) {
      return code
    }

    let newCode = code.replace(/^['"]use client['"];?\s*$/gm, '')
    newCode += '\n\n// Client component registration\n'
    newCode
      += 'import { registerClientComponent } from "virtual:rsc-integration";\n'

    for (const name of exportedNames) {
      if (name === 'default') {
        const defaultExportMatch = code.match(
          /export\s+default\s+function\s+(\w+)/,
        )
        const functionName = defaultExportMatch ? defaultExportMatch[1] : null

        if (functionName) {
          newCode += `\nif (typeof registerClientComponent === 'function') {\n`
          newCode += `  registerClientComponent(${functionName}, ${JSON.stringify(path.relative(process.cwd(), id))}, ${JSON.stringify(name)});\n`
          newCode += `}\n`
        }
      }
      else {
        newCode += `\nif (typeof registerClientComponent === 'function') {\n`
        newCode += `  registerClientComponent(${name}, ${JSON.stringify(path.relative(process.cwd(), id))}, ${JSON.stringify(name)});\n`
        newCode += `}\n`
      }
    }

    return newCode
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

        if (
          typeof config.environments.client.build.rollupOptions.input
          === 'object'
          && !Array.isArray(config.environments.client.build.rollupOptions.input)
        ) {
          (
            config.environments.client.build.rollupOptions.input as Record<
              string,
              string
            >
          )['client-components'] = 'virtual:rsc-client-components'
        }
      }

      return config
    },

    transform(code, id) {
      if (!/\.(?:tsx?|jsx?)$/.test(id)) {
        return null
      }

      const environment = (this as any).environment

      if (code.includes('\'use client\'') || code.includes('"use client"')) {
        componentTypeCache.set(id, 'client')
        clientComponents.add(id)

        if (
          environment
          && (environment.name === 'rsc' || environment.name === 'ssr')
        ) {
          return transformClientModule(code, id)
        }
        else {
          return transformClientModuleForClient(code, id)
        }
      }

      if (code.includes('\'use server\'') || code.includes('"use server"')) {
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

          if (code.includes('\'use server\'') || code.includes('"use server"')) {
            clientTransformedCode = `// HMR acceptance for server component
if (import.meta.hot) {
  import.meta.hot.accept();
}

${clientTransformedCode}`
          }

          return clientTransformedCode
        }
      }

      const cachedType = componentTypeCache.get(id)
      if (cachedType === 'server') {
        return transformServerModule(code, id)
      }

      if (cachedType === 'client') {
        return transformClientModule(code, id)
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
            './server-build'
          )

          const builder = new ServerComponentBuilder(projectRoot, {
            outDir: 'temp',
            serverDir: 'server',
            manifestPath: 'server-manifest.json',
          })

          const srcDir = path.join(projectRoot, 'src')

          if (fs.existsSync(srcDir)) {
            scanDirectory(srcDir, builder)
          }

          const components
            = await builder.getTransformedComponentsForDevelopment()

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
          '../platform'
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
            console.warn(`${output}`)
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
            console.warn(`Rari server stopped by signal ${signal}`)
          }
          else if (code === 0) {
            console.warn('Rari server stopped successfully')
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

// Module map for React Server Components
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
        if (process.env.NODE_ENV === 'production') {
          return `
export function createServerComponentWrapper() { return () => null }
export function registerClientComponent() {}
export const RscErrorComponent = () => null
export const rscClient = { fetchServerComponent: async () => null }
`
        }
        return `
import React, { useState, useEffect, Suspense } from 'react';

// Client component registration for RSC system compatibility
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

  // Register in global registry for RSC traversal
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
      console.warn('[RARI] Failed to register client component with server:', error);
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
        console.warn('createFromReadableStream is not available in react-dom/client');
        createFromReadableStream = null;
      }
      if (typeof createFromFetch !== 'function') {
        console.warn('createFromFetch is not available in react-dom/client');
        createFromFetch = null;
      }

      return rscModule;
    } catch (error) {
      console.warn('Failed to load react-dom/client RSC functions:', error);
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
    const cacheKey = componentId + ':' + JSON.stringify(props);

    if (this.componentCache.has(cacheKey)) {
      return this.componentCache.get(cacheKey);
    }

    if (this.config.enableStreaming) {
      const result = await this.fetchServerComponentStreamV2(componentId, props);
      this.componentCache.set(cacheKey, result);
      return result;
    }

    const encodedProps = encodeURIComponent(JSON.stringify(props));
    const cacheBuster = Date.now();
    const fetchUrl = '/rsc/render/' + componentId + '?props=' + encodedProps + '&_t=' + cacheBuster;

    try {
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

      let result;

      try {
        result = await this.processRscResponseManually(response);
      } catch (manualError) {
        try {
          result = await this.processRscResponse(response);
        } catch (error) {
          console.error('Both RSC parsing methods failed:', { manualError, error });
          throw new Error('Failed to parse RSC response: ' + manualError.message);
        }
      }

      this.componentCache.set(cacheKey, result);
      return result;
    } catch (error) {
      throw new Error('Failed to fetch server component ' + componentId + ': ' + error.message);
    }
  }

  async fetchServerComponentStreamV2(componentId, props = {}) {
    await loadRscClient();

    const endpoints = [
      '/api/rsc/stream-v2',
      'http://127.0.0.1:3000/api/rsc/stream-v2',
    ];
    let response = null;
    let lastError = null;
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
          response = r;
          break;
        }
        lastError = new Error('HTTP ' + r.status + ': ' + (await r.text()));
      } catch (e) {
        lastError = e;
      }
    }
    if (!response) {
      throw lastError || new Error('Failed to reach stream-v2 endpoint');
    }

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error('Server responded with ' + response.status + ': ' + errorText);
    }

    const stream = response.body;
    if (!stream) {
      throw new Error('No ReadableStream from stream-v2 response');
    }

    if (createFromReadableStream) {
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
        console.warn('Failed to use createFromReadableStream:', error);
      }
    }

    const reader = stream.getReader();
    const decoder = new TextDecoder();
    let content = '';

    const convertRscToReact = (element) => {
      if (!React) {
        console.warn('React not available for RSC conversion');
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

          if (type === 'react.suspense') {

            const suspenseWrapper = React.createElement('div',
              {
                'data-boundary-id': props?.boundaryId,
                boundaryId: props?.boundaryId,
                'data-suspense-boundary': true
              },
              convertRscToReact(props?.fallback || props?.children)
            );

            return suspenseWrapper;
          }

          const processedProps = props ? { ...props } : {};
          if (props?.children) {
            processedProps.children = convertRscToReact(props.children);
          }

          if (typeof type === 'string') {
            if (type.includes('.tsx#') || type.includes('.jsx#')) {
              const clientComponent = getClientComponent(type);
              if (clientComponent) {
                const reactElement = React.createElement(clientComponent, key ? { ...processedProps, key } : processedProps);
                return reactElement;
              } else {
                console.warn('Failed to resolve client component:', type);
                return null;
              }
            } else {
              const reactElement = React.createElement(type, key ? { ...processedProps, key } : processedProps);
              return reactElement;
            }
          } else {
            console.warn('Unknown RSC element type:', type);
          }
        }

        return element.map((child, index) => {
          const converted = convertRscToReact(child);
          return converted;
        });
      }

      if (typeof element === 'object') {
        console.warn('Unexpected object in RSC conversion:', element);
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
              } else {
                try {
                  const parsed = JSON.parse(content);

                  if (Array.isArray(parsed) && parsed.length >= 4) {
                    const [marker, selector, key, props] = parsed;

                    if (marker === '$' && typeof selector === 'string' && selector.startsWith('boundary_') && props && props.resolved) {
                      const resolvedContent = convertRscToReact(props.children);
                      boundaryUpdates.set(selector, resolvedContent);

                      if (streamingComponent) {
                        streamingComponent.updateBoundary(selector, resolvedContent);
                      } else {
                        console.warn('No streamingComponent available for update');
                      }
                      continue;
                    }
                  }

                  if (rowId === '2') {
                    initialContent = convertRscToReact(parsed);
                  }
                }
              }
            } catch (e) {
              console.warn('Failed to parse stream line:', line, e);
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
      const [renderTrigger, setRenderTrigger] = React.useState(0);

      React.useEffect(() => {
        streamingComponent = {
          updateBoundary: (boundaryId, resolvedContent) => {
            boundaryUpdates.set(boundaryId, resolvedContent);
            setRenderTrigger(prev => {
              return prev + 1;
            });
          }
        };

        return () => {
          streamingComponent = null;
        };
      }, []);

      const renderWithBoundaryUpdates = (element) => {
        if (!element) return null;

        if (React.isValidElement(element)) {
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
              return React.cloneElement(element, { ...element.props, children: updatedChildren });
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
      _rscPromise: Promise.resolve(React.createElement(StreamingWrapper)),
      readRoot() {
        return Promise.resolve(React.createElement(StreamingWrapper));
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

    const elementKeys = Array.from(elements.keys());

    for (const key of elementKeys) {
      const element = elements.get(key);
      if (Array.isArray(element) && element.length >= 2 && element[0] === '$') {
        const [marker, type] = element;
        if (typeof type === 'string' && (type === 'root_boundary' || type.startsWith('boundary_'))) {
          rootElement = element;
          break;
        }
      }
    }

    if (!rootElement) {
      const sortedKeys = elementKeys.sort((a, b) => parseInt(b) - parseInt(a));
      for (const key of sortedKeys) {
        const element = elements.get(key);
        if (Array.isArray(element) && element[0] === '$') {
          rootElement = element;
          break;
        }
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

        if (typeof type === 'string' && (type === 'root_boundary' || type.startsWith('boundary_'))) {
          const children = props && typeof props === 'object' ? props.children : null;
          return this.reconstructElementFromRscData(children, modules);
        }

        let actualType = type;

        if (typeof type === 'string' && type.includes('#')) {
          const clientComponent = getClientComponent(type);
          if (clientComponent) {
            actualType = clientComponent;
          } else {
            actualType = ({ children, ...restProps }) => React.createElement(
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
              React.createElement('small', { style: { color: '#c00' } },
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
              actualType = ({ children, ...restProps }) => React.createElement(
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
                React.createElement('small', { style: { color: '#c00' } },
                  'Missing Client Component: ' + moduleData.name + ' (' + moduleData.id + ')'
                ),
                children
              );
            }
          }
        }

        const processedProps = props ? this.processPropsRecursively(props, modules) : {};

        return React.createElement(actualType, { key, ...processedProps });
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
            }).filter(child => child !== null && child !== undefined); // Remove null/undefined children

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
  return React.createElement('div',
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
    React.createElement('h3', { style: { margin: '0 0 8px 0', color: '#c00' } }, 'RSC Error'),
    React.createElement('p', { style: { margin: '0 0 8px 0' } }, error),
    details && React.createElement('details',
      { style: { marginTop: '8px' } },
      React.createElement('summary', { style: { cursor: 'pointer' } }, 'Error Details'),
      React.createElement('pre',
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
    return fallback || React.createElement('div', { className: 'rsc-loading' },
      'Loading ' + componentId + '...'
    );
  }

  if (error) {
    return React.createElement(RscErrorComponent, {
      error: 'Error loading component',
      details: { message: error.message, componentId }
    });
  }

    if (data) {
    if (data._isRscResponse) {
      return React.createElement(Suspense,
        { fallback: fallback || React.createElement('div', null, 'Loading...') },
        data.readRoot()
      );
    } else if (data) {
      return data;
    }
  }

  return React.createElement(RscErrorComponent, {
    error: 'No data received for component: ' + componentId,
    details: { componentId, dataType: typeof data, hasData: !!data }
  });
}

function createServerComponentWrapper(componentName, importPath) {
  // Use a global refresh counter to force re-mounting when components change
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

    // Force re-mount when component is invalidated
    useEffect(() => {
      const handleRscInvalidate = (event) => {
        const detail = event.detail;
        if (detail && detail.filePath && isServerComponent(detail.filePath)) {
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

    return React.createElement(Suspense, {
      fallback: React.createElement('div', null, 'Loading ' + componentName + '...')
    }, React.createElement(ServerComponentWrapper, {
      key: componentName + '-' + mountKey, // Force re-mount with key change
      componentId: componentName,
      props: props
    }));
  };

  ServerComponent.displayName = 'ServerComponent(' + componentName + ')';

  return function(props) {
    return React.createElement(ServerComponent, props);
  };
}

export const fetchServerComponent = (componentId, props) =>
  rscClient.fetchServerComponent(componentId, props);

// Helper function to check if a file is a server component (client-side)
function isServerComponent(filePath) {
  // Simple client-side check based on file path patterns
  return filePath && (
    filePath.includes('ServerWithClient') ||
    filePath.includes('server') ||
    filePath.includes('Server')
  );
}

// HMR support for RSC cache invalidation
if (import.meta.hot) {
  // Listen for Vite's beforeFullReload event for server components
  import.meta.hot.on('vite:beforeFullReload', async (data) => {
    if (data?.path && isServerComponent(data.path)) {
      // Immediately invalidate cache and trigger re-registration before reload
      await invalidateRscCache({ filePath: data.path, forceReload: true });
    }
  });



  // Helper function to invalidate RSC cache and trigger component re-registration
  async function invalidateRscCache(data) {
    const filePath = data?.filePath || data;

    // Wait for server to be ready
    const waitForServerReady = async () => {
      for (let i = 0; i < 20; i++) { // Try for up to 2 seconds
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
      // Clear client-side RSC cache immediately
      rscClient.clearCache();

      // Trigger immediate server component re-registration
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

        // Wait a bit for the server to re-register the component
        // The server now immediately reads and re-registers the component
        await new Promise(resolve => setTimeout(resolve, 300));
      } catch (error) {
        // Fallback to existing timeout-based approach
      }

      // Trigger re-render of active server components
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
  React,
  useState,
  useEffect,
  Suspense,
  createServerComponentWrapper,
  RscErrorComponent,
  rscClient
};
`
      }

      if (id === 'virtual:rsc-client-components') {
        if (process.env.NODE_ENV === 'production') {
          return `
export function registerClientComponent() {}
`
        }
        const srcDir = path.join(process.cwd(), 'src')
        const scannedClientComponents = scanForClientComponents(srcDir)

        const allClientComponents = new Set([
          ...clientComponents,
          ...scannedClientComponents,
          ...serverImportedClientComponents,
        ])

        const imports: string[] = []
        const registrations: string[] = []

        Array.from(allClientComponents).forEach((componentPath, index) => {
          const relativePath = path.relative(process.cwd(), componentPath)
          const componentName = `ClientComponent${index}`

          imports.push(
            `import ${componentName} from ${JSON.stringify(componentPath)};`,
          )
          registrations.push(
            `registerClientComponent(${componentName}, ${JSON.stringify(relativePath)}, "default");`,
          )
        })

        return `
import { registerClientComponent } from "virtual:rsc-integration";

${imports.join('\n')}

${registrations.join('\n')}
`
      }
    },

    handleHotUpdate({ file, server }) {
      if (/\.(?:tsx?|jsx?)$/.test(file) && isServerComponent(file)) {
        server.hot.send('vite:beforeFullReload', {
          type: 'full-reload',
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
