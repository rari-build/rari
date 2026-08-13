import type { Metadata } from 'rari'
import { getTodos } from '@/actions/todo-actions'
import TodoAppWithActions from '@/components/TodoAppWithActions'

export default async function ActionsPage() {
  const initialTodos = await getTodos()

  return (
    <div className="space-y-8">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 md:p-12">
        <div className="flex items-center gap-3 mb-4">
          <h1 className="text-4xl font-bold text-gray-900">React Server Actions Demo</h1>
          <span className="text-3xl">⚡</span>
        </div>
        <p className="text-lg text-gray-600 max-w-3xl leading-relaxed">
          This page demonstrates React Server Actions working with rari. Mutations live in a{' '}
          <code className="bg-gray-100 px-1.5 py-0.5 rounded font-mono text-sm">'use server'</code>{' '}
          module and are called from client components through Flight.
        </p>
      </div>

      <div>
        <h2 className="text-2xl font-bold text-gray-900 mb-4">Interactive Todo Application</h2>
        <TodoAppWithActions initialTodos={initialTodos} />
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <h2 className="text-2xl font-bold text-gray-900 mb-6">
          Server Action Patterns Demonstrated
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div className="p-5 bg-green-50 rounded-lg border border-green-200">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-green-600 text-xl">✓</span>
              <h3 className="text-green-900 font-semibold">useActionState Hook</h3>
            </div>
            <p className="text-sm text-gray-700 leading-relaxed">
              Bind a server action directly to a form with pending state and inline errors.
            </p>
          </div>

          <div className="p-5 bg-green-50 rounded-lg border border-green-200">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-green-600 text-xl">✓</span>
              <h3 className="text-green-900 font-semibold">useTransition Hook</h3>
            </div>
            <p className="text-sm text-gray-700 leading-relaxed">
              Call server actions from buttons while tracking a shared pending state.
            </p>
          </div>

          <div className="p-5 bg-green-50 rounded-lg border border-green-200">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-green-600 text-xl">✓</span>
              <h3 className="text-green-900 font-semibold">Form Actions</h3>
            </div>
            <p className="text-sm text-gray-700 leading-relaxed">
              Server functions that accept previous state plus FormData from HTML forms.
            </p>
          </div>

          <div className="p-5 bg-green-50 rounded-lg border border-green-200">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-green-600 text-xl">✓</span>
              <h3 className="text-green-900 font-semibold">Direct Calls</h3>
            </div>
            <p className="text-sm text-gray-700 leading-relaxed">
              Invoke typed server functions with normal arguments outside of forms.
            </p>
          </div>

          <div className="p-5 bg-green-50 rounded-lg border border-green-200">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-green-600 text-xl">✓</span>
              <h3 className="text-green-900 font-semibold">Error Handling</h3>
            </div>
            <p className="text-sm text-gray-700 leading-relaxed">
              Return structured error states and surface them to the user.
            </p>
          </div>

          <div className="p-5 bg-green-50 rounded-lg border border-green-200">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-green-600 text-xl">✓</span>
              <h3 className="text-green-900 font-semibold">Result Sync</h3>
            </div>
            <p className="text-sm text-gray-700 leading-relaxed">
              Update client UI from the data returned by each successful action.
            </p>
          </div>
        </div>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <h2 className="text-2xl font-bold text-gray-900 mb-6">Technical Implementation</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="p-5 bg-gray-50 rounded-lg border border-gray-200">
            <div className="flex items-center gap-2 mb-3">
              <div className="w-8 h-8 bg-indigo-100 rounded flex items-center justify-center">
                <span className="text-indigo-600 font-bold">1</span>
              </div>
              <h4 className="text-gray-900 font-semibold">Server Functions</h4>
            </div>
            <p className="text-sm text-gray-600 leading-relaxed">
              Functions marked with{' '}
              <code className="bg-gray-200 px-1.5 py-0.5 rounded font-mono text-xs">
                'use server'
              </code>{' '}
              are transformed into callable references that client components can invoke.
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
              Server actions are called via{' '}
              <code className="bg-gray-200 px-1.5 py-0.5 rounded font-mono text-xs">
                POST /_rari/action
              </code>{' '}
              with serialized arguments.
            </p>
          </div>

          <div className="p-5 bg-gray-50 rounded-lg border border-gray-200">
            <div className="flex items-center gap-2 mb-3">
              <div className="w-8 h-8 bg-indigo-100 rounded flex items-center justify-center">
                <span className="text-indigo-600 font-bold">3</span>
              </div>
              <h4 className="text-gray-900 font-semibold">State Management</h4>
            </div>
            <p className="text-sm text-gray-600 leading-relaxed">
              Server actions integrate with React hooks like{' '}
              <code className="bg-gray-200 px-1.5 py-0.5 rounded font-mono text-xs">
                useActionState
              </code>{' '}
              and{' '}
              <code className="bg-gray-200 px-1.5 py-0.5 rounded font-mono text-xs">
                useTransition
              </code>
              .
            </p>
          </div>

          <div className="p-5 bg-gray-50 rounded-lg border border-gray-200">
            <div className="flex items-center gap-2 mb-3">
              <div className="w-8 h-8 bg-indigo-100 rounded flex items-center justify-center">
                <span className="text-indigo-600 font-bold">4</span>
              </div>
              <h4 className="text-gray-900 font-semibold">Flight Protocol</h4>
            </div>
            <p className="text-sm text-gray-600 leading-relaxed">
              Arguments and return values travel over React Flight so client components can call
              server functions as if they were local async functions.
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'Server Actions Demo | rari App Router',
  description: 'Demonstration of React Server Actions with rari framework',
}
