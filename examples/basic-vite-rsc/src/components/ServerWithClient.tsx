'use server'

import Counter from './Counter'

export default function ServerWithClient() {
  const serverData = {
    timestamp: new Date().toISOString(),
    message: 'This data was generated on the server',
  }

  return (
    <div className="p-6 border border-blue-300 rounded-lg bg-blue-50">
      <h2 className="text-xl font-semibold mb-4 text-blue-800">
        Server Component Container
      </h2>

      <div className="mb-6">
        <p className="text-blue-700 mb-2">
          This is a server component that contains:
        </p>
        <ul className="list-disc list-inside text-blue-600 text-sm">
          <li>Server-side rendered content</li>
          <li>
            Data fetched at request time:
            {serverData.timestamp}
          </li>
          <li>A client component embedded below</li>
        </ul>
      </div>

      <div className="mb-4">
        <p className="text-sm text-blue-600 font-medium">Server Message:</p>
        <p className="text-blue-800 bg-blue-100 p-2 rounded">
          {serverData.message}
        </p>
      </div>

      <div className="border-t border-blue-200 pt-4">
        <h3 className="text-lg font-medium mb-3 text-blue-800">
          Client Component Below:
        </h3>
        <Counter />
      </div>

      <div className="mt-4 text-xs text-blue-500">
        <p>• The blue container is rendered on the server</p>
        <p>• The Counter component should be rendered on the client</p>
        <p>• If working correctly, the counter should be interactive</p>
      </div>
    </div>
  )
}
