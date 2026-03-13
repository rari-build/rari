import { getTodos } from '@/actions/todo-actions'
import TodoApp from '@/components/TodoApp'

export default async function ActionsPage() {
  const initialTodos = await getTodos()

  return (
    <div className="max-w-2xl mx-auto p-6">
      <h1 data-testid="page-title">Server Actions Test Page</h1>
      <p data-testid="page-description">Testing React Server Actions with Rari</p>

      <TodoApp initialTodos={initialTodos} />
    </div>
  )
}
