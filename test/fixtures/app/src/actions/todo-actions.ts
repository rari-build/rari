'use server'

import { cookies } from 'rari/headers'

export interface Todo {
  id: string
  text: string
  completed: boolean
  createdAt: string
}

const DEFAULT_TODOS: Todo[] = [
  { id: '1', text: 'Learn React Server Components', completed: true, createdAt: new Date().toISOString() },
  { id: '2', text: 'Test Server Actions', completed: false, createdAt: new Date().toISOString() },
]

const sessionStore = new Map<string, Todo[]>()

async function getSessionId(): Promise<string> {
  const store = await cookies()
  let sessionId = store.get('todo-session')?.value
  if (!sessionId) {
    sessionId = crypto.randomUUID()
    store.set('todo-session', sessionId, { path: '/', httpOnly: true })
  }

  return sessionId
}

async function getSessionTodos(sessionId?: string): Promise<{ id: string, todos: Todo[] }> {
  const id = sessionId ?? await getSessionId()
  if (!sessionStore.has(id)) {
    sessionStore.set(id, DEFAULT_TODOS.map(t => ({ ...t })))
  }

  return { id, todos: sessionStore.get(id)! }
}

async function setSessionTodos(sessionId: string, todos: Todo[]): Promise<void> {
  sessionStore.set(sessionId, todos)
}

export async function getTodos() {
  const { todos } = await getSessionTodos()
  return [...todos]
}

export async function addTodo(formData: FormData) {
  const text = formData.get('text') as string
  const { id, todos } = await getSessionTodos()

  if (!text?.trim()) {
    return { success: false, error: 'Todo text is required', todos }
  }

  const newTodo: Todo = {
    id: crypto.randomUUID(),
    text: text.trim(),
    completed: false,
    createdAt: new Date().toISOString(),
  }

  const updated = [...todos, newTodo]
  await setSessionTodos(id, updated)
  return { success: true, todos: updated }
}

export async function toggleTodo(formData: FormData) {
  const todoId = formData.get('id') as string
  const { id, todos } = await getSessionTodos()
  const todo = todos.find(t => t.id === todoId)

  if (!todo) {
    return { success: false, error: 'Todo not found', todos }
  }

  todo.completed = !todo.completed
  await setSessionTodos(id, todos)
  return { success: true, todos: [...todos] }
}

export async function deleteTodo(formData: FormData) {
  const todoId = formData.get('id') as string
  const { id, todos } = await getSessionTodos()
  const updated = todos.filter(t => t.id !== todoId)

  if (updated.length === todos.length) {
    return { success: false, error: 'Todo not found', todos }
  }

  await setSessionTodos(id, updated)
  return { success: true, todos: updated }
}

export async function clearCompleted() {
  const { id, todos } = await getSessionTodos()
  const updated = todos.filter(t => !t.completed)
  await setSessionTodos(id, updated)
  return { success: true, todos: updated }
}

export async function resetTodos() {
  const id = await getSessionId()
  const reset = DEFAULT_TODOS.map(t => ({ ...t }))
  await setSessionTodos(id, reset)
  return { success: true, todos: reset }
}
