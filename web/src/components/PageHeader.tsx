import type { ReactNode } from 'react'
import { formatBlogDate, formatDate } from '@/lib/date'
import LastUpdated from './LastUpdated'

interface PageHeaderProps {
  title: string
  filePath?: string
  pagePath?: string
  date?: string
  author?: string
  authorUrl?: string
  tags?: string[]
  children?: ReactNode
}

export default function PageHeader({ title, filePath, pagePath, date, author, authorUrl, tags, children }: PageHeaderProps) {
  const repoPath = filePath ? `web/public/content/${filePath}` : pagePath
  const isBlog = filePath?.startsWith('blog/') ?? false

  if (isBlog) {
    return (
      <header className="page-header mb-8 pb-8 border-b border-edge">
        {date && (
          <time
            dateTime={date}
            className="block text-sm font-medium text-fg-muted mb-4"
          >
            {formatBlogDate(date)}
          </time>
        )}
        <h1 className="font-bold text-fg leading-[1.1] text-4xl md:text-5xl lg:text-6xl mb-6 tracking-tight">
          {title}
        </h1>
        {author && (
          <div className="flex items-center gap-1 text-base text-fg-muted">
            <span>Posted by</span>
            {authorUrl
              ? (
                  <a
                    href={authorUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-fg-secondary font-medium hover:text-link transition-colors"
                  >
                    {author}
                  </a>
                )
              : (
                  <span className="text-fg-secondary font-medium">{author}</span>
                )}
          </div>
        )}
        {tags && tags.length > 0 && (
          <div className="flex flex-wrap gap-2 mt-6">
            {tags.map(tag => (
              <span
                key={tag}
                className="px-3 py-1.5 text-sm font-medium rounded-md bg-muted border border-edge text-fg-muted hover:border-accent transition-colors"
              >
                {tag}
              </span>
            ))}
          </div>
        )}
        {children}
      </header>
    )
  }

  return (
    <header className="page-header mb-8">
      <h1 className="font-semibold text-3xl text-fg leading-tight mb-4">
        {title}
      </h1>
      {(date || author) && (
        <div className="flex items-center gap-3 text-sm text-fg-muted mb-4">
          {author && <span className="text-fg-muted font-medium">{author}</span>}
          {author && date && <span className="text-edge">·</span>}
          {date && <time dateTime={date}>{formatDate(date)}</time>}
        </div>
      )}
      {tags && tags.length > 0 && (
        <div className="flex flex-wrap gap-2 mb-4">
          {tags.map(tag => (
            <span
              key={tag}
              className="px-2.5 py-0.5 text-xs font-medium rounded-full bg-muted border border-edge text-fg-muted"
            >
              {tag}
            </span>
          ))}
        </div>
      )}
      {repoPath && <LastUpdated filePath={repoPath} />}
      {children}
    </header>
  )
}
