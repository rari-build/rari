import { join } from 'node:path'
import process from 'node:process'

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
