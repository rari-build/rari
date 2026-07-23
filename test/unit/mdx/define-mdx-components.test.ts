import { defineMdxComponents } from '@rari/mdx/components/define'
import { describe, expect, it, vi } from 'vite-plus/test'

vi.mock('@rari/runtime/rsc/references', () => ({
  registerClientReference: (proxy: unknown) => proxy,
}))

describe('defineMdxComponents', () => {
  it('returns only components referenced in MDX content', () => {
    const CodeBlock = () => null
    const MermaidChart = () => null

    const resolve = defineMdxComponents([
      { name: 'CodeBlock', component: CodeBlock, id: 'src/components/CodeBlock.tsx', client: true },
      {
        name: 'MermaidChart',
        component: MermaidChart,
        id: 'src/components/MermaidChart.tsx',
        client: true,
      },
    ])

    const components = resolve('<CodeBlock language="ts" />')

    expect(Object.keys(components)).toEqual(['CodeBlock'])
  })

  it('passes server components through without client references', () => {
    const Heading = () => null

    const resolve = defineMdxComponents([
      { name: 'Heading', component: Heading, id: 'src/components/Heading.tsx', client: false },
    ])

    const components = resolve('<Heading>Title</Heading>')

    expect(components.Heading).toBe(Heading)
  })

  it('requires vite-resolved metadata for object input', () => {
    const CodeBlock = () => null

    expect(() => defineMdxComponents({ CodeBlock })).toThrow(/missing module metadata/)
  })
})
