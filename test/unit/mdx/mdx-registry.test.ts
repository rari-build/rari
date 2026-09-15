import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { ModuleAnalysisCache } from '@rari/vite/analysis/module-cache'
import {
  copyMdxContentDirsToDest,
  discoverMdxRegistryEntries,
  generateMdxRegistryModule,
  isMdxRegistryModuleId,
} from '@rari/vite/mdx/registry'
import { describe, expect, it } from 'vite-plus/test'

describe('mdx registry', () => {
  it('discovers only client components referenced in MDX content', () => {
    const projectRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-mdx-registry-'))
    const componentsDir = path.join(projectRoot, 'src', 'components')
    const contentDir = path.join(projectRoot, 'src', 'content', 'docs')
    fs.mkdirSync(componentsDir, { recursive: true })
    fs.mkdirSync(contentDir, { recursive: true })

    fs.writeFileSync(
      path.join(componentsDir, 'CodeBlock.tsx'),
      `'use client'\nexport default function CodeBlock() {}\n`,
    )
    fs.writeFileSync(
      path.join(componentsDir, 'SearchBar.tsx'),
      `'use client'\nexport default function SearchBar() {}\n`,
    )
    fs.writeFileSync(
      path.join(contentDir, 'page.mdx'),
      '<PageHeader />\n<CodeBlock language="ts" />\n',
    )

    const cache = new ModuleAnalysisCache()
    const entries = discoverMdxRegistryEntries({
      projectRoot,
      componentsDir: 'src/components',
      contentDirs: ['src/content'],
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
    const entries = [
      {
        name: 'CodeBlock',
        binding: 'CodeBlock',
        importPath: '/src/components/CodeBlock.tsx',
        moduleId: 'src/components/CodeBlock.tsx',
        client: true,
      },
    ]

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

  it('keeps earlier content dir files when later dirs conflict', () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-mdx-copy-'))
    const first = path.join(root, 'src', 'content')
    const second = path.join(root, 'content')
    const dest = path.join(root, 'dist', 'content')
    fs.mkdirSync(path.join(first, 'blog'), { recursive: true })
    fs.mkdirSync(path.join(second, 'blog'), { recursive: true })
    fs.mkdirSync(path.join(second, 'docs'), { recursive: true })
    fs.writeFileSync(path.join(first, 'blog', 'post.mdx'), 'from-src')
    fs.writeFileSync(path.join(second, 'blog', 'post.mdx'), 'from-content')
    fs.writeFileSync(path.join(second, 'docs', 'intro.mdx'), 'docs-only')

    copyMdxContentDirsToDest([first, second], dest)

    expect(fs.readFileSync(path.join(dest, 'blog', 'post.mdx'), 'utf-8')).toBe('from-src')
    expect(fs.readFileSync(path.join(dest, 'docs', 'intro.mdx'), 'utf-8')).toBe('docs-only')

    fs.rmSync(root, { recursive: true, force: true })
  })
})
