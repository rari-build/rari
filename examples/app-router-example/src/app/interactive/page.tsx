import Counter from '@/components/Counter'
import TodoList from '@/components/TodoList'

export default async function InteractivePage() {
  return (
    <div className="grid gap-8">
      <div className="bg-white rounded-xl p-12 shadow-2xl">
        <h1 className="text-4xl font-bold mb-4 text-gray-900">
          Interactive Components Test
        </h1>
        <p className="text-lg text-gray-600 mb-8">
          This page demonstrates mixing server and client components using RSC.
          The components below are client-side React components that maintain
          state.
        </p>
      </div>

      <Counter />

      <TodoList />

      <div className="bg-white rounded-xl p-8 shadow-2xl">
        <h3 className="text-gray-900 mb-4 text-xl font-semibold">
          Server Component Info
        </h3>
        <p className="text-gray-600 leading-relaxed">
          This text is rendered on the server at
          {' '}
          <strong>{new Date().toISOString()}</strong>
          . The Counter and TodoList
          above are client components that hydrate on the client.
        </p>
        <p className="text-gray-600 leading-relaxed mt-4">
          Try refreshing the page - the timestamp will update (server-rendered),
          but the counter and todo state will reset (client-rendered).
        </p>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Interactive Components | rari App Router',
  description: 'Testing client components with RSC',
}
