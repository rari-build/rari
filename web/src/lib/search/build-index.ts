import type { SearchIndexEntry } from './types'
import { readdir, readFile } from 'node:fs/promises'
import { join, relative, sep } from 'node:path'
import { TITLE_EXPORT_REGEX } from '@/lib/content/patterns'
import { WHITESPACE_REGEX } from '@/lib/utils/whitespace'

interface SearchCache {
  index: SearchIndexEntry[]
  timestamp: number
}

const exportRegex = /^export .+$/gm
const importRegex = /^import .+$/gm
const codeBlockRegex = /<CodeBlock[^>]*>[\s\S]*?<\/CodeBlock>/gi
const terminalBlockRegex = /<TerminalBlock[^>]*\/>/gi
const packageManagerTabsRegex = /<PackageManagerTabs[^>]*\/>/gi
const pageHeaderRegex = /<PageHeader[^>]*\/>/gi
const codeBlockContentRegex = /```[\s\S]*?```/g
const inlineCodeRegex = /`([^`]+)`/g
const markdownLinkRegex = /\[([^\]]+)\]\([^)]+\)/g
const anyTagRegex = /<[^>]+>/g
const markdownFormattingRegex = /[*_~]/g
const headingRegex = /^#{1,6}\s+/gm
const listMarkerRegex = /^(?:[-*>+]|\d+\.)\s+/gm
const propertyDefRegex = /^-\s+\*\*(?:Type|Default|Required):\*\*.+$/gm
const relatedSectionRegex = /##\s+Related[\s\S]*$/gm
const markdownTableRegex = /^\|.+\|$/gm
const mdxExtRegex = /\.mdx$/
const angleBracketRegex = /[<>]/g

const CACHE_TTL_MS = 5 * 60 * 1000
let searchCache: SearchCache | null = null

interface MdxFileResult {
  files: string[]
  partial: boolean
  error?: Error
}

async function getAllMdxFiles(dir: string, baseDir = dir): Promise<MdxFileResult> {
  const files: string[] = []
  let partial = false
  let firstError: Error | undefined

  try {
    const entries = await readdir(dir, { withFileTypes: true })

    for (const entry of entries) {
      const fullPath = join(dir, entry.name)
      if (entry.isDirectory()) {
        const sub = await getAllMdxFiles(fullPath, baseDir)
        files.push(...sub.files)
        if (sub.partial) {
          partial = true
          firstError = firstError ?? sub.error
        }
      } else if (entry.name.endsWith('.mdx')) {
        files.push(relative(baseDir, fullPath).split(sep).join('/'))
      }
    }
  } catch (error) {
    partial = true
    firstError = error instanceof Error ? error : new Error(String(error))
  }

  return { files, partial, error: firstError }
}

function extractContent(mdxContent: string): {
  title: string
  content: string
  originalContent: string
} {
  const titleMatch = mdxContent.match(TITLE_EXPORT_REGEX)
  const title = titleMatch ? titleMatch[2] : ''

  let content = mdxContent
    .replace(relatedSectionRegex, '')
    .replace(exportRegex, '')
    .replace(importRegex, '')
    .replace(codeBlockRegex, '')
    .replace(terminalBlockRegex, '')
    .replace(packageManagerTabsRegex, '')
    .replace(pageHeaderRegex, '')
    .replace(codeBlockContentRegex, '')
    .replace(propertyDefRegex, '')
    .replace(markdownTableRegex, '')

  let previousContent = ''
  while (content !== previousContent) {
    previousContent = content
    content = content.replace(anyTagRegex, '')
  }
  content = content.replace(angleBracketRegex, '')

  content = content
    .replace(inlineCodeRegex, '$1')
    .replace(markdownLinkRegex, '$1')
    .replace(markdownFormattingRegex, '')
    .replace(headingRegex, '')
    .replace(listMarkerRegex, '')

  content = content
    .split('\n')
    .map(line => line.trim())
    .filter(line => line.length > 0 && !line.startsWith('{') && !line.startsWith('}'))
    .join(' ')
    .replace(WHITESPACE_REGEX, ' ')
    .trim()

  if (title && content.toLowerCase().startsWith(title.toLowerCase()))
    content = content.slice(title.length).trim()

  return { title, content: content.toLowerCase(), originalContent: content }
}

function pathToCategory(path: string): string {
  const parts = path.replace(mdxExtRegex, '').split('/')
  if (parts.length > 1) {
    return parts[0]
      .split('-')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ')
  }

  return 'Documentation'
}

async function buildSearchIndex(
  contentDir: string,
): Promise<{ index: SearchIndexEntry[]; complete: boolean }> {
  const { files: mdxFiles, partial, error } = await getAllMdxFiles(contentDir)
  let complete = !partial

  if (partial && mdxFiles.length === 0) {
    console.error('Failed to read documentation directory:', error?.message ?? 'Unknown error')
    return { index: [], complete: false }
  }

  if (partial)
    console.warn('Search index may be incomplete due to directory read errors:', error?.message)

  const index: SearchIndexEntry[] = []

  const fileReadPromises = mdxFiles.map(async file => {
    const fullPath = join(contentDir, file)
    const fileContent = await readFile(fullPath, 'utf-8')
    return { file, fileContent }
  })

  const settledResults = await Promise.allSettled(fileReadPromises)

  for (const result of settledResults) {
    if (result.status === 'rejected') {
      complete = false
      console.warn('Failed to read file during index build:', result.reason)
      continue
    }

    const { file, fileContent } = result.value

    try {
      const { title, content, originalContent } = extractContent(fileContent)
      const href = `/docs/${file.replace('.mdx', '')}`
      const category = pathToCategory(file)

      index.push({
        file,
        title,
        lowerTitle: title.toLowerCase(),
        content,
        originalContent,
        href,
        category,
      })
    } catch (extractError) {
      complete = false
      console.warn(`Failed to extract content from ${file}:`, extractError)
    }
  }

  return { index, complete }
}

export async function getSearchIndex(contentDir: string): Promise<SearchIndexEntry[]> {
  const now = Date.now()

  if (searchCache && now - searchCache.timestamp < CACHE_TTL_MS) return searchCache.index

  const { index, complete } = await buildSearchIndex(contentDir)

  if (complete || !searchCache) {
    searchCache = {
      index,
      timestamp: now,
    }
  }

  return complete ? index : searchCache.index
}

export function extractExcerpt(content: string, query: string, maxLength = 150): string {
  const lowerContent = content.toLowerCase()
  const lowerQuery = query.toLowerCase()
  const index = lowerContent.indexOf(lowerQuery)

  if (index === -1) return content.slice(0, maxLength)

  const start = Math.max(0, index - 50)
  const end = Math.min(content.length, index + query.length + 100)
  let excerpt = content.slice(start, end)

  if (start > 0) excerpt = `...${excerpt}`
  if (end < content.length) excerpt = `${excerpt}...`

  return excerpt
}

export { angleBracketRegex, inlineCodeRegex, markdownFormattingRegex }
