import type { LayoutProps, Metadata } from 'rari'

export default function RootLayout({ children }: LayoutProps) {
  return (
    <div className="min-h-screen bg-gray-50">
      <nav className="bg-white border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-6">
          <div className="flex items-center gap-6 h-16">
            <a href="/" className="text-xl font-bold text-gray-900 no-underline">
              Test App
            </a>
            <a href="/about" className="text-sm text-gray-700 no-underline hover:text-gray-900">
              About
            </a>
            <a href="/nested" className="text-sm text-gray-700 no-underline hover:text-gray-900">
              Nested
            </a>
            <a href="/blog" className="text-sm text-gray-700 no-underline hover:text-gray-900">
              Blog
            </a>
            <a href="/products" className="text-sm text-gray-700 no-underline hover:text-gray-900">
              Products
            </a>
            <a href="/shop" className="text-sm text-gray-700 no-underline hover:text-gray-900">
              Shop
            </a>
            <a href="/actions" className="text-sm text-gray-700 no-underline hover:text-gray-900">
              Actions
            </a>
          </div>
        </div>
      </nav>
      <main className="max-w-7xl mx-auto px-6 py-8">{children}</main>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'Test App',
  description: 'rari test fixture app',
}
