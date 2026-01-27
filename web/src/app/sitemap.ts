import type { Sitemap } from 'rari'
import { readdir, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import process from 'node:process'

const baseUrl = 'https://rari.build'

interface BlogPost {
  slug: string
  date: string
}

interface DocPage {
  slug: string
  lastModified: Date
}

async function getBlogPosts(): Promise<BlogPost[]> {
  try {
    const blogDir = join(process.cwd(), 'public', 'content', 'blog')
    const files = await readdir(blogDir)
    const mdxFiles = files.filter(file => file.endsWith('.mdx'))

    const posts = await Promise.all(
      mdxFiles.map(async (file) => {
        const slug = file.replace('.mdx', '')
        const content = await readFile(join(blogDir, file), 'utf-8')
        const dateMatch = content.match(/^export\s+const\s+date\s*=\s*['"](.+)['"]/m)

        return {
          slug,
          date: dateMatch?.[1] || new Date().toISOString(),
        }
      }),
    )

    return posts
  }
  catch {
    return []
  }
}

async function getDocPages(): Promise<DocPage[]> {
  const pages: DocPage[] = []

  async function scanDir(dir: string, basePath: string = '') {
    try {
      const entries = await readdir(dir, { withFileTypes: true })

      for (const entry of entries) {
        const fullPath = join(dir, entry.name)

        if (entry.isDirectory()) {
          await scanDir(fullPath, basePath ? `${basePath}/${entry.name}` : entry.name)
        }
        else if (entry.name.endsWith('.mdx')) {
          const slug = entry.name.replace('.mdx', '')
          const finalSlug = basePath ? `${basePath}/${slug}` : slug

          try {
            await readFile(fullPath, 'utf-8')
            pages.push({
              slug: finalSlug,
              lastModified: new Date(),
            })
          }
          catch {}
        }
      }
    }
    catch {}
  }

  const docsDir = join(process.cwd(), 'public', 'content', 'docs')
  await scanDir(docsDir)

  return pages
}

export default async function sitemap(): Promise<Sitemap> {
  const [blogPosts, docPages] = await Promise.all([
    getBlogPosts(),
    getDocPages(),
  ])

  return [
    {
      url: baseUrl,
      lastModified: new Date(),
      changeFrequency: 'daily',
      priority: 1,
    },
    {
      url: `${baseUrl}/docs`,
      lastModified: new Date(),
      changeFrequency: 'weekly',
      priority: 0.9,
    },
    ...docPages.map(doc => ({
      url: `${baseUrl}/docs/${doc.slug}`,
      lastModified: doc.lastModified,
      changeFrequency: 'weekly' as const,
      priority: 0.8,
    })),
    {
      url: `${baseUrl}/blog`,
      lastModified: new Date(),
      changeFrequency: 'weekly',
      priority: 0.8,
    },
    ...blogPosts.map(post => ({
      url: `${baseUrl}/blog/${post.slug}`,
      lastModified: new Date(post.date),
      changeFrequency: 'monthly' as const,
      priority: 0.7,
    })),
  ]
}
