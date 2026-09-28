import { access, readdir, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import process from 'node:process'
import { parseDate } from '@/lib/utils/date'
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

export async function getContentRoot(): Promise<string> {
  const candidates = [
    join(process.cwd(), 'src', 'content'),
    join(process.cwd(), 'content'),
    join(process.cwd(), 'dist', 'content'),
  ]
  for (const dir of candidates) {
    try {
      await access(dir)
      return dir
    } catch {}
  }
  return candidates[0]
}

export async function getBlogDir(): Promise<string> {
  return join(await getContentRoot(), 'blog')
}

export async function getDocsDir(): Promise<string> {
  return join(await getContentRoot(), 'docs')
}

export async function getBlogFilePath(slug: string): Promise<string> {
  return join(await getBlogDir(), `${slug}.mdx`)
}

export async function getDocsFilePath(slug: string | readonly string[]): Promise<string> {
  const slugPath = typeof slug === 'string' ? slug : slug.join('/')
  return join(await getDocsDir(), `${slugPath}.mdx`)
}

export async function contentFileExists(filePath: string): Promise<boolean> {
  try {
    await access(filePath)
    return true
  } catch {
    return false
  }
}

export async function readContentFile(filePath: string): Promise<string | null> {
  try {
    return await readFile(filePath, 'utf-8')
  } catch {
    return null
  }
}

export interface BlogPost {
  slug: string
  title: string
  description: string
  date: string
  author?: string
}

export async function getAllBlogPosts(): Promise<BlogPost[]> {
  try {
    const blogDir = await getBlogDir()
    const files = await readdir(blogDir)
    const mdxFiles = files.filter(file => file.endsWith('.mdx'))

    const posts = await Promise.all(
      mdxFiles.map(async file => {
        const slug = file.replace('.mdx', '')
        const content = await readFile(join(blogDir, file), 'utf-8')
        const metadata = extractBlogMetadata(content)

        return {
          slug,
          title: metadata.title != null && metadata.title !== '' ? metadata.title : 'Untitled',
          description:
            metadata.description != null && metadata.description !== '' ? metadata.description : '',
          date: metadata.date != null && metadata.date !== '' ? metadata.date : '',
          author: metadata.author,
        }
      }),
    )

    return posts.sort((a, b) => parseDate(b.date).getTime() - parseDate(a.date).getTime())
  } catch {
    return []
  }
}

export async function getBlogPostsMinimal(): Promise<Array<{ slug: string; date: string }>> {
  try {
    const blogDir = await getBlogDir()
    const files = await readdir(blogDir)
    const mdxFiles = files.filter(file => file.endsWith('.mdx'))

    return await Promise.all(
      mdxFiles.map(async file => {
        const slug = file.replace('.mdx', '')
        const content = await readFile(join(blogDir, file), 'utf-8')
        const metadata = extractBlogMetadata(content)

        return {
          slug,
          date:
            metadata.date != null && metadata.date !== ''
              ? metadata.date
              : new Date().toISOString(),
        }
      }),
    )
  } catch {
    return []
  }
}
