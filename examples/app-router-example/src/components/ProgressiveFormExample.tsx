'use client'

import { createFormAction } from 'rari/runtime/actions'
import { useEffect, useId, useRef, useState } from 'react'
import { createTodoAndRedirect } from '@/actions/todo-actions'

export default function ProgressiveFormExample() {
  const formId = useId()
  const formRef = useRef<HTMLFormElement>(null)
  const [status, setStatus] = useState<
    'idle' | 'submitting' | 'success' | 'error'
  >('idle')
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
    <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
      <div className="flex items-center gap-2 mb-4">
        <h3 className="text-lg font-semibold text-gray-900">
          Progressive Enhancement
        </h3>
        <span className="text-xs font-medium text-emerald-600 bg-emerald-50 px-2 py-1 rounded">
          Works without JS
        </span>
      </div>

      <p className="text-sm text-gray-600 mb-6 leading-relaxed">
        This form works even without JavaScript! Try disabling JS in your
        browser and submitting the form. It will still work perfectly.
      </p>

      <form ref={formRef} key={formId} className="space-y-4">
        <div>
          <label
            htmlFor={`${formId}-text`}
            className="block text-sm font-medium text-gray-700 mb-2"
          >
            Todo Text
          </label>
          <input
            type="text"
            id={`${formId}-text`}
            name="text"
            placeholder="Enter your todo..."
            required
            disabled={status === 'submitting'}
            autoComplete="off"
            className="w-full px-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-transparent disabled:bg-gray-50 disabled:text-gray-500"
          />
        </div>

        <button
          type="submit"
          disabled={status === 'submitting'}
          className={`w-full px-4 py-2.5 text-sm font-medium text-white rounded-lg transition-all ${status === 'submitting'
            ? 'bg-gray-400 cursor-not-allowed'
            : 'bg-emerald-600 hover:bg-emerald-700 active:bg-emerald-800 shadow-sm'
          }`}
        >
          {status === 'submitting'
            ? (
                <span className="flex items-center justify-center gap-2">
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
                  Creating...
                </span>
              )
            : (
                'Create Todo & Redirect'
              )}
        </button>

        {message && (
          <div
            className={`flex items-start gap-2 text-sm p-3 rounded-lg border ${status === 'error'
              ? 'bg-red-50 border-red-200 text-red-700'
              : 'bg-emerald-50 border-emerald-200 text-emerald-700'
            }`}
          >
            <svg
              className="w-5 h-5 shrink-0 mt-0.5"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              {status === 'error'
                ? (
                    <path
                      fillRule="evenodd"
                      d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                      clipRule="evenodd"
                    />
                  )
                : (
                    <path
                      fillRule="evenodd"
                      d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                      clipRule="evenodd"
                    />
                  )}
            </svg>
            <span>{message}</span>
          </div>
        )}
      </form>

      <div className="mt-6 pt-6 border-t border-gray-200">
        <details className="text-sm text-gray-600">
          <summary className="font-medium text-gray-700 cursor-pointer select-none hover:text-gray-900">
            How progressive enhancement works
          </summary>
          <div className="mt-4 space-y-3">
            <div className="flex gap-3">
              <div className="shrink-0 w-6 h-6 bg-gray-100 rounded flex items-center justify-center">
                <span className="text-xs font-semibold text-gray-600">1</span>
              </div>
              <div>
                <div className="font-medium text-gray-900 text-xs mb-1">
                  Without JavaScript
                </div>
                <p className="text-xs text-gray-600 leading-relaxed">
                  Form posts to
                  {' '}
                  <code className="bg-gray-100 px-1.5 py-0.5 rounded font-mono">
                    /api/rsc/form-action
                  </code>
                  {' '}
                  endpoint using standard HTML form submission
                </p>
              </div>
            </div>

            <div className="flex gap-3">
              <div className="shrink-0 w-6 h-6 bg-gray-100 rounded flex items-center justify-center">
                <span className="text-xs font-semibold text-gray-600">2</span>
              </div>
              <div>
                <div className="font-medium text-gray-900 text-xs mb-1">
                  With JavaScript
                </div>
                <p className="text-xs text-gray-600 leading-relaxed">
                  Enhanced with client-side handling for better UX, loading
                  states, and inline error messages
                </p>
              </div>
            </div>

            <div className="flex gap-3">
              <div className="shrink-0 w-6 h-6 bg-gray-100 rounded flex items-center justify-center">
                <span className="text-xs font-semibold text-gray-600">3</span>
              </div>
              <div>
                <div className="font-medium text-gray-900 text-xs mb-1">
                  Server Redirect
                </div>
                <p className="text-xs text-gray-600 leading-relaxed">
                  After successful submission, the server action returns a
                  redirect instruction to navigate to the actions page
                </p>
              </div>
            </div>
          </div>
        </details>
      </div>
    </div>
  )
}
