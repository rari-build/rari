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
    <div key="todo-form" className="bg-white p-8 rounded-lg border-2 border-blue-600 mb-8">
      <h3 className="text-blue-600 mb-4 text-xl font-semibold">
        Add New Todo (with Server Actions)
      </h3>

      <form onSubmit={handleSubmit} key={formId} className="mb-4">
        <div className="flex gap-2 mb-4">
          <input
            type="text"
            name="text"
            id={`${formId}-text`}
            placeholder="Add a new todo..."
            disabled={isPending}
            required
            autoComplete="off"
            className="flex-1 px-3 py-3 border-2 border-gray-200 rounded-lg text-base"
          />
          <button
            type="submit"
            disabled={isPending}
            className={`px-6 py-3 text-white border-none rounded-lg text-base font-semibold ${
              isPending ? 'bg-gray-300 cursor-not-allowed' : 'bg-blue-600 cursor-pointer hover:bg-blue-700'
            }`}
          >
            {isPending ? 'Adding...' : 'Add Todo'}
          </button>
        </div>

        {state.error && (
          <div className="text-red-600 text-sm p-2 bg-red-100 rounded">
            {state.error}
          </div>
        )}

        {state.success && (
          <div className="text-green-600 text-sm p-2 bg-green-100 rounded">
            Todo added successfully!
          </div>
        )}
      </form>

      <div className="text-sm text-gray-600 p-4 bg-gray-50 rounded">
        <strong>How it works:</strong>
        <ul className="mt-2 ml-6 leading-relaxed">
          <li>Uses React Server Actions</li>
          <li>Shows pending state while action executes</li>
          <li>Displays errors and success messages</li>
          <li>Form resets automatically on success</li>
        </ul>
      </div>
    </div>
  )
}
