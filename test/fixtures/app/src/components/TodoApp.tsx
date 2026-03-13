'use client'

import type { Todo } from '@/actions/todo-actions'
import { useState } from 'react'
import { getTodos } from '@/actions/todo-actions'
import TodoForm from './TodoForm'
import TodoList from './TodoList'

interface TodoAppProps {
  initialTodos: Todo[]
}

export default function TodoApp({ initialTodos }: TodoAppProps) {
  const [todos, setTodos] = useState<Todo[]>(initialTodos)

  const refreshTodos = async () => {
    const updatedTodos = await getTodos()
    setTodos(updatedTodos)
  }

  return (
    <>
      <TodoForm onSuccess={refreshTodos} />
      <TodoList initialTodos={todos} onUpdate={refreshTodos} />
    </>
  )
}
