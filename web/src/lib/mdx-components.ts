import { createMDXClientReferences } from 'rari/mdx'
import CodeBlock from '@/components/CodeBlock'
import MermaidChart from '@/components/MermaidChart'
import PackageManagerTabs from '@/components/PackageManagerTabs'
import TerminalBlock from '@/components/TerminalBlock'

const allComponents = [
  { name: 'CodeBlock', component: CodeBlock, id: 'src/components/CodeBlock.tsx' },
  { name: 'MermaidChart', component: MermaidChart, id: 'src/components/MermaidChart.tsx' },
  { name: 'PackageManagerTabs', component: PackageManagerTabs, id: 'src/components/PackageManagerTabs.tsx' },
  { name: 'TerminalBlock', component: TerminalBlock, id: 'src/components/TerminalBlock.tsx' },
] as const

export function getMDXComponents(content: string) {
  const usedComponentNames = new Set<string>()

  for (const { name } of allComponents) {
    const jsxPattern = new RegExp(`<${name}[\\s>/]`)
    if (jsxPattern.test(content)) {
      usedComponentNames.add(name)
    }
  }

  const usedComponents = allComponents.filter(({ name }) => usedComponentNames.has(name))

  return createMDXClientReferences(
    Object.fromEntries(
      usedComponents.map(({ name, component, id }) => [
        name,
        { component, id },
      ]),
    ),
  )
}
