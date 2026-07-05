import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { describe, expect, it } from 'vite-plus/test'
import { discoverMdxRegistryEntries, generateMdxRegistryModule, isMdxRegistryModuleId } from '../../../packages/rari/src/vite/mdx-registry'
import { ModuleAnalysisCache } from '../../../packages/rari/src/vite/module-analysis-cache'

describe('mdx registry', () => {
  it('discovers only client components referenced in MDX content', () => {
    const projectRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-mdx-registry-'))
    const componentsDir = path.join(projectRoot, 'src', 'components')
    const contentDir = path.join(projectRoot, 'public', 'content', 'docs')
    fs.mkdirSync(componentsDir, { recursive: true })
    fs.mkdirSync(contentDir, { recursive: true })

    fs.writeFileSync(path.join(componentsDir, 'CodeBlock.tsx'), `'use client'\nexport default function CodeBlock() {}\n`)
    fs.writeFileSync(path.join(componentsDir, 'SearchBar.tsx'), `'use client'\nexport default function SearchBar() {}\n`)
    fs.writeFileSync(path.join(contentDir, 'page.mdx'), '<PageHeader />\n<CodeBlock language="ts" />\n')

    const cache = new ModuleAnalysisCache()
    const entries = discoverMdxRegistryEntries({
      projectRoot,
      componentsDir: 'src/components',
      contentDirs: ['public/content'],
      cache,
      componentScanDirs: [componentsDir],
    })

    expect(entries.map(entry => entry.name)).toEqual(['CodeBlock'])

    const moduleSource = generateMdxRegistryModule(entries)
    expect(moduleSource).toContain(`import { defineMdxComponents } from 'rari/mdx/define'`)
    expect(moduleSource).toContain('import CodeBlock from "/src/components/CodeBlock.tsx"')
    expect(moduleSource).not.toContain('SearchBar')

    fs.rmSync(projectRoot, { recursive: true, force: true })
  })

  it('stores production registry metadata without component imports', () => {
    const entries = [{
      name: 'CodeBlock',
      binding: 'CodeBlock',
      importPath: '/src/components/CodeBlock.tsx',
      moduleId: 'src/components/CodeBlock.tsx',
      client: true,
    }]

    const moduleSource = generateMdxRegistryModule(entries, { mode: 'production' })
    expect(moduleSource).toContain(`component: null`)
    expect(moduleSource).toContain(`id: "src/components/CodeBlock.tsx"`)
    expect(moduleSource).not.toContain('import CodeBlock')
  })

  it('matches only rari package MDX registry module ids', () => {
    expect(isMdxRegistryModuleId('rari/mdx/registry')).toBe(true)
    expect(isMdxRegistryModuleId('/app/node_modules/rari/dist/mdx/registry.mjs')).toBe(true)
    expect(isMdxRegistryModuleId('/repo/packages/rari/src/mdx/registry.ts')).toBe(true)

    expect(isMdxRegistryModuleId('./mdx/registry.ts')).toBe(false)
    expect(isMdxRegistryModuleId('/app/src/mdx/registry.ts')).toBe(false)
    expect(isMdxRegistryModuleId('../content/mdx/registry')).toBe(false)
  })
})
