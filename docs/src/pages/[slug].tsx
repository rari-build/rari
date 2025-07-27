import type { PageProps } from 'rari/client'
import MarkdownRenderer from '../components/MarkdownRenderer'

export default function DocPage({ params }: PageProps) {
  const slug = params?.slug

  if (typeof slug !== 'string' || slug.includes('..') || slug.includes('/')) {
    return <div>Invalid documentation path.</div>
  }

  return (
    <div className="prose prose-invert max-w-none">
      <MarkdownRenderer filePath={`${slug}.md`} />
    </div>
  )
}
