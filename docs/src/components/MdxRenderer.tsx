import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cwd } from 'node:process'
import { evaluate } from '@mdx-js/mdx'
import rehypeShikiFromHighlighter from '@shikijs/rehype/core'
import { createMDXClientReferences } from 'rari/mdx'
import * as runtime from 'react/jsx-runtime'
import NotFoundPage from '@/app/not-found'
import PackageManagerTabs from '@/components/PackageManagerTabs'
import TerminalBlock from '@/components/TerminalBlock'
import { getHighlighter, SHIKI_THEME } from '@/lib/shiki'

interface MdxRendererProps {
  filePath: string
  className?: string
}

function findContentFile(filePath: string): string | null {
  const searchPaths = [
    resolve(cwd(), 'public', 'content', filePath),
    resolve(cwd(), 'content', filePath),
    resolve(cwd(), 'dist', 'content', filePath),
  ]

  for (const path of searchPaths) {
    try {
      return readFileSync(path, 'utf-8')
    }
    catch {
      // File doesn't exist at this path, try next
    }
  }

  return null
}

export default async function MdxRenderer({
  filePath,
  className = '',
}: MdxRendererProps) {
  try {
    const content = findContentFile(filePath)
    if (!content) {
      return <NotFoundPage />
    }

    const highlighter = await getHighlighter()

    const { default: MDXContent } = await evaluate(content, {
      ...runtime,
      baseUrl: import.meta.url,
      development: false,
      rehypePlugins: highlighter
        ? [
            [
              rehypeShikiFromHighlighter,
              highlighter,
              {
                theme: SHIKI_THEME,
              },
            ],
          ]
        : [],
    })

    const mdxComponents = createMDXClientReferences({
      PackageManagerTabs: {
        component: PackageManagerTabs,
        id: 'src/components/PackageManagerTabs.tsx',
      },
      TerminalBlock: {
        component: TerminalBlock,
        id: 'src/components/TerminalBlock.tsx',
      },
    })

    return (
      <div
        className={`prose prose-invert max-w-none overflow-hidden ${className}`}
        style={{
          wordWrap: 'break-word',
          overflowWrap: 'break-word',
        }}
      >
        <MDXContent components={mdxComponents} />
      </div>
    )
  }
  catch (error) {
    console.error('Error in MdxRenderer:', error)
    return <NotFoundPage />
  }
}
