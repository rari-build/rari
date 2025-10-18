export default function ProductsPage() {
  const products = [
    { id: 1, name: 'Rari Framework', price: 'Free', description: 'Modern web framework' },
    { id: 2, name: 'RSC Renderer', price: 'Free', description: 'Server component renderer' },
    { id: 3, name: 'App Router', price: 'Free', description: 'Next.js-style routing' },
  ]

  return (
    <div className="bg-white rounded-xl p-12 shadow-2xl">
      <h1 className="text-4xl font-bold mb-8 text-blue-600">
        Our Products
      </h1>

      <div className="grid grid-cols-[repeat(auto-fit,minmax(300px,1fr))] gap-6">
        {products.map(product => (
          <div
            key={product.id}
            className="p-8 bg-gradient-to-br from-blue-600 to-cyan-600 rounded-xl text-white shadow-lg shadow-blue-400/40"
          >
            <h2 className="text-2xl mb-2 font-semibold">
              {product.name}
            </h2>
            <p className="text-xl font-bold mb-4 opacity-90">
              {product.price}
            </p>
            <p className="opacity-90">
              {product.description}
            </p>
          </div>
        ))}
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Products | Rari App Router',
  description: 'Explore our products',
}
