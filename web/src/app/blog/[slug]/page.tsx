import type { PageProps } from 'rari'
import { accessSync, readdirSync, readFileSync } from 'node:fs'
import MdxRenderer from '@/components/docs/MdxRenderer'
import { getBlogDir, getBlogFilePath, isValidSlug } from '@/lib/content'
import { extractBasicMetadata } from '@/lib/content/metadata'
import { container } from '@/lib/site/styles'

const DEFAULT_METADATA = {
  title: 'rari Blog',
  description: 'Latest news and updates from the rari team.',
}

export default function BlogPage({ params }: PageProps) {
  const slug = params.slug

  if (!isValidSlug(slug)) return <div className={container.base}>Invalid blog post path.</div>

  return (
    <article className="max-w-4xl mx-auto px-4 lg:px-8 py-8 lg:py-12 pt-16 lg:pt-12 w-full">
      <MdxRenderer filePath={`blog/${slug}.mdx`} />
    </article>
  )
}

export function getData({ params }: PageProps) {
  const slug = params.slug

  if (!isValidSlug(slug)) return { notFound: true }

  try {
    accessSync(getBlogFilePath(slug))
    return { props: {} }
  } catch {
    return { notFound: true }
  }
}

export function generateMetadata({ params }: PageProps) {
  const slug = params.slug

  if (!isValidSlug(slug)) return DEFAULT_METADATA

  try {
    const content = readFileSync(getBlogFilePath(slug), 'utf-8')
    const metadata = extractBasicMetadata(content)

    const title =
      metadata.title != null && metadata.title !== ''
        ? `${metadata.title} / rari Blog`
        : DEFAULT_METADATA.title
    const description = metadata.description ?? DEFAULT_METADATA.description

    return {
      title,
      description,
      openGraph: {
        title: metadata.title ?? DEFAULT_METADATA.title,
        description,
      },
    }
  } catch {}

  return DEFAULT_METADATA
}

export function generateStaticParams() {
  const contentDir = getBlogDir()

  try {
    const entries = readdirSync(contentDir)
    return entries
      .filter(entry => entry.endsWith('.mdx'))
      .map(entry => ({ slug: entry.replace(/\.mdx$/, '') }))
  } catch {
    return []
  }
}
