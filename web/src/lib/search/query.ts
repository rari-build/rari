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

function scoreSearchEntry(
  entry: {
    readonly lowerTitle: string
    readonly content: string
  },
  lowerQuery: string,
  words: readonly string[],
): number {
  let score = 0

  if (entry.lowerTitle === lowerQuery) score += 100
  else if (entry.lowerTitle.startsWith(lowerQuery)) score += 50
  else if (entry.lowerTitle.includes(lowerQuery)) score += 25

  if (entry.content.includes(lowerQuery)) score += 15

  for (const word of words) {
    if (entry.lowerTitle.includes(word)) score += 10
    if (entry.content.includes(word)) score += 3
  }

  return score
}

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
  const words = normalizedQuery.split(WHITESPACE_REGEX).filter(Boolean)
  const results: Array<SearchResult & { score: number }> = []

  for (const entry of index) {
    const score = scoreSearchEntry(entry, normalizedQuery, words)
    if (score <= 0) continue
    results.push({
      title: entry.title,
      href: entry.href,
      category: entry.category,
      excerpt: extractExcerpt(entry.originalContent, normalizedQuery),
      score,
    })
  }

  return results
    .sort((a, b) => b.score - a.score)
    .slice(0, 10)
    .map(({ score, ...result }) => result)
}
