'use server'

/* Server actions must be `async` (react/rsc-function-definition) even when they do sync in-memory work. */
/* oxlint-disable typescript/require-await */

export interface Todo {
  readonly id: string
  readonly text: string
  readonly completed: boolean
  readonly createdAt: string
}

export interface TodoActionState {
  readonly success: boolean
  readonly error?: string
  readonly todos?: readonly Todo[]
}

let todos: Todo[] = [
  {
    id: '1',
    text: 'Learn React Server Components',
    completed: true,
    createdAt: new Date().toISOString(),
  },
  {
    id: '2',
    text: 'Implement Server Actions',
    completed: false,
    createdAt: new Date().toISOString(),
  },
  { id: '3', text: 'Build with rari', completed: false, createdAt: new Date().toISOString() },
]

export async function getTodos() {
  return [...todos]
}

export async function addTodo(
  _prevState: TodoActionState,
  formData: FormData,
): Promise<TodoActionState> {
  const text = formData.get('text')
  if (typeof text !== 'string' || text.trim() === '')
    return { success: false, error: 'Todo text is required', todos }

  const newTodo: Todo = {
    id: crypto.randomUUID(),
    text: text.trim(),
    completed: false,
    createdAt: new Date().toISOString(),
  }

  todos = [...todos, newTodo]
  return { success: true, todos: [...todos] }
}

export async function toggleTodo(id: string): Promise<TodoActionState> {
  if (!todos.some(t => t.id === id)) return { success: false, error: 'Todo not found', todos }

  todos = todos.map(t => (t.id === id ? { ...t, completed: !t.completed } : t))
  return { success: true, todos: [...todos] }
}

export async function deleteTodo(id: string): Promise<TodoActionState> {
  const next = todos.filter(t => t.id !== id)
  if (next.length === todos.length) return { success: false, error: 'Todo not found', todos }

  todos = next
  return { success: true, todos: [...todos] }
}

export async function clearCompleted(): Promise<TodoActionState> {
  todos = todos.filter(t => !t.completed)
  return { success: true, todos: [...todos] }
}
