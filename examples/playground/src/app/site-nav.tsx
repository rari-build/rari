'use client'

const LINKS = [
  { href: '/', label: 'Home' },
  { href: '/about', label: 'About' },
  { href: '/blog', label: 'Blog' },
  { href: '/interactive', label: 'Interactive' },
  { href: '/server-data', label: 'Server Data' },
  { href: '/server-demo', label: 'Server Demo' },
  { href: '/actions', label: 'Actions' },
  { href: '/react-19', label: 'React 19.3' },
] as const

export function SiteNav() {
  return (
    <nav className="bg-white border-b border-gray-200 sticky top-0 z-50 shadow-sm">
      <div className="max-w-7xl mx-auto px-6">
        <div className="flex items-center justify-between h-16">
          <div className="flex items-center gap-2">
            <span className="text-2xl font-bold text-gray-900">rari</span>
          </div>
          <ul className="flex gap-1 list-none m-0">
            {LINKS.map(link => (
              <li key={link.href}>
                <a
                  href={link.href}
                  className="px-4 py-2 text-sm font-medium text-gray-700 no-underline hover:text-gray-900 hover:bg-gray-50 rounded-md transition-colors"
                >
                  {link.label}
                </a>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </nav>
  )
}
