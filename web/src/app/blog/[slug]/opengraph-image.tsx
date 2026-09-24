import type { PageProps } from 'rari'
import { getBlogFilePath, isValidSlug } from '@/lib/content'
import { loadOgMeta } from '@/lib/content/og-meta'
import { generateOGImage } from '@/lib/site/og-image'

export default async function Image({ params }: PageProps) {
  const slug = params.slug
  const defaults = { title: 'rari Blog' }

  const { title } = isValidSlug(slug) ? await loadOgMeta(getBlogFilePath(slug), defaults) : defaults

  return generateOGImage({
    title,
    section: 'blog',
  })
}
