import type { PageProps } from 'rari/client'

export default function HomePage({ params, searchParams }: PageProps) {
  return (
    <div style={{
      background: 'white',
      borderRadius: '12px',
      padding: '3rem',
      boxShadow: '0 10px 40px rgba(0, 0, 0, 0.1)',
    }}
    >
      <h1 style={{
        fontSize: '3rem',
        marginBottom: '1rem',
        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        WebkitBackgroundClip: 'text',
        WebkitTextFillColor: 'transparent',
        backgroundClip: 'text',
      }}
      >
        Welcome to Rari App Router! 🚀
      </h1>

      <p style={{
        fontSize: '1.25rem',
        color: '#666',
        marginBottom: '2rem',
        lineHeight: '1.6',
      }}
      >
        This is a demonstration of the new Next.js-style app router implementation
        in Rari. Features include:
      </p>

      <ul style={{
        fontSize: '1.1rem',
        color: '#555',
        lineHeight: '2',
        marginLeft: '2rem',
        marginBottom: '2rem',
      }}
      >
        <li>✅ File-based routing with app directory</li>
        <li>✅ Nested layouts with composition</li>
        <li>✅ Server-side rendering (SSR)</li>
        <li>✅ Loading states and error boundaries</li>
        <li>✅ Dynamic routes with params</li>
        <li>✅ Server-side data fetching</li>
        <li>✅ Metadata generation for SEO</li>
      </ul>

      <div style={{
        background: '#f7fafc',
        padding: '1.5rem',
        borderRadius: '8px',
        border: '1px solid #e2e8f0',
      }}
      >
        <h3 style={{ marginBottom: '0.5rem', color: '#667eea' }}>Debug Info:</h3>
        <pre style={{ fontSize: '0.9rem', color: '#4a5568' }}>
          {JSON.stringify({ params, searchParams }, null, 2)}
        </pre>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Home | Rari App Router',
  description: 'Welcome to the Rari app router example',
}
