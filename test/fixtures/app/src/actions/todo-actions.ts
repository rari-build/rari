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

const MAX_SESSIONS = 1000

class LRUMap<K, V> {
  private map: Map<K, V>
  private maxSize: number

  constructor(maxSize: number) {
    this.map = new Map()
    this.maxSize = maxSize
  }

  get(key: K): V | undefined {
    const value = this.map.get(key)
    if (value !== undefined) {
      this.map.delete(key)
      this.map.set(key, value)
    }

    return value
  }

  set(key: K, value: V): void {
    if (this.map.has(key))
      this.map.delete(key)

    this.map.set(key, value)

    if (this.map.size > this.maxSize) {
      const firstKey = this.map.keys().next().value
      if (firstKey !== undefined)
        this.map.delete(firstKey)
    }
  }

  has(key: K): boolean {
    return this.map.has(key)
  }

  delete(key: K): boolean {
    return this.map.delete(key)
  }

  get size(): number {
    return this.map.size
  }
}

const sessionStore = new LRUMap<string, Todo[]>(MAX_SESSIONS)

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

function updateSessionTodos(sessionId: string, updater: (todos: Todo[]) => Todo[]): Todo[] {
  const current = sessionStore.get(sessionId) ?? DEFAULT_TODOS.map(t => ({ ...t }))
  const next = updater(current)
  sessionStore.set(sessionId, next)
  return next
}

export async function getTodos() {
  const { todos } = await getSessionTodos()
  return [...todos]
}

export async function addTodo(formData: FormData) {
  const text = formData.get('text') as string

  if (!text?.trim()) {
    const { todos } = await getSessionTodos()
    return { success: false, error: 'Todo text is required', todos }
  }

  const newTodo: Todo = {
    id: crypto.randomUUID(),
    text: text.trim(),
    completed: false,
    createdAt: new Date().toISOString(),
  }

  const id = await getSessionId()
  const todos = updateSessionTodos(id, current => [...current, newTodo])
  return { success: true, todos }
}

export async function toggleTodo(formData: FormData) {
  const todoId = formData.get('id') as string
  const id = await getSessionId()

  let found = false
  const todos = updateSessionTodos(id, (current) => {
    const todo = current.find(t => t.id === todoId)
    if (!todo)
      return current
    found = true
    return current.map(t => t.id === todoId ? { ...t, completed: !t.completed } : t)
  })

  if (!found)
    return { success: false, error: 'Todo not found', todos }

  return { success: true, todos }
}

export async function deleteTodo(formData: FormData) {
  const todoId = formData.get('id') as string
  const id = await getSessionId()

  let found = false
  const todos = updateSessionTodos(id, (current) => {
    const next = current.filter(t => t.id !== todoId)
    found = next.length !== current.length
    return next
  })

  if (!found)
    return { success: false, error: 'Todo not found', todos }

  return { success: true, todos }
}

export async function clearCompleted() {
  const id = await getSessionId()
  const todos = updateSessionTodos(id, current => current.filter(t => !t.completed))
  return { success: true, todos }
}

export async function resetTodos() {
  const id = await getSessionId()
  const todos = updateSessionTodos(id, () => DEFAULT_TODOS.map(t => ({ ...t })))
  return { success: true, todos }
}
