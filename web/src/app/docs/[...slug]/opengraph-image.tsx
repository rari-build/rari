import type { PageProps } from 'rari'
import { readFile } from 'node:fs/promises'
import { getDocsFilePath, isValidSlugArray } from '@/lib/content'
import { generateOGImage } from '@/lib/og-image'
import { DESCRIPTION_EXPORT_REGEX, TITLE_EXPORT_REGEX } from '@/lib/regex-constants'

export default async function Image({ params }: PageProps) {
  const slug = params?.slug
  let title = 'rari Docs'
  let description = 'Complete documentation for rari framework.'

  if (isValidSlugArray(slug)) {
    try {
      const content = await readFile(getDocsFilePath(slug), 'utf-8')
      const titleMatch = content.match(TITLE_EXPORT_REGEX)
      const descriptionMatch = content.match(DESCRIPTION_EXPORT_REGEX)

      if (titleMatch)
        title = titleMatch[2]
      if (descriptionMatch)
        description = descriptionMatch[2]
    }
    catch {}
  }

  return generateOGImage({
    title,
    description,
    section: 'docs',
  })
}
