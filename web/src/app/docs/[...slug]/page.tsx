import type { PageProps } from 'rari'
import { readdir, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { notFound } from 'rari'
import MdxRenderer from '@/components/mdx/MdxRenderer'
import { contentFileExists, getDocsDir, getDocsFilePath, isValidSlugArray } from '@/lib/content'
import { extractMetadataWithFallback } from '@/lib/content/metadata'
import { container } from '@/lib/site/styles'

const DEFAULT_METADATA = {
  title: 'rari Docs',
  description: 'Complete documentation for rari framework.',
}

export default async function DocPage({ params }: PageProps) {
  const slug = params.slug
  if (!isValidSlugArray(slug)) notFound()
  if (!(await contentFileExists(await getDocsFilePath(slug)))) notFound()

  const slugPath = slug.join('/')
  const pathname = `/docs/${slugPath}`

  return (
    <div className={container.base}>
      <MdxRenderer filePath={`docs/${slugPath}.mdx`} pathname={pathname} />
    </div>
  )
}

export async function generateMetadata({ params }: PageProps) {
  const slug = params.slug
  if (!isValidSlugArray(slug)) return DEFAULT_METADATA

  const content = await readFile(await getDocsFilePath(slug), 'utf-8').catch(() => null)
  if (content == null) return DEFAULT_METADATA

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
}

export async function generateStaticParams() {
  const contentDir = await getDocsDir()
  const params: Array<{ slug: string[] }> = []

  async function scanDir(dir: string, segments: readonly string[]) {
    try {
      const entries = await readdir(dir, { withFileTypes: true })
      for (const entry of entries) {
        const fullPath = join(dir, entry.name)

        if (entry.isDirectory()) {
          await scanDir(fullPath, [...segments, entry.name])
        } else if (entry.name.endsWith('.mdx')) {
          const name = entry.name.replace(/\.mdx?$/, '')
          params.push({ slug: [...segments, name] })
        }
      }
    } catch {}
  }

  await scanDir(contentDir, [])
  return params
}
