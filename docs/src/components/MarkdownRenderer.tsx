import type { HighlighterCore } from '@shikijs/core'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cwd } from 'node:process'
import { createHighlighterCore } from '@shikijs/core'
import { createOnigurumaEngine } from '@shikijs/engine-oniguruma'
import bash from '@shikijs/langs/bash'
import javascript from '@shikijs/langs/javascript'
import json from '@shikijs/langs/json'
import jsx from '@shikijs/langs/jsx'
import rust from '@shikijs/langs/rust'
import tsx from '@shikijs/langs/tsx'
import typescript from '@shikijs/langs/typescript'
import githubDark from '@shikijs/themes/github-dark'
import MarkdownIt from 'markdown-it'
import NotFoundPage from '@/app/not-found'

interface MarkdownRendererProps {
  filePath: string
  className?: string
}

let shikiHighlighter: HighlighterCore | null = null

async function getHighlighter(): Promise<HighlighterCore | null> {
  if (!shikiHighlighter) {
    try {
      shikiHighlighter = await createHighlighterCore({
        themes: [githubDark],
        langs: [javascript, typescript, tsx, jsx, json, rust, bash],
        engine: createOnigurumaEngine(import('@shikijs/engine-oniguruma/wasm-inlined')),
      })
    }
    catch (error) {
      console.error('Failed to initialize syntax highlighter:', error)
      return null
    }
  }
  return shikiHighlighter
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

    const highlighter = await getHighlighter()

    const md = new MarkdownIt({
      html: true,
      linkify: true,
      typographer: true,
      breaks: false,
      highlight: highlighter
        ? (str, lang) => {
            if (!lang) {
              return `<pre><code>${str}</code></pre>`
            }

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
        : undefined,
    })

    const html = md.render(content)

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
