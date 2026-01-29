/* eslint-disable style/object-curly-spacing */
import type { LayoutProps, Metadata } from 'rari'

export default function RootLayout({ children }: LayoutProps) {
  return (
    <div className="min-h-screen">
      <nav className="bg-white border-b border-gray-200 sticky top-0 z-50 shadow-sm">
        <div className="max-w-7xl mx-auto px-6">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-2">
              <span className="text-2xl font-bold text-gray-900">{{PROJECT_NAME}}</span>
            </div>
            <ul className="flex gap-1 list-none m-0">
              <li>
                <a
                  href="/"
                  className="px-4 py-2 text-sm font-medium text-gray-700 no-underline hover:text-gray-900 hover:bg-gray-50 rounded-md transition-colors"
                >
                  Home
                </a>
              </li>
              <li>
                <a
                  href="/about"
                  className="px-4 py-2 text-sm font-medium text-gray-700 no-underline hover:text-gray-900 hover:bg-gray-50 rounded-md transition-colors"
                >
                  About
                </a>
              </li>
            </ul>
          </div>
        </div>
      </nav>
      <main className="max-w-7xl mx-auto px-6 py-8">{children}</main>
    </div>
  )
}

export const metadata: Metadata = {
  title: '{{PROJECT_NAME}}',
  description: 'A rari application',
}
