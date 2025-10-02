import FetchExample from '../../components/FetchExample'

export default function ServerDemoPage() {
  const serverTime = new Date().toISOString()

  return (
    <div style={{
      background: 'white',
      borderRadius: '12px',
      padding: '3rem',
      boxShadow: '0 10px 40px rgba(0, 0, 0, 0.1)',
    }}
    >
      <div style={{
        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        color: 'white',
        padding: '2rem',
        borderRadius: '12px',
        marginBottom: '2rem',
        boxShadow: '0 4px 20px rgba(102, 126, 234, 0.3)',
      }}
      >
        <h1 style={{
          fontSize: '2.5rem',
          fontWeight: 'bold',
          marginBottom: '0.5rem',
        }}
        >
          Server Components Demo
        </h1>
        <p style={{
          fontSize: '1.1rem',
          opacity: '0.9',
          marginBottom: '0.5rem',
        }}
        >
          This page demonstrates React Server Components in action
        </p>
        <p style={{
          fontSize: '0.9rem',
          opacity: '0.8',
        }}
        >
          Page rendered at: {serverTime}
        </p>
      </div>

      <div style={{ marginBottom: '2rem' }}>
        <FetchExample />
      </div>

      <div style={{
        background: '#f7fafc',
        padding: '1.5rem',
        borderRadius: '8px',
        border: '1px solid #e2e8f0',
      }}
      >
        <h2 style={{
          fontSize: '1.25rem',
          fontWeight: '600',
          marginBottom: '1rem',
          color: '#2d3748',
        }}
        >
          How Server Components Work
        </h2>
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '0.5rem',
          fontSize: '0.95rem',
          color: '#4a5568',
        }}
        >
          <p>
            ✅ <strong>Server Components</strong> run only on the server
          </p>
          <p>✅ They can access databases, files, and server-only APIs</p>
          <p>✅ They can be async and use await directly in the component</p>
          <p>✅ They reduce JavaScript bundle size sent to the client</p>
          <p>✅ They can contain Client Components (marked with 'use client')</p>
        </div>
      </div>
    </div>
  )
}
