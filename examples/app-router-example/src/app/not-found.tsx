export default function NotFound() {
  return (
    <div style={{
      background: 'white',
      borderRadius: '12px',
      padding: '3rem',
      boxShadow: '0 10px 40px rgba(0, 0, 0, 0.1)',
      textAlign: 'center',
    }}>
      <h1 style={{
        fontSize: '4rem',
        color: '#667eea',
        marginBottom: '1rem',
      }}>
        404
      </h1>
      <h2 style={{
        fontSize: '2rem',
        color: '#764ba2',
        marginBottom: '1rem',
      }}>
        Page Not Found
      </h2>
      <p style={{
        fontSize: '1.1rem',
        color: '#666',
        marginBottom: '2rem',
      }}>
        The page you're looking for doesn't exist.
      </p>
      <a
        href="/"
        style={{
          display: 'inline-block',
          padding: '0.75rem 1.5rem',
          background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
          color: 'white',
          textDecoration: 'none',
          borderRadius: '8px',
          fontWeight: '600',
        }}
      >
        Go Home
      </a>
    </div>
  )
}
