import type { PageProps } from 'rari'
import { readdir, readFile } from 'node:fs/promises'
import { notFound } from 'rari'
import MdxRenderer from '@/components/mdx/MdxRenderer'
import { contentFileExists, getBlogDir, getBlogFilePath, isValidSlug } from '@/lib/content'
import { extractBasicMetadata } from '@/lib/content/metadata'

const DEFAULT_METADATA = {
  title: 'rari Blog',
  description: 'Latest news and updates from the rari team.',
}

export default async function BlogPage({ params }: PageProps) {
  const slug = params.slug
  if (!isValidSlug(slug)) notFound()
  if (!(await contentFileExists(await getBlogFilePath(slug)))) notFound()

  return (
    <article className="max-w-4xl mx-auto px-4 lg:px-8 py-8 lg:py-12 pt-16 lg:pt-12 w-full">
      <MdxRenderer filePath={`blog/${slug}.mdx`} />
    </article>
  )
}

export async function generateMetadata({ params }: PageProps) {
  const slug = params.slug
  if (!isValidSlug(slug)) return DEFAULT_METADATA

  const content = await readFile(await getBlogFilePath(slug), 'utf-8').catch(() => null)
  if (content == null) return DEFAULT_METADATA

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
}

export async function generateStaticParams() {
  try {
    const entries = await readdir(await getBlogDir())
    return entries
      .filter(entry => entry.endsWith('.mdx'))
      .map(entry => ({ slug: entry.replace(/\.mdx$/, '') }))
  } catch {
    return []
  }
}
