'use client'

import { useState } from 'react'

export default function Counter() {
  const [count, setCount] = useState(0)

  return (
    <div className="bg-white p-8 rounded-lg border border-gray-200 shadow-sm text-center">
      <h2 className="text-gray-900 mb-4 text-2xl font-semibold">
        Interactive Counter (Client Component)
      </h2>
      <p className="text-5xl my-4 font-bold text-gray-900">{count}</p>
      <div className="flex gap-4 justify-center">
        <button
          onClick={() => setCount(count - 1)}
          className="px-6 py-3 text-base bg-indigo-600 text-white border-none rounded cursor-pointer hover:bg-indigo-700 transition-colors"
        >
          Decrement
        </button>
        <button
          onClick={() => setCount(0)}
          className="px-6 py-3 text-base bg-gray-600 text-white border-none rounded cursor-pointer hover:bg-gray-700 transition-colors"
        >
          Reset
        </button>
        <button
          onClick={() => setCount(count + 1)}
          className="px-6 py-3 text-base bg-indigo-600 text-white border-none rounded cursor-pointer hover:bg-indigo-700 transition-colors"
        >
          Increment
        </button>
      </div>
      <p className="mt-4 text-gray-600 text-sm">
        This component runs on the client and maintains state
      </p>
    </div>
  )
}
