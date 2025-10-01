async function fetchData() {
  await new Promise(resolve => setTimeout(resolve, 100))

  return {
    timestamp: new Date().toISOString(),
    randomNumber: Math.floor(Math.random() * 1000),
    serverInfo: {
      nodeVersion: process.version || 'N/A',
      platform: process.platform || 'N/A',
    }
  }
}

export default async function ServerDataPage() {
  const data = await fetchData()

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
        Server Component with Async Data
      </h1>

      <div style={{
        background: '#f7fafc',
        padding: '2rem',
        borderRadius: '8px',
        border: '2px solid #e2e8f0',
        marginBottom: '2rem',
      }}>
        <h2 style={{ color: '#667eea', marginBottom: '1rem' }}>
          Data Fetched on Server
        </h2>
        <pre style={{
          background: '#2d3748',
          color: '#68d391',
          padding: '1.5rem',
          borderRadius: '4px',
          overflow: 'auto',
          fontSize: '0.9rem',
        }}>
          {JSON.stringify(data, null, 2)}
        </pre>
      </div>

      <div style={{
        background: '#edf2f7',
        padding: '1.5rem',
        borderRadius: '8px',
        border: '1px solid #cbd5e0',
      }}>
        <h3 style={{ color: '#667eea', marginBottom: '0.5rem' }}>
          🚀 How This Works
        </h3>
        <ul style={{ lineHeight: '1.8', color: '#666', paddingLeft: '1.5rem' }}>
          <li>
            <strong>This component runs only on the server</strong> - it has access to Node.js APIs like <code>process</code>
          </li>
          <li>
            <strong>The fetchData() function executes during render</strong> - before any HTML is sent to the client
          </li>
          <li>
            <strong>The client receives fully-rendered HTML</strong> - no waterfall loading or client-side fetching
          </li>
          <li>
            <strong>Refresh the page</strong> - you'll see the timestamp and random number change (server-rendered each time)
          </li>
        </ul>
      </div>

      <div style={{
        marginTop: '2rem',
        padding: '1.5rem',
        background: '#fef5e7',
        border: '2px solid #f9e79f',
        borderRadius: '8px',
      }}>
        <h3 style={{ color: '#d68910', marginBottom: '0.5rem' }}>
          💡 With RSC Wire Format
        </h3>
        <p style={{ color: '#666', lineHeight: '1.6' }}>
          If we implemented the full RSC wire format, this server component could:
        </p>
        <ul style={{ lineHeight: '1.8', color: '#666', paddingLeft: '1.5rem', marginTop: '0.5rem' }}>
          <li>Stream progressively as data loads</li>
          <li>Show Suspense boundaries during async operations</li>
          <li>Mix with client components for interactivity</li>
          <li>Refetch data without full page reload</li>
        </ul>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Server Data | Rari App Router',
  description: 'Async server component data fetching',
}

