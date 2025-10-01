export default function ProductsPage() {
  const products = [
    { id: 1, name: 'Rari Framework', price: 'Free', description: 'Modern web framework' },
    { id: 2, name: 'RSC Renderer', price: 'Free', description: 'Server component renderer' },
    { id: 3, name: 'App Router', price: 'Free', description: 'Next.js-style routing' },
  ]

  return (
    <div style={{
      background: 'white',
      borderRadius: '12px',
      padding: '3rem',
      boxShadow: '0 10px 40px rgba(0, 0, 0, 0.1)',
    }}>
      <h1 style={{
        fontSize: '2.5rem',
        marginBottom: '2rem',
        color: '#667eea',
      }}>
        Our Products
      </h1>

      <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))',
        gap: '1.5rem',
      }}>
        {products.map(product => (
          <div
            key={product.id}
            style={{
              padding: '2rem',
              background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
              borderRadius: '12px',
              color: 'white',
              boxShadow: '0 4px 15px rgba(102, 126, 234, 0.4)',
            }}
          >
            <h2 style={{
              fontSize: '1.5rem',
              marginBottom: '0.5rem',
            }}>
              {product.name}
            </h2>
            <p style={{
              fontSize: '1.25rem',
              fontWeight: 'bold',
              marginBottom: '1rem',
              opacity: 0.9,
            }}>
              {product.price}
            </p>
            <p style={{
              opacity: 0.9,
            }}>
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
