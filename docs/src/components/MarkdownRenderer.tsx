import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cwd } from 'node:process'
import { marked } from 'marked'
import NotFoundPage from '@/app/not-found'
import { parseFrontmatter } from '@/lib/frontmatter'
import { getHighlighter, SHIKI_THEME } from '@/lib/shiki'

interface MarkdownRendererProps {
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

export default async function MarkdownRenderer({
  filePath,
  className = '',
}: MarkdownRendererProps) {
  try {
    const content = findContentFile(filePath)
    if (!content) {
      return <NotFoundPage />
    }

    const { content: contentWithoutFrontmatter } = parseFrontmatter(content)

    const highlighter = await getHighlighter()

    marked.setOptions({
      gfm: true,
      breaks: false,
    })

    if (highlighter) {
      marked.use({
        renderer: {
          code({ text, lang }: { text: string, lang?: string }) {
            if (!lang) {
              return `<pre><code>${text}</code></pre>`
            }

            try {
              return highlighter.codeToHtml(text, {
                lang,
                theme: SHIKI_THEME,
              })
            }
            catch (error) {
              console.warn(`Failed to highlight ${lang}:`, error)
              return `<pre><code class="language-${lang}">${text}</code></pre>`
            }
          },
        },
      })
    }

    const html = await marked.parse(contentWithoutFrontmatter)

    return (
      <div
        className={`prose prose-invert max-w-none overflow-hidden ${className}`}
        // eslint-disable-next-line react-dom/no-dangerously-set-innerhtml
        dangerouslySetInnerHTML={{ __html: html }}
        style={{
          wordWrap: 'break-word',
          overflowWrap: 'break-word',
        }}
      />
    )
  }
  catch (error) {
    console.error('Error in MarkdownRenderer:', error)
    return <NotFoundPage />
  }
}
