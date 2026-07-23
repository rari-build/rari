'use client'

import type { Todo } from '@/actions/todo-actions'
import { useState } from 'react'
import { getTodos } from '@/actions/todo-actions'
import TodoFormWithActions from './TodoFormWithActions'
import TodoListWithActions from './TodoListWithActions'

interface TodoAppProps {
  readonly initialTodos: readonly Todo[]
}

export default function TodoAppWithActions({ initialTodos }: TodoAppProps) {
  const [todos, setTodos] = useState<readonly Todo[]>(initialTodos)

  const refreshTodos = async () => {
    const updatedTodos = await getTodos()
    setTodos(updatedTodos)
  }

  return (
    <>
      <TodoFormWithActions
        onSuccess={() => {
          void refreshTodos()
        }}
      />
      <TodoListWithActions
        initialTodos={todos}
        onUpdate={() => {
          void refreshTodos()
        }}
      />
    </>
  )
}
