import type { ASTNode } from '../../../web/src/lib/mdx/remark-fences'
import { describe, expect, it } from 'vite-plus/test'
import { remarkFences } from '../../../web/src/lib/mdx/remark-fences'

const transform = remarkFences()

describe('remarkFences', () => {
  it('maps language fences to CodeBlock with filename meta', () => {
    const tree: ASTNode = {
      type: 'root',
      children: [
        {
          type: 'code',
          lang: 'tsx',
          meta: 'title="src/app/page.tsx"',
          value: 'export default function Page() {}',
        },
      ],
    }

    transform(tree)

    expect(tree.children?.[0]).toMatchObject({
      type: 'mdxJsxFlowElement',
      name: 'CodeBlock',
      attributes: [
        { type: 'mdxJsxAttribute', name: 'language', value: 'tsx' },
        { type: 'mdxJsxAttribute', name: 'filename', value: 'src/app/page.tsx' },
      ],
      children: [{ type: 'text', value: 'export default function Page() {}' }],
    })
  })

  it('maps terminal fences to TerminalBlock', () => {
    const tree: ASTNode = {
      type: 'root',
      children: [{ type: 'code', lang: 'terminal', value: 'pnpm dev\n' }],
    }

    transform(tree)

    expect(tree.children?.[0]).toMatchObject({
      type: 'mdxJsxFlowElement',
      name: 'TerminalBlock',
      attributes: [{ type: 'mdxJsxAttribute', name: 'command', value: 'pnpm dev' }],
    })
  })

  it('maps package-managers fences to PackageManagerTabs flat attrs', () => {
    const tree: ASTNode = {
      type: 'root',
      children: [
        {
          type: 'code',
          lang: 'package-managers',
          value:
            'pnpm: pnpm add rari\nnpm: npm install rari\nyarn: yarn add rari\nbun: bun add rari\n',
        },
      ],
    }

    transform(tree)

    expect(tree.children?.[0]).toMatchObject({
      type: 'mdxJsxFlowElement',
      name: 'PackageManagerTabs',
      attributes: [
        { type: 'mdxJsxAttribute', name: 'pnpm', value: 'pnpm add rari' },
        { type: 'mdxJsxAttribute', name: 'npm', value: 'npm install rari' },
        { type: 'mdxJsxAttribute', name: 'yarn', value: 'yarn add rari' },
        { type: 'mdxJsxAttribute', name: 'bun', value: 'bun add rari' },
      ],
    })
  })

  it('maps mermaid fences to MermaidChart', () => {
    const tree: ASTNode = {
      type: 'root',
      children: [{ type: 'code', lang: 'mermaid', value: 'graph TD\n  A-->B' }],
    }

    transform(tree)

    expect(tree.children?.[0]).toMatchObject({
      type: 'mdxJsxFlowElement',
      name: 'MermaidChart',
      children: [{ type: 'text', value: 'graph TD\n  A-->B' }],
    })
  })

  it('leaves non-code nodes alone', () => {
    const tree: ASTNode = {
      type: 'root',
      children: [{ type: 'paragraph', children: [{ type: 'text', value: 'hi' }] }],
    }

    transform(tree)

    expect(tree.children?.[0]?.type).toBe('paragraph')
  })
})
