import { getBreadcrumbs } from '@/lib/navigation'
import ChevronRight from './icons/ChevronRight'

interface BreadcrumbsProps {
  pathname: string
}

export default function Breadcrumbs({ pathname }: BreadcrumbsProps) {
  const breadcrumbs = getBreadcrumbs(pathname)

  if (breadcrumbs.length <= 1) {
    return null
  }

  const isGettingStarted = pathname === '/docs/getting-started'

  return (
    <nav aria-label="Breadcrumb" className="not-prose mb-6 pt-1 pl-1">
      <ol className="flex items-center space-x-2 text-sm">
        {breadcrumbs.map((crumb, index) => {
          const isLast = index === breadcrumbs.length - 1
          const isDocsLink = crumb.href === '/docs'
          const shouldDisableDocsLink = isDocsLink && isGettingStarted

          return (
            <li key={index} className="flex items-center">
              {index > 0 && (
                <ChevronRight
                  className="w-4 h-4 mx-2 text-gray-600"
                />
              )}
              {isLast || shouldDisableDocsLink
                ? (
                    <span className="text-gray-300">{crumb.label}</span>
                  )
                : crumb.href
                  ? (
                      <a
                        href={crumb.href}
                        className="text-gray-300 hover:text-white transition-colors"
                      >
                        {crumb.label}
                      </a>
                    )
                  : (
                      <span className="text-gray-300">{crumb.label}</span>
                    )}
            </li>
          )
        })}
      </ol>
    </nav>
  )
}
