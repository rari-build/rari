import type { PageProps } from 'rari'
import { accessSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import MdxRenderer from '@/components/MdxRenderer'
import { getBlogFilePath, isValidSlug } from '@/lib/content'
import {
  DESCRIPTION_EXPORT_REGEX,
  HEADING_REGEX,
  TITLE_EXPORT_REGEX,
} from '@/lib/regex-constants'

const DEFAULT_METADATA = {
  title: 'rari Blog',
  description: 'Latest news and updates from the rari team.',
}

export default function BlogPage({ params }: PageProps) {
  const slug = params?.slug

  if (!isValidSlug(slug))
    return <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">Invalid blog post path.</div>

  return (
    <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">
      <MdxRenderer filePath={`blog/${slug}.mdx`} />
    </div>
  )
}

export function getData({ params }: PageProps) {
  const slug = params?.slug

  if (!isValidSlug(slug))
    return { notFound: true }

  try {
    accessSync(getBlogFilePath(slug))
    return { props: {} }
  }
  catch {
    return { notFound: true }
  }
}

export function generateMetadata({ params }: PageProps) {
  const slug = params?.slug

  if (!isValidSlug(slug))
    return DEFAULT_METADATA

  try {
    const content = readFileSync(getBlogFilePath(slug), 'utf-8')
    const titleMatch = content.match(TITLE_EXPORT_REGEX)
    const descriptionMatch = content.match(DESCRIPTION_EXPORT_REGEX)

    if (titleMatch || descriptionMatch) {
      return {
        title: titleMatch ? `${titleMatch[2]} / rari Blog` : DEFAULT_METADATA.title,
        description: descriptionMatch ? descriptionMatch[2] : DEFAULT_METADATA.description,
      }
    }

    const headingMatch = content.match(HEADING_REGEX)
    if (headingMatch) {
      return {
        title: `${headingMatch[1]} / rari Blog`,
        description: DEFAULT_METADATA.description,
      }
    }
  }
  catch {}

  return DEFAULT_METADATA
}

export function generateStaticParams() {
  const contentDir = join(process.cwd(), 'public', 'content', 'blog')

  try {
    const entries = readdirSync(contentDir)
    return entries
      .filter(entry => entry.endsWith('.mdx'))
      .map(entry => ({ slug: entry.replace(/\.mdx$/, '') }))
  }
  catch {
    return []
  }
}
