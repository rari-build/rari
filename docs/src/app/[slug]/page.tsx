/* eslint-disable react-refresh/only-export-components */
import type { PageProps } from 'rari/client'
import { access, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import process from 'node:process'
import MarkdownRenderer from '@/components/MarkdownRenderer'
import { parseFrontmatter } from '@/lib/frontmatter'

export default function DocPage({ params }: PageProps) {
  const slug = params?.slug
  if (typeof slug !== 'string' || slug.includes('..') || slug.includes('/')) {
    return <div>Invalid documentation path.</div>
  }
  return (
    <div className="prose prose-invert max-w-none overflow-hidden">
      <MarkdownRenderer filePath={`${slug}.md`} />
    </div>
  )
}

export async function getData({ params }: PageProps) {
  const slug = params?.slug
  if (typeof slug !== 'string' || slug.includes('..') || slug.includes('/')) {
    return { notFound: true }
  }

  try {
    const filePath = join(process.cwd(), 'public', 'content', `${slug}.md`)
    await access(filePath)
    return { props: {} }
  }
  catch {
    return { notFound: true }
  }
}

export async function generateMetadata({ params }: PageProps) {
  const slug = params?.slug
  if (typeof slug !== 'string' || slug.includes('..') || slug.includes('/')) {
    return {
      title: 'Documentation | Rari',
      description: 'Complete documentation for Rari framework.',
    }
  }

  try {
    const filePath = join(process.cwd(), 'public', 'content', `${slug}.md`)
    const content = await readFile(filePath, 'utf-8')

    const { data: frontmatter, content: markdownContent } = parseFrontmatter(content)

    if (frontmatter.title || frontmatter.description) {
      return {
        title: frontmatter.title ? `${frontmatter.title} | Rari` : 'Documentation | Rari',
        description: frontmatter.description || 'Complete documentation for Rari framework.',
      }
    }

    const headingMatch = markdownContent.match(/^#\s+(\S.*)$/m)
    if (headingMatch) {
      return {
        title: `${headingMatch[1]} | Rari`,
        description: 'Complete documentation for Rari framework.',
      }
    }
  }
  catch (error) {
    console.error(`Failed to read metadata for ${slug}:`, error)
  }

  return {
    title: 'Documentation | Rari',
    description: 'Complete documentation for Rari framework.',
  }
}
