export default function AboutPage() {
  return (
    <div style={{
      background: 'white',
      borderRadius: '12px',
      padding: '3rem',
      boxShadow: '0 10px 40px rgba(0, 0, 0, 0.1)',
    }}>
      <h1 style={{
        fontSize: '2.5rem',
        marginBottom: '1rem',
        color: '#667eea',
      }}>
        About This Example
      </h1>

      <p style={{
        fontSize: '1.1rem',
        color: '#666',
        marginBottom: '1.5rem',
        lineHeight: '1.8',
      }}>
        This example demonstrates the Rari app router, a server-first routing
        system inspired by Next.js App Router.
      </p>

      <h2 style={{
        fontSize: '1.75rem',
        marginBottom: '1rem',
        marginTop: '2rem',
        color: '#764ba2',
      }}>
        Key Features
      </h2>

      <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fit, minmax(250px, 1fr))',
        gap: '1.5rem',
        marginTop: '1.5rem',
      }}>
        <div style={{
          padding: '1.5rem',
          background: '#f7fafc',
          borderRadius: '8px',
          border: '2px solid #e2e8f0',
        }}>
          <h3 style={{ color: '#667eea', marginBottom: '0.5rem' }}>Server Components</h3>
          <p style={{ color: '#666', fontSize: '0.95rem' }}>
            React Server Components rendered on the server for better performance
          </p>
        </div>

        <div style={{
          padding: '1.5rem',
          background: '#f7fafc',
          borderRadius: '8px',
          border: '2px solid #e2e8f0',
        }}>
          <h3 style={{ color: '#667eea', marginBottom: '0.5rem' }}>Nested Layouts</h3>
          <p style={{ color: '#666', fontSize: '0.95rem' }}>
            Share UI across routes with nested layout composition
          </p>
        </div>

        <div style={{
          padding: '1.5rem',
          background: '#f7fafc',
          borderRadius: '8px',
          border: '2px solid #e2e8f0',
        }}>
          <h3 style={{ color: '#667eea', marginBottom: '0.5rem' }}>File-Based Routing</h3>
          <p style={{ color: '#666', fontSize: '0.95rem' }}>
            Automatic routing based on your file structure
          </p>
        </div>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'About | Rari App Router',
  description: 'Learn about the Rari app router implementation',
}
