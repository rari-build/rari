'use client'

import { createFormAction } from 'rari/runtime/actions'
import { useEffect, useId, useRef, useState } from 'react'
import { createTodoAndRedirect } from '../actions/todo-actions'

export default function ProgressiveFormExample() {
  const formId = useId()
  const formRef = useRef<HTMLFormElement>(null)
  const [status, setStatus] = useState<'idle' | 'submitting' | 'success' | 'error'>('idle')
  const [message, setMessage] = useState('')

  useEffect(() => {
    if (!formRef.current)
      return

    const formAction = createFormAction(
      'todo-actions',
      'createTodoAndRedirect',
      createTodoAndRedirect,
    )

    const cleanup = formAction.enhance(formRef.current, {
      onSuccess: () => {
        setStatus('success')
        setMessage('Todo created! Redirecting...')
      },
      onError: (error) => {
        setStatus('error')
        setMessage(`Failed to create todo: ${error}`)
        console.error('[ProgressiveForm] Error:', error)
      },
      onRedirect: (url) => {
        setStatus('success')
        setMessage(`Redirecting to ${url}...`)
      },
    })

    return cleanup
  }, [])

  return (
    <div style={{
      background: 'white',
      padding: '2rem',
      borderRadius: '0.5rem',
      border: '2px solid #10b981',
      marginBottom: '2rem',
    }}
    >
      <h3 style={{ color: '#10b981', marginBottom: '1rem' }}>
        Progressive Enhancement Example
      </h3>

      <p style={{ color: '#666', marginBottom: '1.5rem' }}>
        This form works even without JavaScript! Try disabling JS and submitting.
      </p>

      <form ref={formRef} key={formId} style={{ marginBottom: '1rem' }}>
        <div style={{ marginBottom: '1rem' }}>
          <label htmlFor={`${formId}-text`} style={{ display: 'block', marginBottom: '0.5rem', fontWeight: 600, color: '#374151' }}>
            Todo Text:
          </label>
          <input
            type="text"
            id={`${formId}-text`}
            name="text"
            placeholder="Enter todo text..."
            required
            disabled={status === 'submitting'}
            autoComplete="off"
            style={{
              width: '100%',
              padding: '0.75rem',
              border: '2px solid #e2e8f0',
              borderRadius: '0.5rem',
              fontSize: '1rem',
            }}
          />
        </div>

        <button
          type="submit"
          disabled={status === 'submitting'}
          style={{
            width: '100%',
            padding: '0.75rem',
            background: status === 'submitting' ? '#cbd5e0' : '#10b981',
            color: 'white',
            border: 'none',
            borderRadius: '0.5rem',
            fontSize: '1rem',
            fontWeight: 600,
            cursor: status === 'submitting' ? 'not-allowed' : 'pointer',
          }}
        >
          {status === 'submitting' ? 'Creating...' : 'Create Todo & Redirect'}
        </button>

        {message && (
          <div style={{
            marginTop: '1rem',
            padding: '0.75rem',
            background: status === 'error' ? '#fed7d7' : '#c6f6d5',
            color: status === 'error' ? '#e53e3e' : '#38a169',
            borderRadius: '0.25rem',
            fontSize: '0.9rem',
          }}
          >
            {message}
          </div>
        )}
      </form>

      <div style={{
        background: '#f0f9ff',
        padding: '1.5rem',
        borderRadius: '0.5rem',
        border: '1px solid #bae6fd',
      }}
      >
        <h4 style={{ margin: '0 0 1rem 0', color: '#0369a1' }}>
          How it works:
        </h4>
        <ul style={{ margin: 0, paddingLeft: '1.5rem', lineHeight: '1.8' }}>
          <li>
            <strong>Without JS:</strong>
            {' '}
            Form posts to
            <code style={{
              background: '#e0e7ff',
              padding: '0.125rem 0.25rem',
              borderRadius: '0.25rem',
              fontFamily: 'monospace',
              fontSize: '0.9rem',
            }}
            >
              /api/rsc/form-action
            </code>
          </li>
          <li>
            <strong>With JS:</strong>
            {' '}
            Enhanced with client-side handling and better UX
          </li>
          <li>
            <strong>Progressive:</strong>
            {' '}
            Works in both scenarios seamlessly
          </li>
          <li>
            <strong>Redirect:</strong>
            {' '}
            Server action returns redirect instruction
          </li>
        </ul>
      </div>
    </div>
  )
}
