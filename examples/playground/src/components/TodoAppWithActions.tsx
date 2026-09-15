'use client'

import type { Todo } from '@/actions/todo-actions'
import { useState } from 'react'
import TodoFormWithActions from './TodoFormWithActions'
import TodoListWithActions from './TodoListWithActions'

interface TodoAppProps {
  readonly initialTodos: readonly Todo[]
}

export default function TodoAppWithActions({ initialTodos }: TodoAppProps) {
  const [todos, setTodos] = useState<readonly Todo[]>(initialTodos)

  return (
    <>
      <TodoFormWithActions onSuccess={setTodos} />
      <TodoListWithActions initialTodos={todos} onUpdate={setTodos} />
    </>
  )
}
