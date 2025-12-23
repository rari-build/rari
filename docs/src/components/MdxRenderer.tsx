import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cwd } from 'node:process'
import { compile } from '@mdx-js/mdx'
import rehypeShikiFromHighlighter from '@shikijs/rehype/core'
import { Fragment, jsx, jsxs } from 'react/jsx-runtime'
import NotFoundPage from '@/app/not-found'
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

    const compiled = await compile(content, {
      outputFormat: 'function-body',
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

    const compiledString = String(compiled)

    // oxlint-disable-next-line no-new-func
    const fn = new Function(compiledString)
    const { default: MDXContent } = fn({ Fragment, jsx, jsxs })

    return (
      <div
        className={`prose prose-invert max-w-none overflow-hidden ${className}`}
        style={{
          wordWrap: 'break-word',
          overflowWrap: 'break-word',
        }}
      >
        <MDXContent />
      </div>
    )
  }
  catch (error) {
    console.error('Error in MdxRenderer:', error)
    return <NotFoundPage />
  }
}
