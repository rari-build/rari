import { join } from 'node:path'
import process from 'node:process'

export function isValidSlug(slug: unknown): slug is string {
  return typeof slug === 'string' && !slug.includes('..') && !slug.includes('/')
}

export function getBlogFilePath(slug: string) {
  return join(process.cwd(), 'public', 'content', 'blog', `${slug}.mdx`)
}

export function getDocsFilePath(slug: string) {
  return join(process.cwd(), 'public', 'content', 'docs', `${slug}.mdx`)
}
