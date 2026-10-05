import { component$, Resource, useResource$ } from '@qwik.dev/core'

// Memoised like the React/Next versions (rari's fetch cache, Next's fetch
// cache): one external request per minute, not one per render.
const POST_TTL_MS = 60_000
let cachedPost: { value: unknown; at: number } | undefined

async function loadPost(): Promise<unknown> {
  if (cachedPost && Date.now() - cachedPost.at < POST_TTL_MS) return cachedPost.value
  const response = await fetch('https://jsonplaceholder.typicode.com/posts/1')
  if (!response.ok) throw new Error(`Failed to fetch: ${response.status}`)
  const value: unknown = await response.json()
  cachedPost = { value, at: Date.now() }
  return value
}

export default component$(() => {
  const postResource = useResource$(async () => {
    const post = (await loadPost()) as { title: string; body: string; userId: number; id: number }
    return post
  })

  return (
    <Resource
      value={postResource}
      onResolved={post => {
        const currentTime = new Date().toLocaleTimeString()

        return (
          <div class="p-5 bg-white border rounded-lg shadow-sm" data-component-id="fetchexample">
            <h1 class="text-2xl font-bold text-blue-700 mb-2">Fetch Example (External API)</h1>

            <div class="mb-4 text-sm text-gray-500">Server time: {currentTime}</div>

            <div class="bg-gray-50 p-4 rounded border">
              <h2 class="text-lg font-semibold text-gray-800 mb-3">
                Real Post from JSONPlaceholder:
              </h2>

              <div class="space-y-2">
                <div>
                  <span class="font-medium text-gray-700">Title:</span> {post.title}
                </div>

                <div>
                  <span class="font-medium text-gray-700">Body:</span> {post.body}
                </div>

                <div>
                  <span class="font-medium text-gray-700">User ID:</span> {post.userId}
                </div>

                <div>
                  <span class="font-medium text-gray-700">Post ID:</span> {post.id}
                </div>
              </div>
            </div>

            <div class="mt-4 text-xs text-gray-400">
              This component uses async/await and fetch() to load data from an external API.
            </div>
          </div>
        )
      }}
    />
  )
})
