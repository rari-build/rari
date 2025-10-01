'use client'

import { useState } from 'react'

export default function TodoList() {
  const [todos, setTodos] = useState<Array<{ id: number, text: string, done: boolean }>>([
    { id: 1, text: 'Test RSC wire format', done: true },
    { id: 2, text: 'Add client components', done: true },
    { id: 3, text: 'Verify streaming works', done: false },
  ])
  const [newTodo, setNewTodo] = useState('')

  const addTodo = () => {
    if (newTodo.trim()) {
      setTodos([...todos, { id: Date.now(), text: newTodo, done: false }])
      setNewTodo('')
    }
  }

  const toggleTodo = (id: number) => {
    setTodos(todos.map(todo =>
      todo.id === id ? { ...todo, done: !todo.done } : todo
    ))
  }

  const deleteTodo = (id: number) => {
    setTodos(todos.filter(todo => todo.id !== id))
  }

  return (
    <div style={{
      background: 'white',
      padding: '2rem',
      borderRadius: '8px',
      border: '2px solid #667eea',
    }}>
      <h2 style={{ color: '#667eea', marginBottom: '1.5rem' }}>
        Interactive Todo List (Client Component)
      </h2>

      <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1.5rem' }}>
        <input
          type="text"
          value={newTodo}
          onChange={(e) => setNewTodo(e.target.value)}
          onKeyPress={(e) => e.key === 'Enter' && addTodo()}
          placeholder="Add a new todo..."
          style={{
            flex: 1,
            padding: '0.75rem',
            fontSize: '1rem',
            border: '2px solid #e2e8f0',
            borderRadius: '4px',
          }}
        />
        <button
          onClick={addTodo}
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
          Add
        </button>
      </div>

      <ul style={{ listStyle: 'none', padding: 0 }}>
        {todos.map(todo => (
          <li
            key={todo.id}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '1rem',
              padding: '1rem',
              background: '#f7fafc',
              borderRadius: '4px',
              marginBottom: '0.5rem',
            }}
          >
            <input
              type="checkbox"
              checked={todo.done}
              onChange={() => toggleTodo(todo.id)}
              style={{ width: '20px', height: '20px', cursor: 'pointer' }}
            />
            <span style={{
              flex: 1,
              textDecoration: todo.done ? 'line-through' : 'none',
              color: todo.done ? '#999' : '#333',
            }}>
              {todo.text}
            </span>
            <button
              onClick={() => deleteTodo(todo.id)}
              style={{
                padding: '0.5rem 1rem',
                background: '#f56565',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer',
              }}
            >
              Delete
            </button>
          </li>
        ))}
      </ul>

      <p style={{ marginTop: '1rem', color: '#666', fontSize: '0.9rem' }}>
        {todos.filter(t => !t.done).length} of {todos.length} todos remaining
      </p>
    </div>
  )
}

