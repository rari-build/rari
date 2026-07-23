import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { transformDefineMdxComponents } from '@rari/vite/transform/mdx-components'
import { describe, expect, it } from 'vite-plus/test'

describe('transformDefineMdxComponents', () => {
  it('expands component imports with project-relative ids and client detection', () => {
    const projectRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-mdx-transform-'))
    const srcDir = path.join(projectRoot, 'src', 'components')
    fs.mkdirSync(srcDir, { recursive: true })

    const componentPath = path.join(srcDir, 'CodeBlock.tsx')
    fs.writeFileSync(componentPath, `'use client'\nexport default function CodeBlock() {}\n`)

    const registryPath = path.join(projectRoot, 'src', 'mdx-components.ts')
    fs.writeFileSync(
      registryPath,
      `import { defineMdxComponents } from 'rari/mdx'
import CodeBlock from './components/CodeBlock'

export const getMDXComponents = defineMdxComponents({
  CodeBlock,
})
`,
    )

    const transformed = transformDefineMdxComponents({
      code: fs.readFileSync(registryPath, 'utf-8'),
      id: registryPath,
      projectRoot,
      resolvedAlias: {},
    })

    expect(transformed).toContain('__RARI_MDX_RESOLVED__')
    expect(transformed).toContain('name: "CodeBlock"')
    expect(transformed).toContain('id: "src/components/CodeBlock.tsx"')
    expect(transformed).toContain('client: true')

    fs.rmSync(projectRoot, { recursive: true, force: true })
  })

  it('supports default and named imports in the same statement', () => {
    const projectRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-mdx-transform-'))
    const srcDir = path.join(projectRoot, 'src', 'components')
    fs.mkdirSync(srcDir, { recursive: true })

    const componentsPath = path.join(srcDir, 'CodeBlock.tsx')
    fs.writeFileSync(
      componentsPath,
      `'use client'
export default function CodeBlock() {}
export function MermaidChart() {}
`,
    )

    const registryPath = path.join(projectRoot, 'src', 'mdx-components.ts')
    fs.writeFileSync(
      registryPath,
      `import { defineMdxComponents } from 'rari/mdx'
import CodeBlock, { MermaidChart } from './components/CodeBlock'

export const getMDXComponents = defineMdxComponents({
  CodeBlock,
  MermaidChart,
})
`,
    )

    const transformed = transformDefineMdxComponents({
      code: fs.readFileSync(registryPath, 'utf-8'),
      id: registryPath,
      projectRoot,
      resolvedAlias: {},
    })

    expect(transformed).toContain('__RARI_MDX_RESOLVED__')
    expect(transformed).toContain('name: "CodeBlock"')
    expect(transformed).toContain('name: "MermaidChart"')

    fs.rmSync(projectRoot, { recursive: true, force: true })
  })

  it('ignores inline type imports when resolving component bindings', () => {
    const projectRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'rari-mdx-transform-'))
    const srcDir = path.join(projectRoot, 'src', 'components')
    fs.mkdirSync(srcDir, { recursive: true })

    const componentsPath = path.join(srcDir, 'CodeBlock.tsx')
    fs.writeFileSync(
      componentsPath,
      `'use client'
export default function CodeBlock() {}
export type CodeBlockProps = { language?: string }
`,
    )

    const registryPath = path.join(projectRoot, 'src', 'mdx-components.ts')
    fs.writeFileSync(
      registryPath,
      `import { defineMdxComponents } from 'rari/mdx'
import CodeBlock, { type CodeBlockProps } from './components/CodeBlock'

export const getMDXComponents = defineMdxComponents({
  CodeBlock,
})
`,
    )

    const transformed = transformDefineMdxComponents({
      code: fs.readFileSync(registryPath, 'utf-8'),
      id: registryPath,
      projectRoot,
      resolvedAlias: {},
    })

    expect(transformed).toContain('__RARI_MDX_RESOLVED__')
    expect(transformed).toContain('name: "CodeBlock"')

    fs.rmSync(projectRoot, { recursive: true, force: true })
  })
})
