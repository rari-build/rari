'use client'

import type { Todo, TodoActionState } from '@/actions/todo-actions'
import { useActionState, useEffect, useRef } from 'react'
import { addTodo } from '@/actions/todo-actions'

interface TodoFormProps {
  readonly onSuccess?: (todos?: readonly Todo[]) => void
}

export default function TodoForm({ onSuccess }: TodoFormProps) {
  const [state, formAction, isPending] = useActionState<TodoActionState, FormData>(addTodo, {
    success: false,
    todos: [],
  })
  const lastSuccessCountRef = useRef(0)

  useEffect(() => {
    if (!state.success || !state.todos) return

    if (state.todos.length === lastSuccessCountRef.current) return

    lastSuccessCountRef.current = state.todos.length
    onSuccess?.(state.todos)
  }, [state.success, state.todos, onSuccess])

  return (
    <div data-testid="todo-form">
      <h2>Add Todo</h2>
      <form action={formAction}>
        <input
          type="text"
          name="text"
          data-testid="todo-input"
          placeholder="Enter todo text"
          disabled={isPending}
        />
        <button type="submit" data-testid="submit-button" disabled={isPending}>
          {isPending ? 'Adding...' : 'Add Todo'}
        </button>
      </form>

      {state.error != null && state.error !== '' && (
        <div data-testid="error-message">{state.error}</div>
      )}

      {state.success && <div data-testid="success-message">Todo added successfully!</div>}

      <div data-testid="pending-state">{isPending ? 'pending' : 'idle'}</div>
    </div>
  )
}
