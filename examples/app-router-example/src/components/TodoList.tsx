'use client'

import { useState } from 'react'

export default function TodoList() {
  const [todos, setTodos] = useState<Array<{ id: number, text: string, done: boolean }>>([
    { id: 1, text: 'Test RSC wire format', done: true },
    { id: 2, text: 'Add client components', done: true },
    { id: 3, text: 'Verify streaming works', done: false },
  ])
  const [newTodo, setNewTodo] = useState('')

  const addTodo = () => {
    if (newTodo.trim()) {
      setTodos([...todos, { id: Date.now(), text: newTodo, done: false }])
      setNewTodo('')
    }
  }

  const toggleTodo = (id: number) => {
    setTodos(todos.map(todo =>
      todo.id === id ? { ...todo, done: !todo.done } : todo
    ))
  }

  const deleteTodo = (id: number) => {
    setTodos(todos.filter(todo => todo.id !== id))
  }

  return (
    <div className="bg-white p-8 rounded-lg border-2 border-blue-600">
      <h2 className="text-blue-600 mb-6 text-2xl font-semibold">
        Interactive Todo List (Client Component)
      </h2>

      <div className="flex gap-2 mb-6">
        <input
          type="text"
          value={newTodo}
          onChange={(e) => setNewTodo(e.target.value)}
          onKeyPress={(e) => e.key === 'Enter' && addTodo()}
          placeholder="Add a new todo..."
          className="flex-1 px-3 py-3 text-base border-2 border-gray-200 rounded"
        />
        <button
          onClick={addTodo}
          className="px-6 py-3 text-base bg-blue-600 text-white border-none rounded cursor-pointer hover:bg-blue-700 transition-colors"
        >
          Add
        </button>
      </div>

      <ul className="list-none p-0">
        {todos.map(todo => (
          <li
            key={todo.id}
            className="flex items-center gap-4 p-4 bg-gray-50 rounded mb-2"
          >
            <input
              type="checkbox"
              checked={todo.done}
              onChange={() => toggleTodo(todo.id)}
              className="w-5 h-5 cursor-pointer"
            />
            <span className={`flex-1 ${todo.done ? 'line-through text-gray-400' : 'text-gray-900'}`}>
              {todo.text}
            </span>
            <button
              onClick={() => deleteTodo(todo.id)}
              className="px-4 py-2 bg-red-500 text-white border-none rounded cursor-pointer hover:bg-red-600 transition-colors"
            >
              Delete
            </button>
          </li>
        ))}
      </ul>

      <p className="mt-4 text-gray-600 text-sm">
        {todos.filter(t => !t.done).length} of {todos.length} todos remaining
      </p>
    </div>
  )
}

