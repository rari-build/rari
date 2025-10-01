import Counter from '../../components/Counter'
import TodoList from '../../components/TodoList'

export default function InteractivePage() {
  return (
    <div style={{
      display: 'grid',
      gap: '2rem',
    }}>
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
          Interactive Components Test
        </h1>
        <p style={{
          fontSize: '1.1rem',
          color: '#666',
          marginBottom: '2rem',
        }}>
          This page demonstrates mixing server and client components using RSC.
          The components below are client-side React components that maintain state.
        </p>
      </div>

      <Counter />

      <TodoList />

      <div style={{
        background: 'white',
        borderRadius: '12px',
        padding: '2rem',
        boxShadow: '0 10px 40px rgba(0, 0, 0, 0.1)',
      }}>
        <h3 style={{ color: '#667eea', marginBottom: '1rem' }}>Server Component Info</h3>
        <p style={{ color: '#666', lineHeight: '1.6' }}>
          This text is rendered on the server at{' '}
          <strong>{new Date().toISOString()}</strong>.
          The Counter and TodoList above are client components that hydrate on the client.
        </p>
        <p style={{ color: '#666', lineHeight: '1.6', marginTop: '1rem' }}>
          Try refreshing the page - the timestamp will update (server-rendered),
          but the counter and todo state will reset (client-rendered).
        </p>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Interactive Components | Rari App Router',
  description: 'Testing client components with RSC',
}

