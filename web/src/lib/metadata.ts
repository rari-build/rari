import {
  AUTHOR_EXPORT_REGEX,
  DATE_EXPORT_REGEX,
  DESCRIPTION_EXPORT_REGEX,
  HEADING_REGEX,
  TITLE_EXPORT_REGEX,
} from './regex-constants'

export interface BlogMetadata {
  readonly title?: string
  readonly description?: string
  readonly date?: string
  readonly author?: string
  readonly authorUrl?: string
  readonly tags?: readonly string[]
}

export function extractBlogMetadata(content: string): BlogMetadata {
  const titleMatch = content.match(TITLE_EXPORT_REGEX)
  const descriptionMatch = content.match(DESCRIPTION_EXPORT_REGEX)
  const dateMatch = content.match(DATE_EXPORT_REGEX)
  const authorMatch = content.match(AUTHOR_EXPORT_REGEX)
  const authorUrlMatch = /^export\s+const\s+authorUrl\s*=\s*['"]([^'"]+)['"]/m.exec(content)
  const tagsMatch = /^export\s+const\s+tags\s*=\s*\[([^\]]*)\]/m.exec(content)

  const tags = tagsMatch
    ? tagsMatch[1]
        .split(',')
        .map(tag => tag.trim().replace(/['"]/g, ''))
        .filter(Boolean)
    : undefined

  return {
    title: titleMatch?.[2],
    description: descriptionMatch?.[2],
    date: dateMatch?.[2],
    author: authorMatch?.[2],
    authorUrl: authorUrlMatch?.[1],
    tags,
  }
}

export function extractBasicMetadata(content: string): { title?: string; description?: string } {
  const titleMatch = content.match(TITLE_EXPORT_REGEX)
  const descriptionMatch = content.match(DESCRIPTION_EXPORT_REGEX)

  return {
    title: titleMatch?.[2],
    description: descriptionMatch?.[2],
  }
}

export function extractMetadataWithFallback(content: string): {
  title?: string
  description?: string
} {
  const titleMatch = content.match(TITLE_EXPORT_REGEX)
  const descriptionMatch = content.match(DESCRIPTION_EXPORT_REGEX)

  if (titleMatch || descriptionMatch) {
    return {
      title: titleMatch?.[2],
      description: descriptionMatch?.[2],
    }
  }

  const headingMatch = content.match(HEADING_REGEX)
  if (headingMatch) {
    return {
      title: headingMatch[1],
      description: undefined,
    }
  }

  return {}
}
