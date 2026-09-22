'use server'

import type { SearchResult } from '@/lib/search'
import { searchDocumentation as runSearch } from '@/lib/search'

export async function searchDocumentation(query: string): Promise<SearchResult[]> {
  return runSearch(query)
}
