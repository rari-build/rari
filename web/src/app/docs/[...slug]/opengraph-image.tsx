import type { PageProps } from 'rari'
import { getDocsFilePath, isValidSlugArray } from '@/lib/content'
import { loadOgMeta } from '@/lib/content/og-meta'
import { generateOGImage } from '@/lib/site/og-image'

export default async function Image({ params }: PageProps) {
  const slug = params.slug
  const defaults = {
    title: 'rari Docs',
    description: 'Complete documentation for rari framework.',
  }

  const { title, description } = isValidSlugArray(slug)
    ? await loadOgMeta(getDocsFilePath(slug), defaults)
    : defaults

  return generateOGImage({
    title,
    description: description ?? defaults.description,
    section: 'docs',
  })
}
