import CodeBlock from '@/components/CodeBlock'
import PackageManagerTabs from '@/components/PackageManagerTabs'
import TerminalBlock from '@/components/TerminalBlock'

export const mdxComponentMetadata = [
  { name: 'CodeBlock', component: CodeBlock, id: 'src/components/CodeBlock.tsx' },
  { name: 'PackageManagerTabs', component: PackageManagerTabs, id: 'src/components/PackageManagerTabs.tsx' },
  { name: 'TerminalBlock', component: TerminalBlock, id: 'src/components/TerminalBlock.tsx' },
] as const
