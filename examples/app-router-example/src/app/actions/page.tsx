import { getTodos } from '@/actions/todo-actions'
import ProgressiveFormExample from '@/components/ProgressiveFormExample'
import TodoAppWithActions from '@/components/TodoAppWithActions'

export default async function ActionsPage() {
  const initialTodos = await getTodos()

  return (
    <div className="space-y-8">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 md:p-12">
        <div className="flex items-center gap-3 mb-4">
          <h1 className="text-4xl font-bold text-gray-900">
            React Server Actions Demo
          </h1>
          <span className="text-3xl">⚡</span>
        </div>
        <p className="text-lg text-gray-600 max-w-3xl leading-relaxed">
          This page demonstrates React Server Actions working with Rari. All
          patterns follow React's official server function specifications.
        </p>
      </div>

      <div>
        <h2 className="text-2xl font-bold text-gray-900 mb-4">
          Interactive Todo Application
        </h2>
        <TodoAppWithActions initialTodos={initialTodos} />
      </div>

      <div>
        <h2 className="text-2xl font-bold text-gray-900 mb-4">
          Progressive Enhancement Form
        </h2>
        <ProgressiveFormExample />
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <h2 className="text-2xl font-bold text-gray-900 mb-6">
          Server Action Patterns Demonstrated
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div className="p-5 bg-green-50 rounded-lg border border-green-200">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-green-600 text-xl">✓</span>
              <h3 className="text-green-900 font-semibold">
                useActionState Hook
              </h3>
            </div>
            <p className="text-sm text-gray-700 leading-relaxed">
              Manage server action state with pending states and error handling.
            </p>
          </div>

          <div className="p-5 bg-green-50 rounded-lg border border-green-200">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-green-600 text-xl">✓</span>
              <h3 className="text-green-900 font-semibold">
                useTransition Hook
              </h3>
            </div>
            <p className="text-sm text-gray-700 leading-relaxed">
              Track pending states across multiple actions for better UX.
            </p>
          </div>

          <div className="p-5 bg-green-50 rounded-lg border border-green-200">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-green-600 text-xl">✓</span>
              <h3 className="text-green-900 font-semibold">Form Actions</h3>
            </div>
            <p className="text-sm text-gray-700 leading-relaxed">
              Server functions that work with HTML forms and FormData.
            </p>
          </div>

          <div className="p-5 bg-green-50 rounded-lg border border-green-200">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-green-600 text-xl">✓</span>
              <h3 className="text-green-900 font-semibold">
                Progressive Enhancement
              </h3>
            </div>
            <p className="text-sm text-gray-700 leading-relaxed">
              Forms that work without JavaScript and enhance with it.
            </p>
          </div>

          <div className="p-5 bg-green-50 rounded-lg border border-green-200">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-green-600 text-xl">✓</span>
              <h3 className="text-green-900 font-semibold">Error Handling</h3>
            </div>
            <p className="text-sm text-gray-700 leading-relaxed">
              Proper error states and user feedback for failed actions.
            </p>
          </div>

          <div className="p-5 bg-green-50 rounded-lg border border-green-200">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-green-600 text-xl">✓</span>
              <h3 className="text-green-900 font-semibold">Redirects</h3>
            </div>
            <p className="text-sm text-gray-700 leading-relaxed">
              Server actions can redirect after successful completion.
            </p>
          </div>
        </div>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <h2 className="text-2xl font-bold text-gray-900 mb-6">
          Technical Implementation
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="p-5 bg-gray-50 rounded-lg border border-gray-200">
            <div className="flex items-center gap-2 mb-3">
              <div className="w-8 h-8 bg-indigo-100 rounded flex items-center justify-center">
                <span className="text-indigo-600 font-bold">1</span>
              </div>
              <h4 className="text-gray-900 font-semibold">Server Functions</h4>
            </div>
            <p className="text-sm text-gray-600 leading-relaxed">
              Functions marked with
              {' '}
              <code className="bg-gray-200 px-1.5 py-0.5 rounded font-mono text-xs">
                'use server'
              </code>
              {' '}
              are automatically transformed into callable references that can be
              invoked from client components.
            </p>
          </div>

          <div className="p-5 bg-gray-50 rounded-lg border border-gray-200">
            <div className="flex items-center gap-2 mb-3">
              <div className="w-8 h-8 bg-indigo-100 rounded flex items-center justify-center">
                <span className="text-indigo-600 font-bold">2</span>
              </div>
              <h4 className="text-gray-900 font-semibold">HTTP Endpoints</h4>
            </div>
            <p className="text-sm text-gray-600 leading-relaxed">
              Server actions are called via
              {' '}
              <code className="bg-gray-200 px-1.5 py-0.5 rounded font-mono text-xs">
                POST /_rari/action
              </code>
              {' '}
              with JSON payloads containing serialized arguments.
            </p>
          </div>

          <div className="p-5 bg-gray-50 rounded-lg border border-gray-200">
            <div className="flex items-center gap-2 mb-3">
              <div className="w-8 h-8 bg-indigo-100 rounded flex items-center justify-center">
                <span className="text-indigo-600 font-bold">3</span>
              </div>
              <h4 className="text-gray-900 font-semibold">Form Enhancement</h4>
            </div>
            <p className="text-sm text-gray-600 leading-relaxed">
              Forms can post to
              {' '}
              <code className="bg-gray-200 px-1.5 py-0.5 rounded font-mono text-xs">
                /_rari/form-action
              </code>
              {' '}
              for progressive enhancement, working without JavaScript.
            </p>
          </div>

          <div className="p-5 bg-gray-50 rounded-lg border border-gray-200">
            <div className="flex items-center gap-2 mb-3">
              <div className="w-8 h-8 bg-indigo-100 rounded flex items-center justify-center">
                <span className="text-indigo-600 font-bold">4</span>
              </div>
              <h4 className="text-gray-900 font-semibold">Wire Format</h4>
            </div>
            <p className="text-sm text-gray-600 leading-relaxed">
              Actions return JSON responses that can include redirects, error
              states, and updated data for optimistic UI updates.
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Server Actions Demo | Rari App Router',
  description: 'Demonstration of React Server Actions with Rari framework',
}
