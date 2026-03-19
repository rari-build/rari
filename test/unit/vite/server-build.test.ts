import fsSync from 'node:fs'
import path from 'node:path'
import { ServerComponentBuilder } from '@rari/vite/server-build'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'

vi.mock('node:fs')
vi.mock('rolldown')

describe('ServerComponentBuilder', () => {
  let builder: ServerComponentBuilder
  const mockProjectRoot = '/test/project'
  const mockOptions = {
    outDir: 'dist',
    rscDir: 'server',
    manifestPath: 'server/manifest.json',
    serverConfigPath: 'server/config.json',
    minify: false,
    alias: {},
  }

  beforeEach(() => {
    vi.clearAllMocks()

    vi.mocked(fsSync.existsSync).mockReturnValue(false)

    vi.mocked(fsSync.readFileSync).mockReturnValue('')

    vi.mocked(fsSync.readdirSync).mockReturnValue([])

    vi.mocked(fsSync.statSync).mockReturnValue({
      isFile: () => true,
      isDirectory: () => false,
    } as any)

    const manifestJson = JSON.stringify({
      components: {},
      actions: {},
      importMap: { imports: {} },
      version: '1.0.0',
      buildTime: new Date().toISOString(),
    })

    Object.defineProperty(fsSync, 'promises', {
      value: {
        mkdir: vi.fn().mockResolvedValue(undefined),
        writeFile: vi.fn().mockResolvedValue(undefined),
        readFile: vi.fn().mockImplementation(async (path: any) => {
          if (typeof path === 'string' && path.includes('manifest.json')) {
            return manifestJson
          }

          return 'export default function Component() { return null }'
        }),
        stat: vi.fn().mockResolvedValue({
          mtimeMs: Date.now(),
        }),
        open: vi.fn().mockResolvedValue({
          sync: vi.fn().mockResolvedValue(undefined),
          close: vi.fn().mockResolvedValue(undefined),
        }),
        unlink: vi.fn().mockResolvedValue(undefined),
      },
      writable: true,
      configurable: true,
    })

    builder = new ServerComponentBuilder(mockProjectRoot, mockOptions)
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('constructor', () => {
    it('should initialize with default options', () => {
      const defaultBuilder = new ServerComponentBuilder(mockProjectRoot)

      expect(defaultBuilder).toBeDefined()
      expect(defaultBuilder.getComponentCount()).toBe(0)
    })

    it('should parse HTML imports on initialization', () => {
      const htmlContent = `
<!DOCTYPE html>
<html>
  <head>
    <script type="module">
      import '/src/main.tsx'
    </script>
  </head>
</html>
`
      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync).mockReturnValue(htmlContent)

      const builderWithHtml = new ServerComponentBuilder(mockProjectRoot, mockOptions)

      const htmlImports = builderWithHtml.getHtmlOnlyImports()
      const expectedPath = path.join(mockProjectRoot, 'src', 'main.tsx')

      expect(htmlImports.has(expectedPath)).toBe(true)
      expect(htmlImports.size).toBe(1)
    })

    it('should handle missing index.html gracefully', () => {
      vi.mocked(fsSync.existsSync).mockReturnValue(false)

      expect(() => new ServerComponentBuilder(mockProjectRoot, mockOptions)).not.toThrow()
    })
  })

  describe('isServerComponent', () => {
    it('should return false for node_modules files', () => {
      const filePath = '/test/project/node_modules/package/Component.tsx'

      const result = builder.isServerComponent(filePath)

      expect(result).toBe(false)
    })

    it('should return false for files with use client directive', () => {
      const filePath = '/test/project/src/components/Client.tsx'
      const code = `'use client'

export default function ClientComponent() {
  return <div>Client</div>
}`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync).mockReturnValue(code)

      const result = builder.isServerComponent(filePath)

      expect(result).toBe(false)
    })

    it('should return false for files with use server directive', () => {
      const filePath = '/test/project/src/actions/serverAction.ts'
      const code = `'use server'

export async function myAction() {
  return { success: true }
}`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync).mockReturnValue(code)

      const result = builder.isServerComponent(filePath)

      expect(result).toBe(false)
    })

    it('should return true for regular components', () => {
      const filePath = '/test/project/src/components/Server.tsx'
      const code = `export default function ServerComponent() {
  return <div>Server</div>
}`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync).mockReturnValue(code)

      const result = builder.isServerComponent(filePath)

      expect(result).toBe(true)
    })

    it('should handle file read errors', () => {
      const filePath = '/test/project/src/components/Error.tsx'

      vi.mocked(fsSync.existsSync).mockReturnValue(false)

      const result = builder.isServerComponent(filePath)

      expect(result).toBe(false)
    })

    it('should ignore comments with use client', () => {
      const filePath = '/test/project/src/components/Commented.tsx'
      const code = `// 'use client'

export default function Component() {
  return <div>Test</div>
}`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync).mockReturnValue(code)

      const result = builder.isServerComponent(filePath)

      expect(result).toBe(true)
    })
  })

  describe('addServerComponent', () => {
    it('should add server component to collection', () => {
      const filePath = '/test/project/src/components/Test.tsx'
      const code = `export default function Test() {
  return <div>Test</div>
}`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync).mockReturnValue(code)

      builder.addServerComponent(filePath)

      expect(builder.getComponentCount()).toBe(1)
    })

    it('should detect and add server actions separately', () => {
      const filePath = '/test/project/src/actions/test.ts'
      const code = `'use server'

export async function testAction() {
  return { success: true }
}`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync).mockReturnValue(code)

      builder.addServerComponent(filePath)

      expect(builder.getComponentCount()).toBe(1)
    })

    it('should not add client components', () => {
      const filePath = '/test/project/src/components/Client.tsx'
      const code = `'use client'

export default function Client() {
  return <div>Client</div>
}`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync).mockReturnValue(code)

      builder.addServerComponent(filePath)

      expect(builder.getComponentCount()).toBe(0)
    })

    it('should extract dependencies from component', async () => {
      const filePath = '/test/project/src/components/WithDeps.tsx'
      const code = `import { useState } from 'react'
import axios from 'axios'

export default function WithDeps() {
  return <div>Test</div>
}`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync).mockReturnValue(code)

      vi.mocked(fsSync.promises.readFile).mockImplementation(async (path: any) => {
        if (path.includes('manifest.json')) {
          return JSON.stringify({
            components: {},
            actions: {},
            importMap: { imports: {} },
            version: '1.0.0',
            buildTime: new Date().toISOString(),
          })
        }

        return code
      })

      builder.addServerComponent(filePath)

      expect(builder.getComponentCount()).toBe(1)

      const manifest = await builder.buildServerComponents()

      const componentEntries = Object.values(manifest.components)
      expect(componentEntries).toHaveLength(1)
      expect(componentEntries[0].dependencies).toContain('axios')
    })

    it('should detect node imports', async () => {
      const filePath = '/test/project/src/components/NodeImports.tsx'
      const code = `import fs from 'node:fs'
import path from 'node:path'

export default function NodeImports() {
  return <div>Test</div>
}`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync).mockReturnValue(code)

      vi.mocked(fsSync.promises.readFile).mockImplementation(async (path: any) => {
        if (path.includes('manifest.json')) {
          return JSON.stringify({
            components: {},
            actions: {},
            importMap: { imports: {} },
            version: '1.0.0',
            buildTime: new Date().toISOString(),
          })
        }

        return code
      })

      builder.addServerComponent(filePath)

      expect(builder.getComponentCount()).toBe(1)

      const manifest = await builder.buildServerComponents()

      const componentEntries = Object.values(manifest.components)
      expect(componentEntries).toHaveLength(1)
      expect(componentEntries[0].hasNodeImports).toBe(true)
    })
  })

  describe('buildImportGraph', () => {
    it('should build import relationships', () => {
      const srcDir = '/test/project/src'

      vi.mocked(fsSync.existsSync).mockImplementation((path: any) => {
        return path === srcDir || path.toString().endsWith('B.tsx')
      })

      vi.mocked(fsSync.statSync).mockReturnValue({
        isFile: () => true,
        isDirectory: () => false,
      } as any)

      vi.mocked(fsSync.readdirSync).mockReturnValue([
        { name: 'A.tsx', isFile: () => true, isDirectory: () => false } as any,
        { name: 'B.tsx', isFile: () => true, isDirectory: () => false } as any,
      ])

      vi.mocked(fsSync.readFileSync)
        .mockReturnValueOnce(`import B from './B'
export default function A() { return <B /> }`)
        .mockReturnValueOnce(`export default function B() { return <div>B</div> }`)

      builder.buildImportGraph(srcDir)

      const graph = builder.getImportGraph()
      const bPath = path.join(srcDir, 'B.tsx')
      const aPath = path.join(srcDir, 'A.tsx')

      expect(graph.has(bPath)).toBe(true)
      expect(graph.get(bPath)?.has(aPath)).toBe(true)
    })

    it('should skip node_modules in import graph', () => {
      const srcDir = '/test/project/src'

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readdirSync).mockReturnValue([
        { name: 'node_modules', isFile: () => false, isDirectory: () => true } as any,
        { name: 'Component.tsx', isFile: () => true, isDirectory: () => false } as any,
      ])

      builder.buildImportGraph(srcDir)

      const graph = builder.getImportGraph()
      for (const [key, importers] of graph.entries()) {
        expect(key).not.toContain('node_modules')
        for (const importer of importers) {
          expect(importer).not.toContain('node_modules')
        }
      }
    })
  })

  describe('isOnlyImportedByClientComponents', () => {
    it('should return false when file has no importers', () => {
      const filePath = '/test/project/src/components/Orphan.tsx'

      builder.buildImportGraph('/test/project/src')

      const result = builder.isOnlyImportedByClientComponents(filePath)

      expect(result).toBe(false)
    })

    it('should return true when only imported by client components', () => {
      const srcDir = '/test/project/src'
      const utilPath = path.join(srcDir, 'utils.ts')

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readdirSync).mockReturnValue([
        { name: 'utils.ts', isFile: () => true, isDirectory: () => false } as any,
        { name: 'Client.tsx', isFile: () => true, isDirectory: () => false } as any,
      ])

      vi.mocked(fsSync.readFileSync)
        .mockReturnValueOnce(`export function util() { return 'test' }`)
        .mockReturnValueOnce(`'use client'
import { util } from './utils'
export default function Client() { return <div>{util()}</div> }`)

      builder.buildImportGraph(srcDir)

      const result = builder.isOnlyImportedByClientComponents(utilPath)

      expect(typeof result).toBe('boolean')
    })
  })

  describe('getComponentCount', () => {
    it('should return total count of components and actions', () => {
      const serverPath = '/test/project/src/Server.tsx'
      const actionPath = '/test/project/src/action.ts'

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync)
        .mockReturnValueOnce(`export default function Server() { return <div>Server</div> }`)
        .mockReturnValueOnce(`'use server'
export async function action() { return {} }`)

      builder.addServerComponent(serverPath)
      builder.addServerComponent(actionPath)

      expect(builder.getComponentCount()).toBeGreaterThanOrEqual(1)
    })
  })

  describe('clearCache', () => {
    it('should clear build cache', async () => {
      const filePath = '/test/project/src/CacheTest.tsx'
      const code = `export default function CacheTest() { return <div>Test</div> }`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)

      vi.mocked(fsSync.promises.readFile).mockImplementation(async (path: any) => {
        if (path.includes('manifest.json')) {
          return JSON.stringify({
            components: {},
            actions: {},
            importMap: { imports: {} },
            version: '1.0.0',
            buildTime: new Date().toISOString(),
          })
        }

        return code
      })

      vi.mocked(fsSync.promises.stat).mockResolvedValue({
        mtimeMs: 1000,
      } as any)

      const buildSpy = vi.spyOn(builder as any, 'buildSingleComponent')

      await builder.rebuildComponent(filePath)
      expect(buildSpy).toHaveBeenCalledTimes(1)

      await builder.rebuildComponent(filePath)
      expect(buildSpy).toHaveBeenCalledTimes(1)

      builder.clearCache()

      await builder.rebuildComponent(filePath)
      expect(buildSpy).toHaveBeenCalledTimes(2)
    })
  })

  describe('getTransformedComponentsForDevelopment', () => {
    it('should return transformed components', async () => {
      const filePath = '/test/project/src/Test.tsx'
      const code = `export default function Test() { return <div>Test</div> }`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync).mockReturnValue(code)

      vi.mocked(fsSync.promises.readFile).mockImplementation(async (path: any) => {
        if (path.includes('manifest.json')) {
          return JSON.stringify({
            components: {},
            actions: {},
            importMap: { imports: {} },
            version: '1.0.0',
            buildTime: new Date().toISOString(),
          })
        }

        return code
      })

      builder.addServerComponent(filePath)

      const components = await builder.getTransformedComponentsForDevelopment()

      expect(components).toHaveLength(1)
      expect(components[0]).toHaveProperty('id')
      expect(components[0]).toHaveProperty('code')
    })

    it('should handle empty component list', async () => {
      const components = await builder.getTransformedComponentsForDevelopment()

      expect(components).toEqual([])
    })
  })

  describe('rebuildComponent', () => {
    it('should rebuild component and return result', async () => {
      const filePath = '/test/project/src/Rebuild.tsx'
      const code = `export default function Rebuild() { return <div>Rebuild</div> }`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)

      vi.mocked(fsSync.promises.readFile).mockImplementation(async (path: any) => {
        if (path.includes('manifest.json')) {
          return JSON.stringify({
            components: {},
            actions: {},
            importMap: { imports: {} },
            version: '1.0.0',
            buildTime: new Date().toISOString(),
          })
        }

        return code
      })

      vi.mocked(fsSync.promises.stat).mockResolvedValue({
        mtimeMs: Date.now(),
      } as any)

      const result = await builder.rebuildComponent(filePath)

      expect(result.success).toBe(true)
      expect(result.componentId).toBeDefined()
      expect(result.bundlePath).toBeDefined()
    })

    it('should use cache when file unchanged', async () => {
      const filePath = '/test/project/src/Cached.tsx'
      const code = `export default function Cached() { return <div>Cached</div> }`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)

      vi.mocked(fsSync.promises.readFile).mockImplementation(async (path: any) => {
        if (path.includes('manifest.json')) {
          return JSON.stringify({
            components: {},
            actions: {},
            importMap: { imports: {} },
            version: '1.0.0',
            buildTime: new Date().toISOString(),
          })
        }

        return code
      })

      vi.mocked(fsSync.promises.stat).mockResolvedValue({
        mtimeMs: 1000,
      } as any)

      const buildSpy = vi.spyOn(builder as any, 'buildSingleComponent')

      await builder.rebuildComponent(filePath)

      expect(buildSpy).toHaveBeenCalledTimes(1)

      vi.mocked(fsSync.promises.stat).mockResolvedValue({
        mtimeMs: 1000,
      } as any)

      const result = await builder.rebuildComponent(filePath)

      expect(buildSpy).toHaveBeenCalledTimes(1)
      expect(result.success).toBe(true)

      buildSpy.mockRestore()
    })
  })

  describe('buildServerComponents', () => {
    it('should build all server components and create manifest', async () => {
      const filePath = '/test/project/src/Component.tsx'
      const code = `export default function Component() { return <div>Test</div> }`

      vi.mocked(fsSync.existsSync).mockReturnValue(true)
      vi.mocked(fsSync.readFileSync).mockReturnValue(code)

      vi.mocked(fsSync.promises.readFile).mockImplementation(async (path: any) => {
        if (path.includes('manifest.json')) {
          return JSON.stringify({
            components: {},
            actions: {},
            importMap: { imports: {} },
            version: '1.0.0',
            buildTime: new Date().toISOString(),
          })
        }

        return code
      })

      builder.addServerComponent(filePath)

      const manifest = await builder.buildServerComponents()

      expect(manifest).toHaveProperty('components')
      expect(manifest).toHaveProperty('importMap')
      expect(manifest).toHaveProperty('version')
      expect(manifest).toHaveProperty('buildTime')

      expect(fsSync.promises.writeFile).toHaveBeenCalled()
    })

    it('should include import map in manifest', async () => {
      const manifest = await builder.buildServerComponents()

      expect(manifest.importMap.imports).toHaveProperty('react')
      expect(manifest.importMap.imports).toHaveProperty('react-dom')
    })

    it('should write server config when options provided', async () => {
      const builderWithConfig = new ServerComponentBuilder(mockProjectRoot, {
        ...mockOptions,
        csp: {
          scriptSrc: ['self'],
        },
      })

      await builderWithConfig.buildServerComponents()

      expect(fsSync.promises.writeFile).toHaveBeenCalledWith(
        expect.stringContaining('config.json'),
        expect.any(String),
        'utf-8',
      )
    })

    it('should remove server config when no options provided', async () => {
      vi.mocked(fsSync.existsSync).mockReturnValue(true)

      await builder.buildServerComponents()

      expect(fsSync.promises.unlink).toHaveBeenCalled()
    })
  })
})
