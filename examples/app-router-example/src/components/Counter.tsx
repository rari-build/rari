'use client'

import { useState } from 'react'

export default function Counter() {
  const [count, setCount] = useState(0)

  return (
    <div style={{
      background: 'white',
      padding: '2rem',
      borderRadius: '8px',
      border: '2px solid #667eea',
      textAlign: 'center',
    }}>
      <h2 style={{ color: '#667eea', marginBottom: '1rem' }}>
        Interactive Counter (Client Component)
      </h2>
      <p style={{ fontSize: '3rem', margin: '1rem 0', fontWeight: 'bold', color: '#764ba2' }}>
        {count}
      </p>
      <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center' }}>
        <button
          onClick={() => setCount(count - 1)}
          style={{
            padding: '0.75rem 1.5rem',
            fontSize: '1rem',
            background: '#764ba2',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          Decrement
        </button>
        <button
          onClick={() => setCount(0)}
          style={{
            padding: '0.75rem 1.5rem',
            fontSize: '1rem',
            background: '#666',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          Reset
        </button>
        <button
          onClick={() => setCount(count + 1)}
          style={{
            padding: '0.75rem 1.5rem',
            fontSize: '1rem',
            background: '#667eea',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          Increment
        </button>
      </div>
      <p style={{ marginTop: '1rem', color: '#666', fontSize: '0.9rem' }}>
        This component runs on the client and maintains state
      </p>
    </div>
  )
}

