async function FetchExample() {
  const response = await fetch('https://jsonplaceholder.typicode.com/posts/1')

  if (!response.ok) {
    throw new Error(`Failed to fetch: ${response.status}`)
  }

  const post = await response.json()

  const currentTime = new Date().toLocaleTimeString()

  return (
    <div className="p-5 bg-white border rounded-lg shadow-sm" data-component-id="fetchexample">
      <h1 className="text-2xl font-bold text-blue-700 mb-2">Fetch Example (External API)</h1>

      <div className="mb-4 text-sm text-gray-500">
        Server time:
        {' '}
        {currentTime}
      </div>

      <div className="bg-gray-50 p-4 rounded border">
        <h2 className="text-lg font-semibold text-gray-800 mb-3">Real Post from JSONPlaceholder:</h2>

        <div className="space-y-2">
          <div>
            <span className="font-medium text-gray-700">Title:</span>
            {' '}
            {post.title}
          </div>

          <div>
            <span className="font-medium text-gray-700">Body:</span>
            {' '}
            {post.body}
          </div>

          <div>
            <span className="font-medium text-gray-700">User ID:</span>
            {' '}
            {post.userId}
          </div>

          <div>
            <span className="font-medium text-gray-700">Post ID:</span>
            {' '}
            {post.id}
          </div>
        </div>
      </div>

      <div className="mt-4 text-xs text-gray-400">
        This component uses async/await and fetch() to load data from an external API.
      </div>
    </div>
  )
}

export default FetchExample
