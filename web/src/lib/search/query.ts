import type { SearchResult } from './types'
import { getDocsDir } from '@/lib/content'
import { WHITESPACE_REGEX } from '@/lib/utils/regex-constants'
import {
  angleBracketRegex,
  extractExcerpt,
  getSearchIndex,
  inlineCodeRegex,
  markdownFormattingRegex,
} from './build-index'

export async function searchDocumentation(query: string): Promise<SearchResult[]> {
  const normalizedQuery = query
    .trim()
    .toLowerCase()
    .replace(inlineCodeRegex, '$1')
    .replace(markdownFormattingRegex, '')
    .replace(angleBracketRegex, ' ')
    .replace(WHITESPACE_REGEX, ' ')
    .trim()
  if (!normalizedQuery) return []

  const index = await getSearchIndex(getDocsDir())

  const lowerQuery = normalizedQuery
  const words = lowerQuery.split(WHITESPACE_REGEX).filter(Boolean)

  const results: Array<SearchResult & { score: number }> = []

  for (const entry of index) {
    let score = 0

    if (entry.lowerTitle === lowerQuery) score += 100
    else if (entry.lowerTitle.startsWith(lowerQuery)) score += 50
    else if (entry.lowerTitle.includes(lowerQuery)) score += 25

    if (entry.content.includes(lowerQuery)) score += 15

    for (const word of words) {
      if (entry.lowerTitle.includes(word)) score += 10
      if (entry.content.includes(word)) score += 3
    }

    if (score > 0) {
      results.push({
        title: entry.title,
        href: entry.href,
        category: entry.category,
        excerpt: extractExcerpt(entry.originalContent, lowerQuery),
        score,
      })
    }
  }

  return results
    .sort((a, b) => b.score - a.score)
    .slice(0, 10)
    .map(({ score, ...result }) => result)
}
