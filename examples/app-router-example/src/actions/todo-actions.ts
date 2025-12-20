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
  await new Promise(resolve => setTimeout(resolve, 100))
  return todos
}

export async function addTodo(_prevState: any, formData: FormData) {
  try {
    if (!formData || typeof formData.get !== 'function') {
      console.error('[addTodo] Invalid formData:', formData)
      return {
        success: false,
        error: 'Invalid form data provided.',
        todos,
      }
    }

    const text = formData.get('text') as string

    if (!text || text.trim().length === 0) {
      console.error('[addTodo] Validation error: Todo text is required')
      return {
        success: false,
        error: 'Todo text is required. Please enter some text before submitting.',
        todos,
      }
    }

    const newTodo: Todo = {
      id: Date.now().toString(),
      text: text.trim(),
      completed: false,
      createdAt: new Date().toISOString(),
    }

    todos.push(newTodo)

    return {
      success: true,
      todos,
    }
  }
  catch (error) {
    console.error('[addTodo] Unexpected error:', error)
    const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
    return {
      success: false,
      error: `Failed to add todo: ${errorMessage}. Please try again.`,
      todos,
    }
  }
}

export async function toggleTodo(_prevState: any, formData: FormData) {
  try {
    if (!formData || typeof formData.get !== 'function') {
      console.error('[toggleTodo] Invalid formData:', formData)
      return {
        success: false,
        error: 'Invalid form data provided.',
        todos,
      }
    }

    const id = formData.get('id') as string

    if (!id) {
      console.error('[toggleTodo] Validation error: Todo ID is required')
      return {
        success: false,
        error: 'Todo ID is required to toggle completion status.',
        todos,
      }
    }

    const todo = todos.find(t => t.id === id)
    if (!todo) {
      console.error('[toggleTodo] Todo not found:', id)
      return {
        success: false,
        error: `Todo with ID ${id} not found. It may have been deleted.`,
        todos,
      }
    }

    todo.completed = !todo.completed

    return {
      success: true,
      todos,
    }
  }
  catch (error) {
    console.error('[toggleTodo] Unexpected error:', error)
    const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
    return {
      success: false,
      error: `Failed to toggle todo: ${errorMessage}. Please try again.`,
      todos,
    }
  }
}

export async function deleteTodo(_prevState: any, formData: FormData) {
  try {
    if (!formData || typeof formData.get !== 'function') {
      console.error('[deleteTodo] Invalid formData:', formData)
      return {
        success: false,
        error: 'Invalid form data provided.',
        todos,
      }
    }

    const id = formData.get('id') as string

    if (!id) {
      console.error('[deleteTodo] Validation error: Todo ID is required')
      return {
        success: false,
        error: 'Todo ID is required to delete a todo.',
        todos,
      }
    }

    const initialLength = todos.length
    todos = todos.filter(t => t.id !== id)

    if (todos.length === initialLength) {
      return {
        success: false,
        error: `Todo with ID ${id} not found. It may have already been deleted.`,
        todos,
      }
    }

    return {
      success: true,
      todos,
    }
  }
  catch (error) {
    console.error('[deleteTodo] Unexpected error:', error)
    const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
    return {
      success: false,
      error: `Failed to delete todo: ${errorMessage}. Please try again.`,
      todos,
    }
  }
}

export async function clearCompleted(_prevState: any, _formData: FormData) {
  try {
    const completedCount = todos.filter(t => t.completed).length

    if (completedCount === 0) {
      return {
        success: true,
        todos,
        message: 'No completed todos to clear.',
      }
    }

    todos = todos.filter(t => !t.completed)

    return {
      success: true,
      todos,
      message: `Cleared ${completedCount} completed todo${completedCount > 1 ? 's' : ''}.`,
    }
  }
  catch (error) {
    console.error('[clearCompleted] Unexpected error:', error)
    const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
    return {
      success: false,
      error: `Failed to clear completed todos: ${errorMessage}. Please try again.`,
      todos,
    }
  }
}

export async function createTodoAndRedirect(prevStateOrFormData: any, formData?: FormData) {
  try {
    let data: any

    if (formData instanceof FormData) {
      data = formData
    }
    else if (prevStateOrFormData instanceof FormData) {
      data = prevStateOrFormData
    }
    else if (typeof prevStateOrFormData === 'object' && prevStateOrFormData !== null) {
      data = prevStateOrFormData
    }
    else {
      console.error('[createTodoAndRedirect] Invalid arguments - no data provided')
      return {
        success: false,
        error: 'Invalid form data',
      }
    }

    let text: string | null | undefined

    if (data instanceof FormData) {
      text = data.get('text') as string
    }
    else if (data.get && typeof data.get === 'function') {
      text = data.get('text') as string
    }
    else if (data.data && typeof data.data === 'object') {
      text = data.data.text
    }
    else {
      text = data.text
    }

    if (!text || (typeof text === 'string' && text.trim().length === 0)) {
      console.error('[createTodoAndRedirect] Validation error: Todo text is required')
      return {
        success: false,
        error: 'Todo text is required. Please enter some text before submitting.',
      }
    }

    const newTodo: Todo = {
      id: Date.now().toString(),
      text: typeof text === 'string' ? text.trim() : String(text).trim(),
      completed: false,
      createdAt: new Date().toISOString(),
    }

    todos.push(newTodo)

    return {
      success: true,
      redirect: '/actions',
      todos,
    }
  }
  catch (error) {
    console.error('[createTodoAndRedirect] Unexpected error:', error)
    const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
    return {
      success: false,
      error: `Failed to create todo: ${errorMessage}. Please try again.`,
    }
  }
}
