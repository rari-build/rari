import type { Metadata } from 'rari'

export default async function ProductsPage() {
  return (
    <div>
      <h1>Products</h1>
      <p>Browse our products by category.</p>
      <nav className="space-y-2 mt-4">
        <a href="/products/electronics/laptop-123" className="block text-blue-600 hover:underline">
          Electronics - Laptop 123
        </a>
        <a href="/products/books/novel-456" className="block text-blue-600 hover:underline">
          Books - Novel 456
        </a>
      </nav>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'Products',
  description: 'All products',
}
