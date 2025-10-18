import { getTodos } from '../../actions/todo-actions'
import ProgressiveFormExample from '../../components/ProgressiveFormExample'
import TodoAppWithActions from '../../components/TodoAppWithActions'

export default async function ActionsPage() {
  const initialTodos = await getTodos()

  return (
    <div className="bg-white rounded-xl p-12 shadow-2xl">
      <header className="text-center mb-12 pb-8 border-b-2 border-gray-200">
        <h1 className="text-5xl font-bold mb-4 bg-gradient-to-r from-blue-600 to-cyan-600 bg-clip-text text-transparent">
          React Server Actions Demo
        </h1>
        <p className="text-xl text-gray-600 max-w-2xl mx-auto leading-relaxed">
          This page demonstrates React Server Actions working with Rari.
          All patterns follow React's official server function specifications.
        </p>
      </header>

      <main className="flex flex-col gap-8">
        <section>
          <h2 className="text-3xl text-blue-600 mb-6 font-bold">
            Interactive Todo App
          </h2>

          <TodoAppWithActions initialTodos={initialTodos} />
        </section>

        <section>
          <h2 className="text-3xl text-blue-600 mb-6 font-bold">
            Progressive Enhancement
          </h2>

          <ProgressiveFormExample />
        </section>

        <section className="bg-white p-8 rounded-2xl shadow-2xl">
          <h2 className="text-3xl text-blue-600 mb-6 font-bold">
            Server Action Patterns Demonstrated
          </h2>

          <div className="grid grid-cols-[repeat(auto-fit,minmax(300px,1fr))] gap-6">
            <div className="p-6 bg-gray-50 rounded-lg border border-gray-200">
              <h3 className="text-green-600 mb-2 text-xl font-semibold">
                ✅ useActionState Hook
              </h3>
              <p className="text-gray-600 m-0 leading-relaxed">
                Manage server action state with pending states and error handling.
              </p>
            </div>

            <div className="p-6 bg-gray-50 rounded-lg border border-gray-200">
              <h3 className="text-green-600 mb-2 text-xl font-semibold">
                ✅ useTransition Hook
              </h3>
              <p className="text-gray-600 m-0 leading-relaxed">
                Track pending states across multiple actions for better UX.
              </p>
            </div>

            <div className="p-6 bg-gray-50 rounded-lg border border-gray-200">
              <h3 className="text-green-600 mb-2 text-xl font-semibold">
                ✅ Form Actions
              </h3>
              <p className="text-gray-600 m-0 leading-relaxed">
                Server functions that work with HTML forms and FormData.
              </p>
            </div>

            <div className="p-6 bg-gray-50 rounded-lg border border-gray-200">
              <h3 className="text-green-600 mb-2 text-xl font-semibold">
                ✅ Progressive Enhancement
              </h3>
              <p className="text-gray-600 m-0 leading-relaxed">
                Forms that work without JavaScript and enhance with it.
              </p>
            </div>

            <div className="p-6 bg-gray-50 rounded-lg border border-gray-200">
              <h3 className="text-green-600 mb-2 text-xl font-semibold">
                ✅ Error Handling
              </h3>
              <p className="text-gray-600 m-0 leading-relaxed">
                Proper error states and user feedback for failed actions.
              </p>
            </div>

            <div className="p-6 bg-gray-50 rounded-lg border border-gray-200">
              <h3 className="text-green-600 mb-2 text-xl font-semibold">
                ✅ Redirects
              </h3>
              <p className="text-gray-600 m-0 leading-relaxed">
                Server actions can redirect after successful completion.
              </p>
            </div>
          </div>
        </section>

        <section className="bg-white p-8 rounded-2xl shadow-2xl">
          <h2 className="text-3xl text-blue-600 mb-6 font-bold">
            Technical Implementation
          </h2>

          <div className="grid grid-cols-[repeat(auto-fit,minmax(250px,1fr))] gap-6">
            <div className="p-6 bg-amber-50 rounded-lg border border-amber-200">
              <h4 className="text-amber-700 mb-2 text-lg font-semibold">
                Server Functions
              </h4>
              <p className="text-gray-600 m-0 leading-relaxed">
                Functions marked with{' '}
                <code className="bg-gray-200 px-1 py-0.5 rounded font-mono text-sm">
                  'use server'
                </code>{' '}
                are automatically transformed into callable references.
              </p>
            </div>

            <div className="p-6 bg-amber-50 rounded-lg border border-amber-200">
              <h4 className="text-amber-700 mb-2 text-lg font-semibold">
                HTTP Endpoints
              </h4>
              <p className="text-gray-600 m-0 leading-relaxed">
                Server actions are called via{' '}
                <code className="bg-gray-200 px-1 py-0.5 rounded font-mono text-sm">
                  POST /api/rsc/action
                </code>{' '}
                with JSON payloads.
              </p>
            </div>

            <div className="p-6 bg-amber-50 rounded-lg border border-amber-200">
              <h4 className="text-amber-700 mb-2 text-lg font-semibold">
                Form Enhancement
              </h4>
              <p className="text-gray-600 m-0 leading-relaxed">
                Forms can post to{' '}
                <code className="bg-gray-200 px-1 py-0.5 rounded font-mono text-sm">
                  /api/rsc/form-action
                </code>{' '}
                for progressive enhancement.
              </p>
            </div>

            <div className="p-6 bg-amber-50 rounded-lg border border-amber-200">
              <h4 className="text-amber-700 mb-2 text-lg font-semibold">
                Wire Format
              </h4>
              <p className="text-gray-600 m-0 leading-relaxed">
                Actions return JSON responses that can include redirects and error states.
              </p>
            </div>
          </div>
        </section>
      </main>
    </div>
  )
}

export const metadata = {
  title: 'Server Actions Demo | Rari App Router',
  description: 'Demonstration of React Server Actions with Rari framework',
}
