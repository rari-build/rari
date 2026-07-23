import { formatDate } from '@/lib/date'

interface BlogPostCardProps {
  readonly slug: string
  readonly title: string
  readonly description: string
  readonly date: string
}

export default function BlogPostCard({ slug, title, description, date }: BlogPostCardProps) {
  const safeSlug = encodeURIComponent(slug)
  return (
    <a
      href={`/blog/${safeSlug}`}
      className="group block p-6 bg-surface border border-edge rounded-lg hover:border-accent hover:shadow-lg hover:shadow-accent/10 transition-all duration-200"
    >
      <div className="flex items-center gap-2 text-sm text-fg-muted mb-3">
        <time>{formatDate(date)}</time>
      </div>
      <h2 className="text-xl font-semibold text-fg mb-3 group-hover:text-link transition-colors">
        {title}
      </h2>
      <p className="text-fg-muted leading-relaxed">{description}</p>
    </a>
  )
}
