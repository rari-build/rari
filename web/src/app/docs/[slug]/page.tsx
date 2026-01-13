import type { PageProps } from 'rari'
import { access, readFile } from 'node:fs/promises'
import MdxRenderer from '@/components/MdxRenderer'
import { getDocsFilePath, isValidSlug } from '@/lib/content-utils'

const DEFAULT_METADATA = {
  title: 'Rari Docs',
  description: 'Complete documentation for Rari framework.',
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
    await access(getDocsFilePath(slug))
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
    const content = await readFile(getDocsFilePath(slug), 'utf-8')
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
