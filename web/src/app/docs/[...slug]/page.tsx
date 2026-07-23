import type { PageProps } from 'rari'
import { accessSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import MdxRenderer from '@/components/MdxRenderer'
import { getDocsFilePath, isValidSlugArray } from '@/lib/content'
import { extractMetadataWithFallback } from '@/lib/metadata'
import { container } from '@/lib/styles'

const DEFAULT_METADATA = {
  title: 'rari Docs',
  description: 'Complete documentation for rari framework.',
}

export default function DocPage({ params }: PageProps) {
  const slug = params.slug

  if (!isValidSlugArray(slug))
    return <div className={container.base}>Invalid documentation path.</div>

  const slugPath = slug.join('/')
  const pathname = `/docs/${slugPath}`

  return (
    <div className={container.base}>
      <MdxRenderer filePath={`docs/${slugPath}.mdx`} pathname={pathname} />
    </div>
  )
}

export function getData({ params }: PageProps) {
  const slug = params.slug

  if (!isValidSlugArray(slug)) return { notFound: true }

  try {
    accessSync(getDocsFilePath(slug))
    return { props: {} }
  } catch {
    return { notFound: true }
  }
}

export function generateMetadata({ params }: PageProps) {
  const slug = params.slug

  if (!isValidSlugArray(slug)) return DEFAULT_METADATA

  try {
    const content = readFileSync(getDocsFilePath(slug), 'utf-8')
    const metadata = extractMetadataWithFallback(content)

    const pageTitle =
      metadata.title != null && metadata.title !== ''
        ? `${metadata.title} / rari Docs`
        : DEFAULT_METADATA.title
    const pageDescription = metadata.description ?? DEFAULT_METADATA.description

    return {
      title: pageTitle,
      description: pageDescription,
      openGraph: {
        title: pageTitle,
        description: pageDescription,
      },
    }
  } catch {}

  return DEFAULT_METADATA
}

export function generateStaticParams() {
  const contentDir = join(process.cwd(), 'public', 'content', 'docs')
  const params: Array<{ slug: string[] }> = []

  function scanDir(dir: string, segments: readonly string[]) {
    try {
      const entries = readdirSync(dir)
      for (const entry of entries) {
        const fullPath = join(dir, entry)
        const stat = statSync(fullPath)

        if (stat.isDirectory()) {
          scanDir(fullPath, [...segments, entry])
        } else if (entry.endsWith('.mdx')) {
          const name = entry.replace(/\.mdx?$/, '')
          params.push({ slug: [...segments, name] })
        }
      }
    } catch {}
  }

  scanDir(contentDir, [])
  return params
}
