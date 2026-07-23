'use server'

/* Server actions must be `async` (react/rsc-function-definition) even when they do sync in-memory work. */
/* oxlint-disable typescript/require-await */

export interface Todo {
  readonly id: string
  readonly text: string
  readonly completed: boolean
  readonly createdAt: string
}

type TodoActionResult =
  | { success: boolean; error: string; todos: Todo[] }
  | { error?: undefined; success: boolean; todos: Todo[] }

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

export async function addTodo(formData: FormData): Promise<TodoActionResult> {
  const text = formData.get('text')
  if (typeof text !== 'string' || text.trim() === '')
    return { success: false, error: 'Todo text is required', todos }

  const newTodo = {
    id: crypto.randomUUID(),
    text: text.trim(),
    completed: false,
    createdAt: new Date().toISOString(),
  }

  todos.push(newTodo)

  return { success: true, todos: [...todos] }
}

export async function toggleTodo(formData: FormData): Promise<TodoActionResult> {
  const id = formData.get('id')
  if (typeof id !== 'string') return { success: false, error: 'Todo not found', todos }

  const exists = todos.some(t => t.id === id)

  if (!exists) return { success: false, error: 'Todo not found', todos }
  todos = todos.map(t => (t.id === id ? { ...t, completed: !t.completed } : t))
  return { success: true, todos }
}

export async function deleteTodo(formData: FormData): Promise<TodoActionResult> {
  const id = formData.get('id')
  if (typeof id !== 'string') return { success: false, error: 'Todo not found', todos }

  const initialLength = todos.length
  todos = todos.filter(t => t.id !== id)

  if (todos.length === initialLength) return { success: false, error: 'Todo not found', todos }

  return { success: true, todos }
}

export async function clearCompleted(): Promise<TodoActionResult> {
  todos = todos.filter(t => !t.completed)
  return { success: true, todos }
}
