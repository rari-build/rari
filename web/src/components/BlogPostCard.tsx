import { formatDate } from '@/lib/date'

interface BlogPostCardProps {
  slug: string
  title: string
  description: string
  date: string
}

export default function BlogPostCard({ slug, title, description, date }: BlogPostCardProps) {
  const safeSlug = encodeURIComponent(slug)
  return (
    <a
      href={`/blog/${safeSlug}`}
      className="group block p-6 bg-[#161b22] border border-[#30363d] rounded-lg hover:border-[#fd7e14] hover:shadow-lg hover:shadow-[#fd7e14]/10 transition-all duration-200"
    >
      <div className="flex items-center gap-2 text-sm text-gray-400 mb-3">
        <time>{formatDate(date)}</time>
      </div>
      <h2 className="text-xl font-semibold text-[#f0f6fc] mb-3 group-hover:text-[#fd7e14] transition-colors">
        {title}
      </h2>
      <p className="text-gray-300 leading-relaxed">
        {description}
      </p>
    </a>
  )
}
