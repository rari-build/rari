import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import { parseDate } from '@/lib/utils'
import { extractBlogMetadata } from './metadata'

export function isValidSlug(slug: unknown): slug is string {
  return typeof slug === 'string' && !slug.includes('..') && !slug.includes('/')
}

export function isValidSlugArray(slug: unknown): slug is string[] {
  if (!Array.isArray(slug)) return false

  return slug.every(
    s => typeof s === 'string' && !s.includes('..') && !s.includes('/') && s.length > 0,
  )
}

export function getContentRoot(): string {
  const candidates = [
    join(process.cwd(), 'src', 'content'),
    join(process.cwd(), 'content'),
    join(process.cwd(), 'dist', 'content'),
  ]
  for (const dir of candidates) {
    if (existsSync(dir)) return dir
  }
  return candidates[0]
}

export function getBlogDir(): string {
  return join(getContentRoot(), 'blog')
}

export function getDocsDir(): string {
  return join(getContentRoot(), 'docs')
}

export function getBlogFilePath(slug: string) {
  return join(getBlogDir(), `${slug}.mdx`)
}

export function getDocsFilePath(slug: string | readonly string[]) {
  const slugPath = typeof slug === 'string' ? slug : slug.join('/')
  return join(getDocsDir(), `${slugPath}.mdx`)
}

export interface BlogPost {
  slug: string
  title: string
  description: string
  date: string
  author?: string
}

export function getAllBlogPosts(): BlogPost[] {
  try {
    const blogDir = getBlogDir()
    const files = readdirSync(blogDir)
    const mdxFiles = files.filter(file => file.endsWith('.mdx'))

    const posts = mdxFiles.map(file => {
      const slug = file.replace('.mdx', '')
      const content = readFileSync(join(blogDir, file), 'utf-8')
      const metadata = extractBlogMetadata(content)

      return {
        slug,
        title: metadata.title != null && metadata.title !== '' ? metadata.title : 'Untitled',
        description:
          metadata.description != null && metadata.description !== '' ? metadata.description : '',
        date: metadata.date != null && metadata.date !== '' ? metadata.date : '',
        author: metadata.author,
      }
    })

    return posts.sort((a, b) => parseDate(b.date).getTime() - parseDate(a.date).getTime())
  } catch {
    return []
  }
}

export function getBlogPostsMinimal(): Array<{ slug: string; date: string }> {
  try {
    const blogDir = getBlogDir()
    const files = readdirSync(blogDir)
    const mdxFiles = files.filter(file => file.endsWith('.mdx'))

    return mdxFiles.map(file => {
      const slug = file.replace('.mdx', '')
      const content = readFileSync(join(blogDir, file), 'utf-8')
      const metadata = extractBlogMetadata(content)

      return {
        slug,
        date:
          metadata.date != null && metadata.date !== '' ? metadata.date : new Date().toISOString(),
      }
    })
  } catch {
    return []
  }
}
