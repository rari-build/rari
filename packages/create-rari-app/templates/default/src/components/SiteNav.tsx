const LINKS = [
  { href: '/', label: 'Home' },
  { href: '/about', label: 'About' },
] as const

export function SiteNav() {
  return (
    <nav className="bg-white border-b border-gray-200 sticky top-0 z-50 shadow-sm">
      <div className="max-w-7xl mx-auto px-6 flex items-center justify-between h-16">
        <a href="/" className="text-2xl font-bold text-gray-900 no-underline">
          {'{{PROJECT_NAME}}'}
        </a>
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
    </nav>
  )
}
