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

async function getSessionTodos(): Promise<Todo[]> {
  const id = await getSessionId()
  if (!sessionStore.has(id)) {
    sessionStore.set(id, DEFAULT_TODOS.map(t => ({ ...t })))
  }

  return sessionStore.get(id)!
}

async function setSessionTodos(todos: Todo[]): Promise<void> {
  sessionStore.set(await getSessionId(), todos)
}

export async function getTodos() {
  return [...await getSessionTodos()]
}

export async function addTodo(formData: FormData) {
  const text = formData.get('text') as string
  const todos = await getSessionTodos()

  if (!text?.trim()) {
    return { success: false, error: 'Todo text is required', todos }
  }

  const newTodo: Todo = {
    id: Date.now().toString(),
    text: text.trim(),
    completed: false,
    createdAt: new Date().toISOString(),
  }

  const updated = [...todos, newTodo]
  await setSessionTodos(updated)
  return { success: true, todos: updated }
}

export async function toggleTodo(formData: FormData) {
  const id = formData.get('id') as string
  const todos = await getSessionTodos()
  const todo = todos.find(t => t.id === id)

  if (!todo) {
    return { success: false, error: 'Todo not found', todos }
  }

  todo.completed = !todo.completed
  await setSessionTodos(todos)
  return { success: true, todos: [...todos] }
}

export async function deleteTodo(formData: FormData) {
  const id = formData.get('id') as string
  const todos = await getSessionTodos()
  const updated = todos.filter(t => t.id !== id)

  if (updated.length === todos.length) {
    return { success: false, error: 'Todo not found', todos }
  }

  await setSessionTodos(updated)
  return { success: true, todos: updated }
}

export async function clearCompleted() {
  const todos = await getSessionTodos()
  const updated = todos.filter(t => !t.completed)
  await setSessionTodos(updated)
  return { success: true, todos: updated }
}

export async function resetTodos() {
  const reset = DEFAULT_TODOS.map(t => ({ ...t }))
  await setSessionTodos(reset)
  return { success: true, todos: reset }
}
