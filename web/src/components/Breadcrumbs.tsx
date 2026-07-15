import { getBreadcrumbs } from '@/lib/docs-navigation'
import ChevronRight from './icons/ChevronRight'

interface BreadcrumbsProps {
  pathname: string
}

export default function Breadcrumbs({ pathname }: BreadcrumbsProps) {
  const breadcrumbs = getBreadcrumbs(pathname)

  if (breadcrumbs.length <= 1)
    return null

  return (
    <nav aria-label="Breadcrumb" className="not-prose mb-6 pt-1 pl-1 overflow-x-auto scrollbar-none">
      <ol className="flex items-center space-x-2 text-sm whitespace-nowrap">
        {breadcrumbs.map((crumb, index) => {
          const isLast = index === breadcrumbs.length - 1
          const uniqueKey = `breadcrumb-${index}-${crumb.href ?? crumb.label}`

          return (
            <li key={uniqueKey} className="flex items-center">
              {index > 0 && (
                <ChevronRight
                  className="w-4 h-4 mx-2 text-fg-muted"
                />
              )}
              {isLast || !crumb.href
                ? <span className="text-fg-muted">{crumb.label}</span>
                : (
                    <a
                      href={crumb.href}
                      className="text-fg-secondary hover:text-fg transition-colors"
                    >
                      {crumb.label}
                    </a>
                  )}
            </li>
          )
        })}
      </ol>
    </nav>
  )
}
