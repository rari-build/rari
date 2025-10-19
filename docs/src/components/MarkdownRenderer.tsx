import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { cwd } from 'node:process'

interface MarkdownRendererProps {
  filePath: string
  className?: string
}

let shikiHighlighter: any | null = null

async function getHighlighter(createHighlighter: any) {
  if (!shikiHighlighter) {
    try {
      shikiHighlighter = await createHighlighter({
        themes: ['github-dark'],
        langs: [
          'javascript',
          'typescript',
          'tsx',
          'jsx',
          'json',
          'rust',
          'bash',
        ],
      })
    }
    catch {
      return null
    }
  }
  return shikiHighlighter
}

export default async function MarkdownRenderer({
  filePath,
  className = '',
}: MarkdownRendererProps) {
  let html: string | null = null
  let error: Error | null = null

  try {
    const MarkdownIt = (await import('markdown-it')).default
    const { createHighlighter } = await import('shiki')

    const distPath = join(cwd(), 'dist', 'content', filePath)
    const contentPath = join(cwd(), 'content', filePath)
    const publicPath = join(cwd(), 'public', 'content', filePath)

    let content: string
    if (existsSync(distPath)) {
      content = readFileSync(distPath, 'utf-8')
    }
    else if (existsSync(contentPath)) {
      content = readFileSync(contentPath, 'utf-8')
    }
    else {
      content = readFileSync(publicPath, 'utf-8')
    }

    const highlighter = await getHighlighter(createHighlighter)

    const md = new MarkdownIt({
      html: true,
      linkify: true,
      typographer: true,
      breaks: false,
      highlight: highlighter
        ? (str, lang) => {
            if (lang && highlighter) {
              try {
                return highlighter.codeToHtml(str, {
                  lang,
                  theme: 'github-dark',
                })
              }
              catch (highlightError) {
                console.warn(`Failed to highlight ${lang}:`, highlightError)
                return `<pre><code class="language-${lang}">${str}</code></pre>`
              }
            }
            return `<pre><code>${str}</code></pre>`
          }
        : undefined,
    })

    html = md.render(content)
  }
  catch (err) {
    console.error('Error in MarkdownRenderer:', err)
    error = err instanceof Error ? err : new Error(String(err))
  }

  if (error || !html) {
    return (
      <div className="prose prose-invert max-w-none">
        <p>
          Error loading markdown file:
          {filePath}
        </p>
      </div>
    )
  }

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
