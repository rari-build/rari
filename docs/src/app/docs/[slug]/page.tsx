import type { PageProps } from 'rari'
import { access, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import process from 'node:process'
import MdxRenderer from '@/components/MdxRenderer'

const DEFAULT_METADATA = {
  title: 'Rari Docs',
  description: 'Complete documentation for Rari framework.',
}

function isValidSlug(slug: unknown): slug is string {
  return typeof slug === 'string' && !slug.includes('..') && !slug.includes('/')
}

function getFilePath(slug: string) {
  return join(process.cwd(), 'public', 'content', 'docs', `${slug}.mdx`)
}

export default async function DocPage({ params }: PageProps) {
  const slug = params?.slug

  if (!isValidSlug(slug))
    return <div>Invalid documentation path.</div>

  return <MdxRenderer filePath={`docs/${slug}.mdx`} />
}

export async function getData({ params }: PageProps) {
  const slug = params?.slug

  if (!isValidSlug(slug))
    return { notFound: true }

  try {
    await access(getFilePath(slug))
    return { props: {} }
  }
  catch {
    return { notFound: true }
  }
}

export async function generateMetadata({ params }: PageProps) {
  const slug = params?.slug

  if (!isValidSlug(slug))
    return DEFAULT_METADATA

  try {
    const content = await readFile(getFilePath(slug), 'utf-8')
    const titleMatch = content.match(/^export\s+const\s+title\s*=\s*['"](.+)['"]/m)
    const descriptionMatch = content.match(/^export\s+const\s+description\s*=\s*['"](.+)['"]/m)

    if (titleMatch || descriptionMatch) {
      return {
        title: titleMatch ? `${titleMatch[1]} / Rari Docs` : DEFAULT_METADATA.title,
        description: descriptionMatch ? descriptionMatch[1] : DEFAULT_METADATA.description,
      }
    }

    const headingMatch = content.match(/^#\s+(\S.*)$/m)
    if (headingMatch) {
      return {
        title: `${headingMatch[1]} / Rari Docs`,
        description: DEFAULT_METADATA.description,
      }
    }
  }
  catch {}

  return DEFAULT_METADATA
}
