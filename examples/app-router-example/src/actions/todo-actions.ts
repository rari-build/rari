'use server'

export interface Todo {
  id: string
  text: string
  completed: boolean
  createdAt: string
}

let todos: Todo[] = [
  { id: '1', text: 'Learn React Server Components', completed: true, createdAt: new Date().toISOString() },
  { id: '2', text: 'Implement Server Actions', completed: false, createdAt: new Date().toISOString() },
  { id: '3', text: 'Build with Rari', completed: false, createdAt: new Date().toISOString() },
]

export async function getTodos() {
  return todos
}

export async function addTodo(formData: FormData) {
  const text = formData.get('text') as string

  if (!text?.trim()) {
    return { success: false, error: 'Todo text is required', todos }
  }

  todos.push({
    id: Date.now().toString(),
    text: text.trim(),
    completed: false,
    createdAt: new Date().toISOString(),
  })

  return { success: true, todos }
}

export async function toggleTodo(formData: FormData) {
  const id = formData.get('id') as string
  const todo = todos.find(t => t.id === id)

  if (!todo) {
    return { success: false, error: 'Todo not found', todos }
  }

  todo.completed = !todo.completed
  return { success: true, todos }
}

export async function deleteTodo(formData: FormData) {
  const id = formData.get('id') as string
  const initialLength = todos.length
  todos = todos.filter(t => t.id !== id)

  if (todos.length === initialLength) {
    return { success: false, error: 'Todo not found', todos }
  }

  return { success: true, todos }
}

export async function clearCompleted() {
  todos = todos.filter(t => !t.completed)
  return { success: true, todos }
}

export async function createTodoAndRedirect(formData: FormData) {
  const text = formData.get('text') as string

  if (!text?.trim()) {
    return { success: false, error: 'Todo text is required' }
  }

  todos.push({
    id: Date.now().toString(),
    text: text.trim(),
    completed: false,
    createdAt: new Date().toISOString(),
  })

  return { success: true, redirect: '/actions', todos }
}
