'use client'

import { useId, useState, useTransition } from 'react'
import { addTodo } from '../actions/todo-actions'

interface TodoFormProps {
  onSuccess?: () => void
}

export default function TodoFormWithActions({ onSuccess }: TodoFormProps) {
  const formId = useId()
  const [isPending, startTransition] = useTransition()
  const [state, setState] = useState<{ error?: string, success?: boolean, todos?: any[] }>({})

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    const form = e.currentTarget

    startTransition(async () => {
      try {
        const formData = new FormData(form)
        const result = await addTodo(state, formData)
        setState(result)

        if (result.success) {
          form.reset()
          if (onSuccess)
            onSuccess()
        }
      }
      catch (error) {
        setState({ error: error instanceof Error ? error.message : 'Failed to add todo' })
      }
    })
  }

  return (
    <div
      key="todo-form"
      style={{
        background: 'white',
        padding: '2rem',
        borderRadius: '0.5rem',
        border: '2px solid #667eea',
        marginBottom: '2rem',
      }}
    >
      <h3 style={{ color: '#667eea', marginBottom: '1rem' }}>
        Add New Todo (with Server Actions)
      </h3>

      <form onSubmit={handleSubmit} key={formId} style={{ marginBottom: '1rem' }}>
        <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
          <input
            type="text"
            name="text"
            id={`${formId}-text`}
            placeholder="Add a new todo..."
            disabled={isPending}
            required
            autoComplete="off"
            style={{
              flex: 1,
              padding: '0.75rem',
              border: '2px solid #e2e8f0',
              borderRadius: '0.5rem',
              fontSize: '1rem',
            }}
          />
          <button
            type="submit"
            disabled={isPending}
            style={{
              padding: '0.75rem 1.5rem',
              background: isPending ? '#cbd5e0' : '#667eea',
              color: 'white',
              border: 'none',
              borderRadius: '0.5rem',
              fontSize: '1rem',
              cursor: isPending ? 'not-allowed' : 'pointer',
              fontWeight: 600,
            }}
          >
            {isPending ? 'Adding...' : 'Add Todo'}
          </button>
        </div>

        {state.error && (
          <div style={{
            color: '#e53e3e',
            fontSize: '0.9rem',
            padding: '0.5rem',
            background: '#fed7d7',
            borderRadius: '0.25rem',
          }}
          >
            {state.error}
          </div>
        )}

        {state.success && (
          <div style={{
            color: '#38a169',
            fontSize: '0.9rem',
            padding: '0.5rem',
            background: '#c6f6d5',
            borderRadius: '0.25rem',
          }}
          >
            Todo added successfully!
          </div>
        )}
      </form>

      <div style={{
        fontSize: '0.85rem',
        color: '#666',
        padding: '1rem',
        background: '#f7fafc',
        borderRadius: '0.25rem',
      }}
      >
        <strong>How it works:</strong>
        <ul style={{ margin: '0.5rem 0 0 1.5rem', lineHeight: '1.6' }}>
          <li>Uses React Server Actions</li>
          <li>Shows pending state while action executes</li>
          <li>Displays errors and success messages</li>
          <li>Form resets automatically on success</li>
        </ul>
      </div>
    </div>
  )
}
