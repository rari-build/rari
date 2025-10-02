async function FetchExample() {
  const response = await fetch('https://jsonplaceholder.typicode.com/posts/1')

  if (!response.ok) {
    throw new Error(`Failed to fetch: ${response.status}`)
  }

  const post = await response.json()

  const currentTime = new Date().toLocaleTimeString()

  return (
    <div style={{
      padding: '20px',
      backgroundColor: 'white',
      border: '1px solid #e5e7eb',
      borderRadius: '8px',
      boxShadow: '0 1px 3px rgba(0, 0, 0, 0.1)',
      marginBottom: '16px'
    }}>
      <h1 style={{
        fontSize: '24px',
        fontWeight: 'bold',
        color: '#1d4ed8',
        marginBottom: '8px'
      }}>
        Fetch Example (External API)
      </h1>

      <div style={{
        marginBottom: '16px',
        fontSize: '14px',
        color: '#6b7280'
      }}>
        Server time: {currentTime}
      </div>

      <div style={{
        backgroundColor: '#f9fafb',
        padding: '16px',
        borderRadius: '6px',
        border: '1px solid #e5e7eb'
      }}>
        <h2 style={{
          fontSize: '18px',
          fontWeight: '600',
          color: '#1f2937',
          marginBottom: '12px'
        }}>
          Real Post from JSONPlaceholder:
        </h2>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          <div>
            <span style={{ fontWeight: '500', color: '#374151' }}>Title:</span>
            {' '}
            {post.title}
          </div>

          <div>
            <span style={{ fontWeight: '500', color: '#374151' }}>Body:</span>
            {' '}
            {post.body}
          </div>

          <div>
            <span style={{ fontWeight: '500', color: '#374151' }}>User ID:</span>
            {' '}
            {post.userId}
          </div>

          <div>
            <span style={{ fontWeight: '500', color: '#374151' }}>Post ID:</span>
            {' '}
            {post.id}
          </div>
        </div>
      </div>

      <div style={{
        marginTop: '16px',
        fontSize: '12px',
        color: '#9ca3af'
      }}>
        This component uses async/await and fetch() to load data from an external API.
      </div>
    </div>
  )
}

export default FetchExample

