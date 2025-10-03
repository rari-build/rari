'use client'

import type { Todo } from '../actions/todo-actions'
import { useState } from 'react'
import { getTodos } from '../actions/todo-actions'
import TodoFormWithActions from './TodoFormWithActions'
import TodoListWithActions from './TodoListWithActions'

interface TodoAppProps {
  initialTodos: Todo[]
}

export default function TodoAppWithActions({ initialTodos }: TodoAppProps) {
  const [todos, setTodos] = useState<Todo[]>(initialTodos)

  const refreshTodos = async () => {
    const updatedTodos = await getTodos()
    setTodos(updatedTodos)
  }

  return (
    <>
      <TodoFormWithActions onSuccess={refreshTodos} />
      <TodoListWithActions initialTodos={todos} onUpdate={refreshTodos} />
    </>
  )
}
