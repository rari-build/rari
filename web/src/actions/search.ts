'use server'

import { readdir, readFile } from 'node:fs/promises'
import { join } from 'node:path'
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
const listMarkerRegex = /^[->]\s+/gm
const propertyDefRegex = /^-\s+\*\*(?:Type|Default|Required):\*\*.+$/gm
const relatedSectionRegex = /##\s+Related[\s\S]*$/gm
const markdownTableRegex = /^\|.+\|$/gm
const mdxExtRegex = /\.mdx$/
const leadingSlashRegex = /^\//

async function getAllMdxFiles(dir: string, baseDir = dir): Promise<string[]> {
  const files: string[] = []
  try {
    const entries = await readdir(dir, { withFileTypes: true })

    for (const entry of entries) {
      const fullPath = join(dir, entry.name)
      if (entry.isDirectory()) {
        files.push(...await getAllMdxFiles(fullPath, baseDir))
      }
      else if (entry.name.endsWith('.mdx')) {
        files.push(fullPath.replace(baseDir, '').replace(leadingSlashRegex, ''))
      }
    }
  }
  catch {}

  return files
}

function extractContent(mdxContent: string): { title: string, content: string, originalContent: string } {
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
    .replace(jsxComponentRegex, '$1')
    .replace(jsxSelfClosingRegex, '')
    .replace(codeBlockContentRegex, '')
    .replace(inlineCodeRegex, '$1')
    .replace(propertyDefRegex, '')
    .replace(markdownTableRegex, '')
    .replace(markdownLinkRegex, '$1')
    .replace(htmlTagRegex, '')
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
  if (!query.trim())
    return []

  const contentDir = join(process.cwd(), 'public', 'content', 'docs')
  const mdxFiles = await getAllMdxFiles(contentDir)

  const lowerQuery = query.toLowerCase()
  const words = lowerQuery.split(WHITESPACE_REGEX)

  const results: Array<SearchResult & { score: number }> = []

  for (const file of mdxFiles) {
    const fullPath = join(contentDir, file)
    const fileContent = await readFile(fullPath, 'utf-8')
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

  return results
    .sort((a, b) => b.score - a.score)
    .slice(0, 10)
    .map(({ score, ...result }) => result)
}
