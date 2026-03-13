'use client'

import type { Todo } from '@/actions/todo-actions'
import { useRef, useState, useTransition } from 'react'
import { clearCompleted, deleteTodo, resetTodos, toggleTodo } from '@/actions/todo-actions'

interface TodoListProps {
  initialTodos: Todo[]
  onUpdate?: () => void
}

export default function TodoList({ initialTodos, onUpdate }: TodoListProps) {
  const [todos, setTodos] = useState<Todo[]>(initialTodos)
  const [isPending, startTransition] = useTransition()

  const todosKey = initialTodos.map(t => t.id).join(',')
  const prevKeyRef = useRef(todosKey)

  if (prevKeyRef.current !== todosKey) {
    setTodos(initialTodos)
    prevKeyRef.current = todosKey
  }

  const handleToggle = async (id: string) => {
    startTransition(async () => {
      const formData = new FormData()
      formData.append('id', id)
      const result = await toggleTodo(formData)
      if (result.success && result.todos) {
        setTodos(result.todos)
        if (onUpdate)
          onUpdate()
      }
    })
  }

  const handleDelete = async (id: string) => {
    startTransition(async () => {
      const formData = new FormData()
      formData.append('id', id)
      const result = await deleteTodo(formData)
      if (result.success && result.todos) {
        setTodos(result.todos)
        if (onUpdate)
          onUpdate()
      }
    })
  }

  const handleClearCompleted = async () => {
    startTransition(async () => {
      const result = await clearCompleted()
      if (result.success && result.todos) {
        setTodos(result.todos)
        if (onUpdate)
          onUpdate()
      }
    })
  }

  const handleReset = async () => {
    startTransition(async () => {
      const result = await resetTodos()
      if (result.success && result.todos) {
        setTodos(result.todos)
        if (onUpdate)
          onUpdate()
      }
    })
  }

  return (
    <div data-testid="todo-list">
      <h2>Todo List</h2>
      <div data-testid="transition-state">{isPending ? 'pending' : 'idle'}</div>
      <div data-testid="todo-count">
        Total:
        {' '}
        {todos.length}
      </div>
      <ul>
        {todos.map(todo => (
          <li key={todo.id} data-testid={`todo-item-${todo.id}`}>
            <span data-testid={`todo-text-${todo.id}`}>{todo.text}</span>
            <span data-testid={`todo-status-${todo.id}`}>
              {todo.completed ? 'completed' : 'active'}
            </span>
            <button type="button" onClick={() => handleToggle(todo.id)} data-testid={`toggle-button-${todo.id}`} disabled={isPending}>
              Toggle
            </button>
            <button type="button" onClick={() => handleDelete(todo.id)} data-testid={`delete-button-${todo.id}`} disabled={isPending}>
              Delete
            </button>
          </li>
        ))}
      </ul>
      <button type="button" onClick={handleClearCompleted} data-testid="clear-completed-button" disabled={isPending}>
        Clear Completed
      </button>
      <button type="button" onClick={handleReset} data-testid="reset-button" disabled={isPending}>
        Reset Todos
      </button>
    </div>
  )
}
