import type { PageProps } from 'rari'
import { access, readFile } from 'node:fs/promises'
import MdxRenderer from '@/components/MdxRenderer'
import { getDocsFilePath, isValidSlugArray } from '@/lib/content-utils'

const DEFAULT_METADATA = {
  title: 'Rari Docs',
  description: 'Complete documentation for Rari framework.',
}

export default async function DocPage({ params }: PageProps) {
  const slug = params?.slug

  if (!isValidSlugArray(slug))
    return <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">Invalid documentation path.</div>

  const slugPath = slug.join('/')
  const pathname = `/docs/${slugPath}`

  return (
    <div className="max-w-5xl mx-auto px-4 lg:px-8 py-4 lg:py-8 pt-16 lg:pt-8 w-full">
      <MdxRenderer filePath={`docs/${slugPath}.mdx`} pathname={pathname} />
    </div>
  )
}

export async function getData({ params }: PageProps) {
  const slug = params?.slug

  if (!isValidSlugArray(slug))
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

  if (!isValidSlugArray(slug))
    return DEFAULT_METADATA

  try {
    const content = await readFile(getDocsFilePath(slug), 'utf-8')
    const titleMatch = content.match(/^export\s+const\s+title\s*=\s*['"](.+)['"]/m)
    const descriptionMatch = content.match(/^export\s+const\s+description\s*=\s*['"](.+)['"]/m)

    if (titleMatch || descriptionMatch) {
      const pageTitle = titleMatch ? `${titleMatch[1]} / Rari Docs` : DEFAULT_METADATA.title
      const pageDescription = descriptionMatch ? descriptionMatch[1] : DEFAULT_METADATA.description

      return {
        title: pageTitle,
        description: pageDescription,
        openGraph: {
          title: pageTitle,
          description: pageDescription,
        },
      }
    }

    const headingMatch = content.match(/^#\s+(\S.*)$/m)
    if (headingMatch) {
      const pageTitle = `${headingMatch[1]} / Rari Docs`
      return {
        title: pageTitle,
        description: DEFAULT_METADATA.description,
        openGraph: {
          title: pageTitle,
          description: DEFAULT_METADATA.description,
        },
      }
    }
  }
  catch {}

  return DEFAULT_METADATA
}
