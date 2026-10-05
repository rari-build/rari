import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { cwd } from 'node:process'
import { component$, Resource, useResource$ } from '@qwik.dev/core'
import MarkdownIt from 'markdown-it'

interface MarkdownTestProps {
  filePath?: string
}

export default component$((props: MarkdownTestProps) => {
  const filePath = props.filePath ?? 'demo-article.md'

  const htmlResource = useResource$(async () => {
    const sharedPath = join(
      /* turbopackIgnore: true */ cwd(),
      'node_modules',
      '@benchmark',
      'shared-qwik',
      'content',
      filePath,
    )
    const sharedDevPath = join(
      /* turbopackIgnore: true */ cwd(),
      '..',
      'shared',
      'content',
      filePath,
    )
    const distPath = join(/* turbopackIgnore: true */ cwd(), 'dist', 'content', filePath)
    const contentPath = join(/* turbopackIgnore: true */ cwd(), 'content', filePath)
    const publicPath = join(/* turbopackIgnore: true */ cwd(), 'public', 'content', filePath)

    const paths = [sharedDevPath, sharedPath, distPath, contentPath, publicPath]
    const fullPath = paths.find(p => existsSync(/* turbopackIgnore: true */ p)) || publicPath
    const content = readFileSync(/* turbopackIgnore: true */ fullPath, 'utf-8')

    const md = new MarkdownIt({
      html: true,
      linkify: true,
      typographer: true,
      breaks: false,
    })

    const html = md.render(content)
    return html
  })

  return (
    <Resource
      value={htmlResource}
      onResolved={html => (
        <div class="max-w-4xl mx-auto p-6">
          <div class="mb-8 p-4 bg-linear-to-r from-blue-50 to-indigo-50 border border-blue-200 rounded-lg">
            <div class="flex items-center gap-3 mb-2">
              <div class="w-8 h-8 bg-blue-600 rounded-lg flex items-center justify-center">
                <span class="text-white text-sm font-bold">MD</span>
              </div>
              <h2 class="text-xl font-bold text-blue-900">Markdown Rendering Demo</h2>
            </div>
            <p class="text-blue-700 text-sm">
              Server-side markdown processing using{' '}
              <code class="bg-blue-100 px-1 py-0.5 rounded text-xs">markdown-it</code> from npm
            </p>
          </div>

          <style>
            {`
          .markdown-content h1 { margin: 2rem 0 1.5rem 0; font-size: 2.25rem; font-weight: bold; color: #1f2937; }
          .markdown-content h2 { margin: 1.5rem 0 1rem 0; font-size: 1.875rem; font-weight: bold; color: #1f2937; }
          .markdown-content h3 { margin: 1.25rem 0 0.75rem 0; font-size: 1.5rem; font-weight: bold; color: #1f2937; }
          .markdown-content p { margin: 0 0 1rem 0; line-height: 1.7; color: #374151; }
          .markdown-content ul, .markdown-content ol { margin: 1rem 0; padding-left: 1.5rem; }
          .markdown-content li { margin: 0.25rem 0; }
          .markdown-content pre { margin: 1.5rem 0; padding: 1rem; background: #1f2937; color: #f9fafb; border-radius: 0.5rem; overflow-x: auto; }
          .markdown-content code { background: #f3f4f6; padding: 0.125rem 0.25rem; border-radius: 0.25rem; font-family: ui-monospace, monospace; }
          .markdown-content pre code { background: transparent; padding: 0; }
          .markdown-content table { width: 100%; border-collapse: collapse; margin: 1.5rem 0; }
          .markdown-content th, .markdown-content td { border: 1px solid #d1d5db; padding: 0.5rem 1rem; text-align: left; }
          .markdown-content th { background: #f9fafb; font-weight: 600; }
          .markdown-content blockquote { border-left: 4px solid #3b82f6; padding-left: 1rem; margin: 1.5rem 0; font-style: italic; color: #6b7280; }
          .markdown-content hr { margin: 2rem 0; border: none; border-top: 1px solid #d1d5db; }
          .markdown-content a { color: #2563eb; text-decoration: underline; }
          .markdown-content a:hover { color: #1d4ed8; }
          .markdown-content strong { font-weight: 600; color: #1f2937; }
          .markdown-content em { font-style: italic; }
        `}
          </style>
          <div class="markdown-content" dangerouslySetInnerHTML={html} />
        </div>
      )}
      onRejected={error => (
        <div class="max-w-4xl mx-auto p-6">
          <div class="p-6 bg-red-50 border border-red-200 rounded-lg">
            <h2 class="text-xl font-bold text-red-900 mb-4">Markdown Processing Error</h2>
            <p class="text-red-700 mb-3">
              Error loading markdown file:{' '}
              <code>
                content/
                {filePath}
              </code>
            </p>
            <div class="bg-red-100 p-3 rounded border">
              <code class="text-red-800 text-sm">
                {error instanceof Error ? error.message : String(error)}
              </code>
            </div>
          </div>
        </div>
      )}
    />
  )
})
