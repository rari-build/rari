import type { PageProps } from 'rari'
import { readFile } from 'node:fs/promises'
import { getBlogFilePath, isValidSlug } from '@/lib/content'
import { extractBasicMetadata } from '@/lib/metadata'
import { generateOGImage } from '@/lib/og-image'

export default async function Image({ params }: PageProps) {
  const slug = params?.slug
  let title = 'rari Blog'

  if (isValidSlug(slug)) {
    try {
      const content = await readFile(getBlogFilePath(slug), 'utf-8')
      const metadata = extractBasicMetadata(content)

      if (metadata.title)
        title = metadata.title
    }
    catch {}
  }

  return generateOGImage({
    title,
    section: 'blog',
  })
}
