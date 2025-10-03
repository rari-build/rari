'use client'

import type { Todo } from '../actions/todo-actions'
import { useEffect, useState, useTransition } from 'react'
import { clearCompleted, deleteTodo, toggleTodo } from '../actions/todo-actions'

interface TodoListProps {
  initialTodos: Todo[]
  onUpdate?: () => void
}

export default function TodoListWithActions({ initialTodos, onUpdate }: TodoListProps) {
  const [todos, setTodos] = useState<Todo[]>(initialTodos)
  const [isPending, startTransition] = useTransition()
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setTodos(initialTodos)
  }, [initialTodos])

  const completedCount = todos.filter(todo => todo.completed).length
  const activeCount = todos.length - completedCount

  const handleToggle = async (id: string) => {
    setError(null)
    startTransition(async () => {
      try {
        const formData = new FormData()
        formData.append('id', id)
        const result = await toggleTodo({ todos }, formData)
        if (result.success && result.todos) {
          setTodos(result.todos)
          if (onUpdate)
            onUpdate()
        }
        else if (result.error) {
          setError(result.error)
        }
      }
      catch (error) {
        const errorMsg = error instanceof Error ? error.message : 'Failed to toggle todo'
        console.error('[TodoList] Toggle error:', error)
        setError(errorMsg)
      }
    })
  }

  const handleDelete = async (id: string) => {
    setError(null)
    startTransition(async () => {
      try {
        const formData = new FormData()
        formData.append('id', id)
        const result = await deleteTodo({ todos }, formData)
        if (result.success && result.todos) {
          setTodos(result.todos)
          if (onUpdate)
            onUpdate()
        }
        else if (result.error) {
          setError(result.error)
        }
      }
      catch (error) {
        const errorMsg = error instanceof Error ? error.message : 'Failed to delete todo'
        console.error('[TodoList] Delete error:', error)
        setError(errorMsg)
      }
    })
  }

  const handleClearCompleted = async () => {
    setError(null)
    startTransition(async () => {
      try {
        const result = await clearCompleted({ todos }, new FormData())
        if (result.success && result.todos) {
          setTodos(result.todos)
          if (onUpdate)
            onUpdate()
        }
        else if (result.error) {
          setError(result.error)
        }
      }
      catch (error) {
        const errorMsg = error instanceof Error ? error.message : 'Failed to clear completed todos'
        console.error('[TodoList] Clear completed error:', error)
        setError(errorMsg)
      }
    })
  }

  return (
    <div style={{
      background: 'white',
      padding: '2rem',
      borderRadius: '0.5rem',
      border: '2px solid #667eea',
    }}
    >
      <h3 style={{ color: '#667eea', marginBottom: '1rem' }}>
        Todo List (with Server Actions)
      </h3>

      {error && (
        <div style={{
          color: '#e53e3e',
          fontSize: '0.9rem',
          padding: '0.75rem',
          background: '#fed7d7',
          borderRadius: '0.25rem',
          marginBottom: '1rem',
          border: '1px solid #fc8181',
        }}
        >
          <strong>Error:</strong>
          {' '}
          {error}
        </div>
      )}

      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: '1rem',
        padding: '1rem',
        background: '#f7fafc',
        borderRadius: '0.5rem',
        fontSize: '0.9rem',
        color: '#666',
      }}
      >
        <span>
          <strong>{activeCount}</strong>
          {' '}
          active
        </span>
        <span>
          <strong>{completedCount}</strong>
          {' '}
          completed
        </span>
        {completedCount > 0 && (
          <button
            onClick={handleClearCompleted}
            disabled={isPending}
            style={{
              padding: '0.5rem 1rem',
              background: '#e53e3e',
              color: 'white',
              border: 'none',
              borderRadius: '0.25rem',
              fontSize: '0.8rem',
              cursor: isPending ? 'not-allowed' : 'pointer',
              opacity: isPending ? 0.6 : 1,
            }}
          >
            Clear Completed
          </button>
        )}
      </div>

      <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
        {todos.map(todo => (
          <li
            key={todo.id}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '1rem',
              padding: '1rem',
              border: '1px solid #e2e8f0',
              borderRadius: '0.5rem',
              marginBottom: '0.5rem',
              background: todo.completed ? '#f7fafc' : 'white',
              opacity: isPending ? 0.7 : 1,
              transition: 'all 0.2s',
            }}
          >
            <button
              onClick={() => handleToggle(todo.id)}
              disabled={isPending}
              style={{
                width: '2rem',
                height: '2rem',
                border: '2px solid #667eea',
                borderRadius: '50%',
                background: todo.completed ? '#667eea' : 'white',
                color: todo.completed ? 'white' : '#667eea',
                cursor: isPending ? 'not-allowed' : 'pointer',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontWeight: 'bold',
                fontSize: '1rem',
              }}
            >
              {todo.completed ? '✓' : '○'}
            </button>

            <span style={{
              flex: 1,
              fontSize: '1rem',
              textDecoration: todo.completed ? 'line-through' : 'none',
              color: todo.completed ? '#999' : '#333',
            }}
            >
              {todo.text}
            </span>

            <button
              onClick={() => handleDelete(todo.id)}
              disabled={isPending}
              style={{
                width: '2rem',
                height: '2rem',
                border: '1px solid #e53e3e',
                borderRadius: '0.25rem',
                background: 'white',
                color: '#e53e3e',
                cursor: isPending ? 'not-allowed' : 'pointer',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                fontSize: '1.2rem',
                fontWeight: 'bold',
              }}
            >
              ×
            </button>
          </li>
        ))}
      </ul>

      {todos.length === 0 && (
        <div style={{
          textAlign: 'center',
          padding: '3rem',
          color: '#666',
          fontStyle: 'italic',
        }}
        >
          No todos yet. Add one above!
        </div>
      )}

      <div style={{
        fontSize: '0.85rem',
        color: '#666',
        padding: '1rem',
        background: '#f7fafc',
        borderRadius: '0.25rem',
        marginTop: '1rem',
      }}
      >
        <strong>Features demonstrated:</strong>
        <ul style={{ margin: '0.5rem 0 0 1.5rem', lineHeight: '1.6' }}>
          <li>Multiple server actions in one component</li>
          <li>Pending states during execution</li>
          <li>Optimistic UI updates</li>
          <li>State management with server actions</li>
        </ul>
      </div>
    </div>
  )
}
