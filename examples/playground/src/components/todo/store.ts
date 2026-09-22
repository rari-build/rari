import type { Todo } from './types'

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

export function getTodos(): Todo[] {
  return [...todos]
}

export function snapshotTodos(): Todo[] {
  return [...todos]
}

export function insertTodo(text: string): Todo {
  const newTodo: Todo = {
    id: crypto.randomUUID(),
    text: text.trim(),
    completed: false,
    createdAt: new Date().toISOString(),
  }
  todos = [...todos, newTodo]
  return newTodo
}

export function toggleTodoById(id: string): boolean {
  if (!todos.some(t => t.id === id)) return false
  todos = todos.map(t => (t.id === id ? { ...t, completed: !t.completed } : t))
  return true
}

export function deleteTodoById(id: string): boolean {
  const next = todos.filter(t => t.id !== id)
  if (next.length === todos.length) return false
  todos = next
  return true
}

export function clearCompletedTodos(): void {
  todos = todos.filter(t => !t.completed)
}
