'use client'

export function SiteNav() {
  return (
    <nav
      className="bg-white border-b border-gray-200"
      style={{ viewTransitionName: 'rari-site-nav' }}
      data-testid="site-nav"
    >
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
          <a href="/contact" className="text-sm text-gray-700 no-underline hover:text-gray-900">
            Contact
          </a>
          <a href="/pricing" className="text-sm text-gray-700 no-underline hover:text-gray-900">
            Pricing
          </a>
          <a href="/login" className="text-sm text-gray-700 no-underline hover:text-gray-900">
            Login
          </a>
          <a href="/signup" className="text-sm text-gray-700 no-underline hover:text-gray-900">
            Signup
          </a>
          <a href="/forgot" className="text-sm text-gray-700 no-underline hover:text-gray-900">
            Forgot
          </a>
        </div>
      </div>
    </nav>
  )
}
