import type { LayoutProps } from 'rari/client'

export default function RootLayout({ children }: LayoutProps) {
  return (
    <html lang="en">
      <head>
        <meta charSet="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>Rari App Router Example</title>
      </head>
      <body className="min-h-screen bg-gradient-to-br from-blue-500 to-cyan-600 text-gray-900">
        <div id="root">
          <nav className="bg-white/95 px-8 py-4 shadow-md">
            <ul className="flex gap-8 list-none">
              <li><a href="/" className="text-blue-600 no-underline font-semibold hover:text-cyan-600 transition-colors">Home</a></li>
              <li><a href="/about" className="text-blue-600 no-underline font-semibold hover:text-cyan-600 transition-colors">About</a></li>
              <li><a href="/blog" className="text-blue-600 no-underline font-semibold hover:text-cyan-600 transition-colors">Blog</a></li>
              <li><a href="/products" className="text-blue-600 no-underline font-semibold hover:text-cyan-600 transition-colors">Products</a></li>
              <li><a href="/interactive" className="text-blue-600 no-underline font-semibold hover:text-cyan-600 transition-colors">Interactive</a></li>
              <li><a href="/server-data" className="text-blue-600 no-underline font-semibold hover:text-cyan-600 transition-colors">Server Data</a></li>
              <li><a href="/server-demo" className="text-blue-600 no-underline font-semibold hover:text-cyan-600 transition-colors">Server Demo</a></li>
              <li><a href="/actions" className="text-blue-600 no-underline font-semibold hover:text-cyan-600 transition-colors">Server Actions</a></li>
            </ul>
          </nav>
          <main className="max-w-7xl mx-auto p-8">{children}</main>
        </div>
      </body>
    </html>
  )
}

export const metadata = {
  title: 'Rari App Router Example',
  description: 'Testing the new app router implementation',
}
