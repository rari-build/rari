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
    <div className="bg-white p-8 rounded-lg border-2 border-green-600 mb-8">
      <h3 className="text-green-600 mb-4 text-xl font-semibold">
        Progressive Enhancement Example
      </h3>

      <p className="text-gray-600 mb-6">
        This form works even without JavaScript! Try disabling JS and submitting.
      </p>

      <form ref={formRef} key={formId} className="mb-4">
        <div className="mb-4">
          <label htmlFor={`${formId}-text`} className="block mb-2 font-semibold text-gray-700">
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
            className="w-full px-3 py-3 border-2 border-gray-200 rounded-lg text-base"
          />
        </div>

        <button
          type="submit"
          disabled={status === 'submitting'}
          className={`w-full px-3 py-3 text-white border-none rounded-lg text-base font-semibold ${
            status === 'submitting'
              ? 'bg-gray-300 cursor-not-allowed'
              : 'bg-green-600 cursor-pointer hover:bg-green-700'
          }`}
        >
          {status === 'submitting' ? 'Creating...' : 'Create Todo & Redirect'}
        </button>

        {message && (
          <div
            className={`mt-4 p-3 rounded text-sm ${
              status === 'error'
                ? 'bg-red-100 text-red-600'
                : 'bg-green-100 text-green-600'
            }`}
          >
            {message}
          </div>
        )}
      </form>

      <div className="bg-blue-50 p-6 rounded-lg border border-blue-200">
        <h4 className="m-0 mb-4 text-blue-700 font-semibold">
          How it works:
        </h4>
        <ul className="m-0 pl-6 leading-loose">
          <li>
            <strong>Without JS:</strong> Form posts to{' '}
            <code className="bg-indigo-100 px-1 py-0.5 rounded font-mono text-sm">
              /api/rsc/form-action
            </code>
          </li>
          <li>
            <strong>With JS:</strong> Enhanced with client-side handling and better UX
          </li>
          <li>
            <strong>Progressive:</strong> Works in both scenarios seamlessly
          </li>
          <li>
            <strong>Redirect:</strong> Server action returns redirect instruction
          </li>
        </ul>
      </div>
    </div>
  )
}
