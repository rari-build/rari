import type { ComponentProps } from 'react'
import type { BlogMetadata } from '@/lib/metadata'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { cwd } from 'node:process'
import { evaluate } from 'rari/mdx'
import * as runtime from 'react/jsx-runtime'
import remarkGfm from 'remark-gfm'
import NotFoundPage from '@/app/not-found'
import Breadcrumbs from '@/components/Breadcrumbs'
import Heading from '@/components/Heading'
import PageHeader from '@/components/PageHeader'
import { extractBlogMetadata } from '@/lib/metadata'
import { remarkCodeBlock } from '@/lib/remark-codeblock'
import { getHighlighter, SHIKI_THEMES } from '@/lib/shiki'

interface MdxRendererProps {
  readonly filePath: string
  readonly className?: string
  readonly pathname?: string
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
    } catch {}
  }

  return null
}

function PageHeaderWithFilePath({
  filePath,
  blogMetadata,
  ...props
}: ComponentProps<typeof PageHeader> &
  Readonly<Readonly<{ readonly filePath: string; readonly blogMetadata?: BlogMetadata }>>) {
  return <PageHeader {...blogMetadata} {...props} filePath={filePath} />
}

function createMdxComponents(filePath: string, blogMetadata?: BlogMetadata) {
  return {
    PageHeader: (props: any) => (
      <PageHeaderWithFilePath {...props} filePath={filePath} blogMetadata={blogMetadata} />
    ), // oxlint-disable-line react/component-hook-factories
    h2: (props: any) => <Heading level={2} {...props} />,
    h3: (props: any) => <Heading level={3} {...props} />,
    h4: (props: any) => <Heading level={4} {...props} />,
    h5: (props: any) => <Heading level={5} {...props} />,
    h6: (props: any) => <Heading level={6} {...props} />,
  }
}

export default async function MdxRenderer({
  filePath,
  className = '',
  pathname,
}: MdxRendererProps) {
  const content = findContentFile(filePath)
  if (content == null || content === '') return <NotFoundPage />

  // eslint-disable-next-line react/error-boundaries
  try {
    const highlighter = await getHighlighter()
    const remarkPlugins: any[] = [
      remarkGfm,
      [remarkCodeBlock, { highlighter, themes: SHIKI_THEMES }],
    ]

    const isBlogPost = filePath.startsWith('blog/')
    const blogMetadata = isBlogPost ? extractBlogMetadata(content) : undefined

    const { default: MDXContent } = await evaluate(content, {
      ...runtime,
      baseUrl: import.meta.url,
      development: false,
      remarkPlugins,
      components: createMdxComponents(filePath, blogMetadata),
    })

    return (
      <div
        className={`prose max-w-none overflow-hidden ${className}`}
        style={{
          wordWrap: 'break-word',
          overflowWrap: 'break-word',
        }}
      >
        {pathname != null && pathname !== '' && <Breadcrumbs pathname={pathname} />}
        <MDXContent />
      </div>
    )
  } catch (error) {
    console.error('Error rendering MDX:', error)
    throw error
  }
}
