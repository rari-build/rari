'use client'

import type { Todo } from '@/actions/todo-actions'
import { useActionState, useState } from 'react'
import { addTodo } from '@/actions/todo-actions'

interface FormState {
  success: boolean
  error?: string
  todos?: Todo[]
}

interface TodoFormProps {
  onSuccess?: () => void
}

export default function TodoForm({ onSuccess }: TodoFormProps) {
  const [resetKey, setResetKey] = useState(0)

  const [state, formAction, isPending] = useActionState<FormState, FormData>(
    async (_prevState, formData) => {
      const result = await addTodo(formData)
      if (result.success) {
        queueMicrotask(() => {
          setResetKey(prev => prev + 1)
          if (onSuccess) {
            onSuccess()
          }
        })
      }

      return result
    },
    { success: false, todos: [] },
  )

  return (
    <div data-testid="todo-form">
      <h2>Add Todo</h2>
      <form action={formAction} key={resetKey}>
        <input
          type="text"
          name="text"
          data-testid="todo-input"
          placeholder="Enter todo text"
          disabled={isPending}
        />
        <button
          type="submit"
          data-testid="submit-button"
          disabled={isPending}
        >
          {isPending ? 'Adding...' : 'Add Todo'}
        </button>
      </form>

      {state.error && (
        <div data-testid="error-message">{state.error}</div>
      )}

      {state.success && (
        <div data-testid="success-message">Todo added successfully!</div>
      )}

      <div data-testid="pending-state">{isPending ? 'pending' : 'idle'}</div>
    </div>
  )
}
