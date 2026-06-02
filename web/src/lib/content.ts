import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import { extractBlogMetadata } from '@/lib/metadata'

export function isValidSlug(slug: unknown): slug is string {
  return typeof slug === 'string' && !slug.includes('..') && !slug.includes('/')
}

export function isValidSlugArray(slug: unknown): slug is string[] {
  if (!Array.isArray(slug))
    return false

  return slug.every(s => typeof s === 'string' && !s.includes('..') && !s.includes('/') && s.length > 0)
}

export function getBlogFilePath(slug: string) {
  return join(process.cwd(), 'public', 'content', 'blog', `${slug}.mdx`)
}

export function getDocsFilePath(slug: string | string[]) {
  const slugPath = Array.isArray(slug) ? slug.join('/') : slug
  return join(process.cwd(), 'public', 'content', 'docs', `${slugPath}.mdx`)
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
    const blogDir = join(process.cwd(), 'public', 'content', 'blog')
    const files = readdirSync(blogDir)
    const mdxFiles = files.filter(file => file.endsWith('.mdx'))

    const posts = mdxFiles.map((file) => {
      const slug = file.replace('.mdx', '')
      const content = readFileSync(join(blogDir, file), 'utf-8')
      const metadata = extractBlogMetadata(content)

      return {
        slug,
        title: metadata.title || 'Untitled',
        description: metadata.description || '',
        date: metadata.date || '',
        author: metadata.author,
      }
    })

    return posts.sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime())
  }
  catch {
    return []
  }
}

export function getBlogPostsMinimal(): Array<{ slug: string, date: string }> {
  try {
    const blogDir = join(process.cwd(), 'public', 'content', 'blog')
    const files = readdirSync(blogDir)
    const mdxFiles = files.filter(file => file.endsWith('.mdx'))

    return mdxFiles.map((file) => {
      const slug = file.replace('.mdx', '')
      const content = readFileSync(join(blogDir, file), 'utf-8')
      const metadata = extractBlogMetadata(content)

      return {
        slug,
        date: metadata.date || new Date().toISOString(),
      }
    })
  }
  catch {
    return []
  }
}
