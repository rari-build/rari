'use client'

import type { Todo } from '@/actions/todo-actions'
import { useState } from 'react'
import { getTodos } from '@/actions/todo-actions'
import TodoForm from './TodoForm'
import TodoList from './TodoList'

interface TodoAppProps {
  readonly initialTodos: readonly Todo[]
}

export default function TodoApp({ initialTodos }: TodoAppProps) {
  const [todos, setTodos] = useState<readonly Todo[]>(initialTodos)

  const refreshTodos = async (todos?: readonly Todo[]) => {
    if (todos) {
      setTodos(todos)
      return
    }

    const updatedTodos = await getTodos()
    setTodos(updatedTodos)
  }

  return (
    <>
      <TodoForm
        onSuccess={todos => {
          void refreshTodos(todos)
        }}
      />
      <TodoList
        initialTodos={todos}
        onUpdate={todos => {
          void refreshTodos(todos)
        }}
      />
    </>
  )
}
