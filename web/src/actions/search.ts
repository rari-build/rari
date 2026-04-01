'use server'

import { readdir, readFile } from 'node:fs/promises'
import { join, relative, sep } from 'node:path'
import process from 'node:process'
import { TITLE_EXPORT_REGEX, WHITESPACE_REGEX } from '@/lib/regex-constants'

export interface SearchResult {
  title: string
  href: string
  category: string
  excerpt?: string
}

const exportRegex = /^export .+$/gm
const importRegex = /^import .+$/gm
const codeBlockRegex = /<CodeBlock[^>]*>[\s\S]*?<\/CodeBlock>/gi
const terminalBlockRegex = /<TerminalBlock[^>]*\/>/gi
const packageManagerTabsRegex = /<PackageManagerTabs[^>]*\/>/gi
const pageHeaderRegex = /<PageHeader[^>]*\/>/gi
const jsxComponentRegex = /<[A-Z]\w[^>]*>([^<]*)<\/[A-Z]\w+>/g
const jsxSelfClosingRegex = /<[A-Z]\w[^>]*\/>/g
const codeBlockContentRegex = /```[\s\S]*?```/g
const inlineCodeRegex = /`([^`]+)`/g
const markdownLinkRegex = /\[([^\]]+)\]\([^)]+\)/g
const htmlTagRegex = /<[^>]+>/g
const markdownFormattingRegex = /[*_~]/g
const headingRegex = /^#{1,6}\s+/gm
const listMarkerRegex = /^(?:[-*>+]|\d+\.)\s+/gm
const propertyDefRegex = /^-\s+\*\*(?:Type|Default|Required):\*\*.+$/gm
const relatedSectionRegex = /##\s+Related[\s\S]*$/gm
const markdownTableRegex = /^\|.+\|$/gm
const mdxExtRegex = /\.mdx$/
const incompleteOpeningTagRegex = /<[^>]*$/g
const incompleteClosingTagRegex = /^[^<]*>/g

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
      }
      else if (entry.name.endsWith('.mdx')) {
        files.push(relative(baseDir, fullPath).split(sep).join('/'))
      }
    }
  }
  catch (error) {
    partial = true
    firstError = error instanceof Error ? error : new Error(String(error))
  }

  return { files, partial, error: firstError }
}

function extractContent(mdxContent: string): { title: string, content: string, originalContent: string } {
  const titleMatch = mdxContent.match(TITLE_EXPORT_REGEX)
  const title = titleMatch ? titleMatch[2] : ''

  let content = mdxContent
  let previousContent = ''
  const maxIterations = 10
  let iterations = 0

  content = content
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

  while (content !== previousContent && iterations < maxIterations) {
    previousContent = content

    content = content.replace(jsxComponentRegex, '$1')

    content = content.replace(jsxSelfClosingRegex, '')

    content = content.replace(htmlTagRegex, '')

    content = content.replace(incompleteOpeningTagRegex, '')
    content = content.replace(incompleteClosingTagRegex, '')

    iterations++
  }

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

function extractExcerpt(content: string, query: string, maxLength = 150): string {
  const lowerContent = content.toLowerCase()
  const lowerQuery = query.toLowerCase()
  const index = lowerContent.indexOf(lowerQuery)

  if (index === -1)
    return content.slice(0, maxLength)

  const start = Math.max(0, index - 50)
  const end = Math.min(content.length, index + query.length + 100)
  let excerpt = content.slice(start, end)

  if (start > 0)
    excerpt = `...${excerpt}`
  if (end < content.length)
    excerpt = `${excerpt}...`

  return excerpt
}

export async function searchDocumentation(query: string): Promise<SearchResult[]> {
  const normalizedQuery = query.trim().toLowerCase()
  if (!normalizedQuery)
    return []

  const contentDir = join(process.cwd(), 'public', 'content', 'docs')
  const { files: mdxFiles, partial, error } = await getAllMdxFiles(contentDir)

  if (partial && mdxFiles.length === 0)
    throw error ?? new Error('Failed to read documentation directory')

  if (partial)
    console.warn('Search results may be incomplete due to directory read errors:', error?.message)

  const lowerQuery = normalizedQuery
  const words = lowerQuery.split(WHITESPACE_REGEX).filter(Boolean)

  const results: Array<SearchResult & { score: number }> = []

  const fileReadPromises = mdxFiles.map(async (file) => {
    const fullPath = join(contentDir, file)
    const fileContent = await readFile(fullPath, 'utf-8')
    return { file, fileContent }
  })

  const settledResults = await Promise.allSettled(fileReadPromises)

  for (const result of settledResults) {
    if (result.status === 'rejected') {
      console.warn('Failed to read file during search:', result.reason)
      continue
    }

    const { file, fileContent } = result.value

    try {
      const { title, content, originalContent } = extractContent(fileContent)

      const lowerTitle = title.toLowerCase()
      let score = 0

      if (lowerTitle === lowerQuery)
        score += 100
      else if (lowerTitle.startsWith(lowerQuery))
        score += 50
      else if (lowerTitle.includes(lowerQuery))
        score += 25

      if (content.includes(lowerQuery))
        score += 15

      for (const word of words) {
        if (lowerTitle.includes(word))
          score += 10
        if (content.includes(word))
          score += 3
      }

      if (score > 0) {
        const href = `/docs/${file.replace('.mdx', '')}`
        const category = pathToCategory(file)

        results.push({
          title,
          href,
          category,
          excerpt: extractExcerpt(originalContent, lowerQuery),
          score,
        })
      }
    }
    catch (extractError) {
      console.warn(`Failed to extract content from ${file}:`, extractError)
    }
  }

  return results
    .sort((a, b) => b.score - a.score)
    .slice(0, 10)
    .map(({ score, ...result }) => result)
}
