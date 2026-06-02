import type { PageProps } from 'rari'
import { readFile } from 'node:fs/promises'
import { getDocsFilePath, isValidSlugArray } from '@/lib/content'
import { extractBasicMetadata } from '@/lib/metadata'
import { generateOGImage } from '@/lib/og-image'

export default async function Image({ params }: PageProps) {
  const slug = params?.slug
  let title = 'rari Docs'
  let description = 'Complete documentation for rari framework.'

  if (isValidSlugArray(slug)) {
    try {
      const content = await readFile(getDocsFilePath(slug), 'utf-8')
      const metadata = extractBasicMetadata(content)

      if (metadata.title)
        title = metadata.title
      if (metadata.description)
        description = metadata.description
    }
    catch {}
  }

  return generateOGImage({
    title,
    description,
    section: 'docs',
  })
}
