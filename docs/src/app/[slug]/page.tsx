import type { PageProps } from 'rari/client'
import { access, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import process from 'node:process'
import MdxRenderer from '@/components/MdxRenderer'

export default function DocPage({ params }: PageProps) {
  const slug = params?.slug
  if (typeof slug !== 'string' || slug.includes('..') || slug.includes('/')) {
    return <div>Invalid documentation path.</div>
  }
  return (
    <div className="prose prose-invert max-w-none overflow-hidden">
      <MdxRenderer filePath={`${slug}.mdx`} />
    </div>
  )
}

export async function getData({ params }: PageProps) {
  const slug = params?.slug
  if (typeof slug !== 'string' || slug.includes('..') || slug.includes('/')) {
    return { notFound: true }
  }

  try {
    const filePath = join(process.cwd(), 'public', 'content', `${slug}.mdx`)
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
    const filePath = join(process.cwd(), 'public', 'content', `${slug}.mdx`)
    const content = await readFile(filePath, 'utf-8')

    const titleMatch = content.match(/^export\s+const\s+title\s*=\s*['"](.+)['"]/m)
    const descriptionMatch = content.match(/^export\s+const\s+description\s*=\s*['"](.+)['"]/m)

    if (titleMatch || descriptionMatch) {
      return {
        title: titleMatch ? `${titleMatch[1]} | Rari` : 'Documentation | Rari',
        description: descriptionMatch ? descriptionMatch[1] : 'Complete documentation for Rari framework.',
      }
    }

    const headingMatch = content.match(/^#\s+(\S.*)$/m)
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
