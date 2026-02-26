import type { PageProps } from 'rari'
import { access, readFile } from 'node:fs/promises'
import MdxRenderer from '@/components/MdxRenderer'
import { getDocsFilePath, isValidSlugArray } from '@/lib/content-utils'
import {
  DESCRIPTION_EXPORT_REGEX,
  HEADING_REGEX,
  TITLE_EXPORT_REGEX,
} from '@/lib/regex-constants'

const DEFAULT_METADATA = {
  title: 'rari Docs',
  description: 'Complete documentation for rari framework.',
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
    const titleMatch = content.match(TITLE_EXPORT_REGEX)
    const descriptionMatch = content.match(DESCRIPTION_EXPORT_REGEX)

    if (titleMatch || descriptionMatch) {
      const pageTitle = titleMatch ? `${titleMatch[2]} / rari Docs` : DEFAULT_METADATA.title
      const pageDescription = descriptionMatch ? descriptionMatch[2] : DEFAULT_METADATA.description

      return {
        title: pageTitle,
        description: pageDescription,
        openGraph: {
          title: pageTitle,
          description: pageDescription,
        },
      }
    }

    const headingMatch = content.match(HEADING_REGEX)
    if (headingMatch) {
      const pageTitle = `${headingMatch[1]} / rari Docs`
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
