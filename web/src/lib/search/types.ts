export interface SearchResult {
  title: string
  href: string
  category: string
  excerpt?: string
}

export interface SearchIndexEntry {
  file: string
  title: string
  lowerTitle: string
  content: string
  originalContent: string
  href: string
  category: string
}
