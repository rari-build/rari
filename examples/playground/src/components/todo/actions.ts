'use server'

// oxlint-disable typescript/require-await
import type { TodoActionState } from './types'
import {
  clearCompletedTodos,
  deleteTodoById,
  insertTodo,
  snapshotTodos,
  toggleTodoById,
} from './store'

export async function addTodo(
  _prevState: TodoActionState,
  formData: FormData,
): Promise<TodoActionState> {
  const text = formData.get('text')
  if (typeof text !== 'string' || text.trim() === '') {
    return { success: false, error: 'Todo text is required', todos: snapshotTodos() }
  }

  insertTodo(text)
  return { success: true, todos: snapshotTodos() }
}

export async function toggleTodo(id: string): Promise<TodoActionState> {
  if (!toggleTodoById(id)) {
    return { success: false, error: 'Todo not found', todos: snapshotTodos() }
  }
  return { success: true, todos: snapshotTodos() }
}

export async function deleteTodo(id: string): Promise<TodoActionState> {
  if (!deleteTodoById(id)) {
    return { success: false, error: 'Todo not found', todos: snapshotTodos() }
  }
  return { success: true, todos: snapshotTodos() }
}

export async function clearCompleted(): Promise<TodoActionState> {
  clearCompletedTodos()
  return { success: true, todos: snapshotTodos() }
}
