'use client'

import type { Todo } from '../actions/todo-actions'
import { useEffect, useState, useTransition } from 'react'
import { clearCompleted, deleteTodo, toggleTodo } from '../actions/todo-actions'

interface TodoListProps {
  initialTodos: Todo[]
  onUpdate?: () => void
}

export default function TodoListWithActions({ initialTodos, onUpdate }: TodoListProps) {
  const [todos, setTodos] = useState<Todo[]>(initialTodos)
  const [isPending, startTransition] = useTransition()
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setTodos(initialTodos)
  }, [initialTodos])

  const completedCount = todos.filter(todo => todo.completed).length
  const activeCount = todos.length - completedCount

  const handleToggle = async (id: string) => {
    setError(null)
    startTransition(async () => {
      try {
        const formData = new FormData()
        formData.append('id', id)
        const result = await toggleTodo({ todos }, formData)
        if (result.success && result.todos) {
          setTodos(result.todos)
          if (onUpdate)
            onUpdate()
        }
        else if (result.error) {
          setError(result.error)
        }
      }
      catch (error) {
        const errorMsg = error instanceof Error ? error.message : 'Failed to toggle todo'
        console.error('[TodoList] Toggle error:', error)
        setError(errorMsg)
      }
    })
  }

  const handleDelete = async (id: string) => {
    setError(null)
    startTransition(async () => {
      try {
        const formData = new FormData()
        formData.append('id', id)
        const result = await deleteTodo({ todos }, formData)
        if (result.success && result.todos) {
          setTodos(result.todos)
          if (onUpdate)
            onUpdate()
        }
        else if (result.error) {
          setError(result.error)
        }
      }
      catch (error) {
        const errorMsg = error instanceof Error ? error.message : 'Failed to delete todo'
        console.error('[TodoList] Delete error:', error)
        setError(errorMsg)
      }
    })
  }

  const handleClearCompleted = async () => {
    setError(null)
    startTransition(async () => {
      try {
        const result = await clearCompleted({ todos }, new FormData())
        if (result.success && result.todos) {
          setTodos(result.todos)
          if (onUpdate)
            onUpdate()
        }
        else if (result.error) {
          setError(result.error)
        }
      }
      catch (error) {
        const errorMsg = error instanceof Error ? error.message : 'Failed to clear completed todos'
        console.error('[TodoList] Clear completed error:', error)
        setError(errorMsg)
      }
    })
  }

  return (
    <div className="bg-white p-8 rounded-lg border-2 border-blue-600">
      <h3 className="text-blue-600 mb-4 text-xl font-semibold">
        Todo List (with Server Actions)
      </h3>

      {error && (
        <div className="text-red-600 text-sm p-3 bg-red-100 rounded mb-4 border border-red-300">
          <strong>Error:</strong> {error}
        </div>
      )}

      <div className="flex justify-between items-center mb-4 p-4 bg-gray-50 rounded-lg text-sm text-gray-600">
        <span>
          <strong>{activeCount}</strong> active
        </span>
        <span>
          <strong>{completedCount}</strong> completed
        </span>
        {completedCount > 0 && (
          <button
            onClick={handleClearCompleted}
            disabled={isPending}
            className={`px-4 py-2 text-white border-none rounded text-xs ${
              isPending ? 'bg-red-300 cursor-not-allowed opacity-60' : 'bg-red-600 cursor-pointer hover:bg-red-700'
            }`}
          >
            Clear Completed
          </button>
        )}
      </div>

      <ul className="list-none p-0 m-0">
        {todos.map(todo => (
          <li
            key={todo.id}
            className={`flex items-center gap-4 p-4 border border-gray-200 rounded-lg mb-2 transition-all ${
              todo.completed ? 'bg-gray-50' : 'bg-white'
            } ${isPending ? 'opacity-70' : ''}`}
          >
            <button
              onClick={() => handleToggle(todo.id)}
              disabled={isPending}
              className={`w-8 h-8 border-2 rounded-full flex items-center justify-center font-bold text-base ${
                todo.completed
                  ? 'border-blue-600 bg-blue-600 text-white'
                  : 'border-blue-600 bg-white text-blue-600'
              } ${isPending ? 'cursor-not-allowed' : 'cursor-pointer'}`}
            >
              {todo.completed ? '✓' : '○'}
            </button>

            <span
              className={`flex-1 text-base ${
                todo.completed ? 'line-through text-gray-400' : 'text-gray-900'
              }`}
            >
              {todo.text}
            </span>

            <button
              onClick={() => handleDelete(todo.id)}
              disabled={isPending}
              className={`w-8 h-8 border border-red-600 rounded bg-white text-red-600 flex items-center justify-center text-xl font-bold ${
                isPending ? 'cursor-not-allowed' : 'cursor-pointer hover:bg-red-50'
              }`}
            >
              ×
            </button>
          </li>
        ))}
      </ul>

      {todos.length === 0 && (
        <div className="text-center p-12 text-gray-600 italic">
          No todos yet. Add one above!
        </div>
      )}

      <div className="text-sm text-gray-600 p-4 bg-gray-50 rounded mt-4">
        <strong>Features demonstrated:</strong>
        <ul className="mt-2 ml-6 leading-relaxed">
          <li>Multiple server actions in one component</li>
          <li>Pending states during execution</li>
          <li>Optimistic UI updates</li>
          <li>State management with server actions</li>
        </ul>
      </div>
    </div>
  )
}
