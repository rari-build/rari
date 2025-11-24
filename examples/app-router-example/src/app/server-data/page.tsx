import process from 'node:process'

async function fetchData() {
  await new Promise(resolve => setTimeout(resolve, 3000))

  return {
    timestamp: new Date().toISOString(),
    randomNumber: Math.floor(Math.random() * 1000),
    serverInfo: {
      nodeVersion: process.version || 'N/A',
      platform: process.platform || 'N/A',
    },
  }
}

export default async function ServerDataPage() {
  const data = await fetchData()

  return (
    <div className="bg-white rounded-xl p-12 shadow-2xl">
      <h1 className="text-4xl font-bold mb-8 text-gray-900">
        Server Component with Async Data
      </h1>

      <div className="bg-gray-50 p-8 rounded-lg border border-gray-200 mb-8">
        <h2 className="text-gray-900 mb-4 text-2xl font-semibold">
          Data Fetched on Server
        </h2>
        <pre className="bg-gray-800 text-green-400 p-6 rounded overflow-auto text-sm">
          {JSON.stringify(data, null, 2)}
        </pre>
      </div>

      <div className="bg-gray-100 p-6 rounded-lg border border-gray-300">
        <h3 className="text-gray-900 mb-2 text-xl font-semibold">
          🚀 How This Works
        </h3>
        <ul className="leading-loose text-gray-600 pl-6">
          <li>
            <strong>This component runs only on the server</strong>
            {' '}
            - it has
            access to Node.js APIs like
            {' '}
            <code className="bg-gray-200 px-1 py-0.5 rounded font-mono text-sm">
              process
            </code>
          </li>
          <li>
            <strong>The fetchData() function executes during render</strong>
            {' '}
            -
            before any HTML is sent to the client
          </li>
          <li>
            <strong>The client receives fully-rendered HTML</strong>
            {' '}
            - no
            waterfall loading or client-side fetching
          </li>
          <li>
            <strong>Refresh the page</strong>
            {' '}
            - you'll see the timestamp and
            random number change (server-rendered each time)
          </li>
        </ul>
      </div>

      <div className="mt-8 p-6 bg-amber-50 border border-amber-200 rounded-lg">
        <h3 className="text-amber-700 mb-2 text-xl font-semibold">
          💡 With RSC Wire Format
        </h3>
        <p className="text-gray-600 leading-relaxed">
          If we implemented the full RSC wire format, this server component
          could:
        </p>
        <ul className="leading-loose text-gray-600 pl-6 mt-2">
          <li>Stream progressively as data loads</li>
          <li>Show Suspense boundaries during async operations</li>
          <li>Mix with client components for interactivity</li>
          <li>Refetch data without full page reload</li>
        </ul>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Server Data | Rari App Router',
  description: 'Async server component data fetching',
}
