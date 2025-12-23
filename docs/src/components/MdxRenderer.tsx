import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cwd } from 'node:process'
import { Fragment, jsx, jsxs } from 'react/jsx-runtime'
import NotFoundPage from '@/app/not-found'
import { compileMDXRemote } from '@/lib/mdx-remote'
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
}: MdxRendererProps) {
  try {
    const content = findContentFile(filePath)
    if (!content) {
      return <NotFoundPage />
    }

    const highlighter = await getHighlighter()

    const remarkShiki = () => {
      return async (tree: any) => {
        const { visit } = await import('unist-util-visit')

        visit(tree, 'code', (node: any) => {
          if (node.lang && highlighter) {
            try {
              const html = highlighter.codeToHtml(node.value, {
                lang: node.lang,
                theme: SHIKI_THEME,
              })

              node.type = 'html'
              node.value = html
              delete node.lang
              delete node.meta
            }
            catch (error) {
              console.warn(`Failed to highlight ${node.lang}:`, error)
            }
          }
        })
      }
    }

    const compiledCode = await compileMDXRemote({
      source: content,
      options: {
        remarkPlugins: highlighter ? [remarkShiki] : [],
        rehypePlugins: [
          [
            (await import('rehype-raw')).default,
            { passThrough: ['mdxjsEsm', 'mdxFlowExpression', 'mdxJsxFlowElement', 'mdxJsxTextElement', 'mdxTextExpression'] },
          ],
        ],
      },
    })

    // oxlint-disable-next-line no-new-func
    const fn = new Function(compiledCode)
    const module = fn({ Fragment, jsx, jsxs })
    const MDXComponent = module.default
    const mdxContent = MDXComponent({})

    return mdxContent
  }
  catch (error) {
    console.error('Error in MdxRenderer:', error)
    return <NotFoundPage />
  }
}
