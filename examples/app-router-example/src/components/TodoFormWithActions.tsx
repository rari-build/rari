'use client'

import type { Todo } from '@/actions/todo-actions'
import { useActionState, useId, useState } from 'react'
import { addTodo } from '@/actions/todo-actions'

interface TodoFormProps {
  onSuccess?: () => void
}

type FormState = {
  success: boolean
  error?: string
  todos?: Todo[]
}

export default function TodoFormWithActions({ onSuccess }: TodoFormProps) {
  const formId = useId()
  const [resetKey, setResetKey] = useState(0)

  const [state, formAction, isPending] = useActionState<FormState, FormData>(
    async (_prevState, formData) => {
      const result = await addTodo(formData)
      if (result.success) {
        queueMicrotask(() => {
          setResetKey(prev => prev + 1)
          if (onSuccess)
            onSuccess()
        })
      }

      return result
    },
    { success: false, todos: [] },
  )

  return (
    <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
      <div className="flex items-center gap-2 mb-4">
        <h3 className="text-lg font-semibold text-gray-900">Add New Todo</h3>
        <span className="text-xs font-medium text-indigo-600 bg-indigo-50 px-2 py-1 rounded">
          Server Actions
        </span>
      </div>

      <form action={formAction} key={resetKey} className="space-y-4">
        <div className="flex gap-3">
          <input
            type="text"
            name="text"
            id={`${formId}-text`}
            placeholder="What needs to be done?"
            disabled={isPending}
            required
            autoComplete="off"
            className="flex-1 px-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-transparent disabled:bg-gray-50 disabled:text-gray-500"
          />
          <button
            type="submit"
            disabled={isPending}
            className={`px-6 py-2.5 text-sm font-medium text-white rounded-lg transition-all ${isPending
              ? 'bg-gray-400 cursor-not-allowed'
              : 'bg-indigo-600 hover:bg-indigo-700 active:bg-indigo-800 shadow-sm'
            }`}
          >
            {isPending
              ? (
                  <span className="flex items-center gap-2">
                    <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24">
                      <circle
                        className="opacity-25"
                        cx="12"
                        cy="12"
                        r="10"
                        stroke="currentColor"
                        strokeWidth="4"
                        fill="none"
                      >
                      </circle>
                      <path
                        className="opacity-75"
                        fill="currentColor"
                        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                      >
                      </path>
                    </svg>
                    Adding...
                  </span>
                )
              : (
                  'Add Todo'
                )}
          </button>
        </div>

        {state.error && (
          <div className="flex items-start gap-2 text-sm text-red-700 p-3 bg-red-50 border border-red-200 rounded-lg">
            <svg
              className="w-5 h-5 shrink-0 mt-0.5"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fillRule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                clipRule="evenodd"
              />
            </svg>
            <span>{state.error}</span>
          </div>
        )}

        {state.success && (
          <div className="flex items-start gap-2 text-sm text-green-700 p-3 bg-green-50 border border-green-200 rounded-lg">
            <svg
              className="w-5 h-5 shrink-0 mt-0.5"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fillRule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                clipRule="evenodd"
              />
            </svg>
            <span>Todo added successfully!</span>
          </div>
        )}
      </form>

      <div className="mt-6 pt-6 border-t border-gray-200">
        <details className="text-sm text-gray-600">
          <summary className="font-medium text-gray-700 cursor-pointer select-none hover:text-gray-900">
            How it works
          </summary>
          <ul className="mt-3 ml-4 space-y-2 text-xs leading-relaxed">
            <li className="flex items-start gap-2">
              <span className="text-indigo-600 mt-0.5">•</span>
              <span>Uses React Server Actions for server-side logic</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-indigo-600 mt-0.5">•</span>
              <span>Shows pending state while action executes</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-indigo-600 mt-0.5">•</span>
              <span>Displays errors and success messages inline</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-indigo-600 mt-0.5">•</span>
              <span>Form resets automatically on success</span>
            </li>
          </ul>
        </details>
      </div>
    </div>
  )
}
