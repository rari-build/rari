import type { PageProps } from 'rari'
import { readFile } from 'node:fs/promises'
import { extractBasicMetadata, getBlogFilePath, isValidSlug } from '@/lib/content'
import { generateOGImage } from '@/lib/site'

export default async function Image({ params }: PageProps) {
  const slug = params.slug
  let title = 'rari Blog'

  if (isValidSlug(slug)) {
    try {
      const content = await readFile(getBlogFilePath(slug), 'utf-8')
      const metadata = extractBasicMetadata(content)

      if (metadata.title != null && metadata.title !== '') title = metadata.title
    } catch {}
  }

  return generateOGImage({
    title,
    section: 'blog',
  })
}
