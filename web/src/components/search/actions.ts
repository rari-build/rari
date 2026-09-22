'use server'

import type { SearchResult } from '@/lib/search/types'
import { searchDocumentation as runSearch } from '@/lib/search/query'

export async function searchDocumentation(query: string): Promise<SearchResult[]> {
  return runSearch(query)
}
