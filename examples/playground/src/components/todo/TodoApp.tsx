'use client'

import type { Todo } from './types'
import { useState } from 'react'
import TodoForm from './TodoForm'
import TodoList from './TodoList'

interface TodoAppProps {
  readonly initialTodos: readonly Todo[]
}

export default function TodoApp({ initialTodos }: TodoAppProps) {
  const [todos, setTodos] = useState<readonly Todo[]>(initialTodos)

  return (
    <>
      <TodoForm onSuccess={setTodos} />
      <TodoList initialTodos={todos} onUpdate={setTodos} />
    </>
  )
}
