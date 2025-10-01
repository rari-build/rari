import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { cwd } from 'node:process'
import MarkdownIt from 'markdown-it'
import { createHighlighter } from 'shiki'

interface MarkdownRendererProps {
  filePath: string
  className?: string
}

let shikiHighlighter: Awaited<ReturnType<typeof createHighlighter>> | null
  = null

async function getHighlighter() {
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
  try {
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

    const highlighter = await getHighlighter()

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
              catch (error) {
                console.warn(`Failed to highlight ${lang}:`, error)
                return `<pre><code class="language-${lang}">${str}</code></pre>`
              }
            }
            return `<pre><code>${str}</code></pre>`
          }
        : undefined,
    })

    const html = md.render(content)

    return (
      <div
        className={`prose prose-invert max-w-none overflow-hidden ${className}`}
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
    return (
      <div className="prose prose-invert max-w-none">
        <p>
          Error loading markdown file:
          {filePath}
        </p>
      </div>
    )
  }
}
