'use client'

import { useState } from 'react'

export default function Counter() {
  const [count, setCount] = useState(0)

  return (
    <div className="p-6 border border-gray-300 rounded-lg bg-white">
      <h3 className="text-lg font-semibold mb-4">Counter Component</h3>
      <p className="text-gray-600 mb-4">This is a client component with state and interactivity.</p>

      <div className="flex items-center gap-4 mb-4">
        <button
          onClick={() => setCount(count - 1)}
          type="button"
          className="px-3 py-1 bg-red-500 text-white rounded hover:bg-red-600 transition-colors"
        >
          -
        </button>

        <span className="text-2xl font-bold text-blue-600 min-w-[2rem] text-center">
          {count}
        </span>

        <button
          onClick={() => setCount(count + 1)}
          type="button"
          className="px-3 py-1 bg-green-500 text-white rounded hover:bg-green-600 transition-colors"
        >
          +
        </button>
      </div>

      <div className="text-sm text-gray-500">
        <p>• This component uses React hooks (useState)</p>
        <p>• It has event handlers (onClick)</p>
        <p>• It should run on the client, not the server</p>
      </div>
    </div>
  )
}
