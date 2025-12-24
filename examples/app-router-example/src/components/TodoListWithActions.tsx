'use client'

import type { Todo } from '@/actions/todo-actions'
import { useEffect, useState, useTransition } from 'react'
import {
  clearCompleted,
  deleteTodo,
  toggleTodo,
} from '@/actions/todo-actions'

interface TodoListProps {
  initialTodos: Todo[]
  onUpdate?: () => void
}

export default function TodoListWithActions({
  initialTodos,
  onUpdate,
}: TodoListProps) {
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
        const result = await toggleTodo(formData)
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
        const errorMsg
          = error instanceof Error ? error.message : 'Failed to toggle todo'
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
        const result = await deleteTodo(formData)
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
        const errorMsg
          = error instanceof Error ? error.message : 'Failed to delete todo'
        console.error('[TodoList] Delete error:', error)
        setError(errorMsg)
      }
    })
  }

  const handleClearCompleted = async () => {
    setError(null)
    startTransition(async () => {
      try {
        const result = await clearCompleted()
        if (result.success && result.todos) {
          setTodos(result.todos)
          if (onUpdate)
            onUpdate()
        }
      }
      catch (error) {
        const errorMsg
          = error instanceof Error
            ? error.message
            : 'Failed to clear completed todos'
        console.error('[TodoList] Clear completed error:', error)
        setError(errorMsg)
      }
    })
  }

  return (
    <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
      <div className="flex items-center gap-2 mb-4">
        <h3 className="text-lg font-semibold text-gray-900">Todo List</h3>
        <span className="text-xs font-medium text-indigo-600 bg-indigo-50 px-2 py-1 rounded">
          Server Actions
        </span>
      </div>

      {error && (
        <div className="flex items-start gap-2 text-sm text-red-700 p-3 bg-red-50 border border-red-200 rounded-lg mb-4">
          <svg
            className="w-5 h-5 shrink-0 mt-0.5"
            fill="currentColor"
            viewBox="0 0 20 20"
          >
            <path
              fillRule="evenodd"
              d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
              clipRule="evenodd"
            />
          </svg>
          <div>
            <strong className="font-semibold">Error:</strong>
            {' '}
            {error}
          </div>
        </div>
      )}

      <div className="flex justify-between items-center mb-4 p-4 bg-gray-50 rounded-lg border border-gray-200">
        <div className="flex gap-6 text-sm">
          <span className="text-gray-600">
            <strong className="font-semibold text-gray-900">
              {activeCount}
            </strong>
            {' '}
            active
          </span>
          <span className="text-gray-600">
            <strong className="font-semibold text-gray-900">
              {completedCount}
            </strong>
            {' '}
            completed
          </span>
        </div>
        {completedCount > 0 && (
          <button
            onClick={handleClearCompleted}
            disabled={isPending}
            className={`px-3 py-1.5 text-xs font-medium rounded-lg transition-all ${isPending
              ? 'bg-gray-200 text-gray-400 cursor-not-allowed'
              : 'bg-red-50 text-red-600 hover:bg-red-100 border border-red-200'
            }`}
          >
            Clear Completed
          </button>
        )}
      </div>

      <ul className="space-y-2">
        {todos.map(todo => (
          <li
            key={todo.id}
            className={`flex items-center gap-3 p-4 rounded-lg border transition-all ${todo.completed
              ? 'bg-gray-50 border-gray-200'
              : 'bg-white border-gray-200 hover:border-gray-300 hover:shadow-sm'
            } ${isPending ? 'opacity-60' : ''}`}
          >
            <button
              onClick={() => handleToggle(todo.id)}
              disabled={isPending}
              className={`shrink-0 w-5 h-5 rounded border-2 flex items-center justify-center transition-all ${todo.completed
                ? 'bg-indigo-600 border-indigo-600'
                : 'border-gray-300 hover:border-indigo-500'
              } ${isPending ? 'cursor-not-allowed' : 'cursor-pointer'}`}
              aria-label={
                todo.completed ? 'Mark as incomplete' : 'Mark as complete'
              }
            >
              {todo.completed && (
                <svg
                  className="w-3 h-3 text-white"
                  fill="currentColor"
                  viewBox="0 0 12 12"
                >
                  <path d="M10.28 2.28L3.989 8.575 1.695 6.28A1 1 0 00.28 7.695l3 3a1 1 0 001.414 0l7-7A1 1 0 0010.28 2.28z" />
                </svg>
              )}
            </button>

            <span
              className={`flex-1 text-sm transition-all ${todo.completed ? 'line-through text-gray-400' : 'text-gray-900'
              }`}
            >
              {todo.text}
            </span>

            <button
              onClick={() => handleDelete(todo.id)}
              disabled={isPending}
              className={`shrink-0 w-8 h-8 rounded-lg flex items-center justify-center transition-all ${isPending
                ? 'text-gray-300 cursor-not-allowed'
                : 'text-gray-400 hover:text-red-600 hover:bg-red-50'
              }`}
              aria-label="Delete todo"
            >
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <path
                  fillRule="evenodd"
                  d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z"
                  clipRule="evenodd"
                />
              </svg>
            </button>
          </li>
        ))}
      </ul>

      {todos.length === 0 && (
        <div className="text-center py-12 text-gray-500">
          <svg
            className="w-12 h-12 mx-auto mb-3 text-gray-300"
            fill="currentColor"
            viewBox="0 0 20 20"
          >
            <path d="M9 2a1 1 0 000 2h2a1 1 0 100-2H9z" />
            <path
              fillRule="evenodd"
              d="M4 5a2 2 0 012-2 3 3 0 003 3h2a3 3 0 003-3 2 2 0 012 2v11a2 2 0 01-2 2H6a2 2 0 01-2-2V5zm9.707 5.707a1 1 0 00-1.414-1.414L9 12.586l-1.293-1.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
              clipRule="evenodd"
            />
          </svg>
          <p className="text-sm">No todos yet. Add one above to get started!</p>
        </div>
      )}

      <div className="mt-6 pt-6 border-t border-gray-200">
        <details className="text-sm text-gray-600">
          <summary className="font-medium text-gray-700 cursor-pointer select-none hover:text-gray-900">
            Features demonstrated
          </summary>
          <ul className="mt-3 ml-4 space-y-2 text-xs leading-relaxed">
            <li className="flex items-start gap-2">
              <span className="text-indigo-600 mt-0.5">•</span>
              <span>Multiple server actions in one component</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-indigo-600 mt-0.5">•</span>
              <span>Pending states during execution</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-indigo-600 mt-0.5">•</span>
              <span>Optimistic UI updates with state management</span>
            </li>
            <li className="flex items-start gap-2">
              <span className="text-indigo-600 mt-0.5">•</span>
              <span>Error handling with user feedback</span>
            </li>
          </ul>
        </details>
      </div>
    </div>
  )
}
