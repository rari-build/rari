import { component$, Resource, useResource$ } from '@qwik.dev/core'

interface BlueskyPost {
  uri: string
  cid: string
  author: {
    did: string
    handle: string
    displayName?: string
    avatar?: string
  }
  record: {
    text: string
    createdAt: string
    langs?: string[]
  }
  replyCount: number
  repostCount: number
  likeCount: number
  indexedAt: string
}

interface BlueskyFeedResponse {
  feed: Array<{
    post: BlueskyPost
  }>
  cursor?: string
}

// Memoised like the React/Next versions (rari's fetch cache, Next's fetch
// cache): one external request per minute, not one per render.
const FEED_TTL_MS = 60_000
let cachedFeed: { value: BlueskyFeedResponse; at: number } | undefined

async function loadFeed(): Promise<BlueskyFeedResponse> {
  if (cachedFeed && Date.now() - cachedFeed.at < FEED_TTL_MS) return cachedFeed.value
  const response = await fetch(
    'https://public.api.bsky.app/xrpc/app.bsky.feed.getFeed?feed=at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.generator/whats-hot&limit=10',
    { headers: { Accept: 'application/json' } },
  )
  if (!response.ok) throw new Error(`Failed to fetch Bluesky feed: ${response.status}`)
  const value: BlueskyFeedResponse = await response.json()
  cachedFeed = { value, at: Date.now() }
  return value
}

export default component$(() => {
  const feedResource = useResource$<BlueskyFeedResponse>(async () => loadFeed())

  return (
    <Resource
      value={feedResource}
      onResolved={data => {
        const currentTime = new Date().toLocaleTimeString()

        return (
          <div class="p-5 bg-white border rounded-lg shadow-sm" data-component-id="whatshot">
            <h1 class="text-2xl font-bold text-blue-600 mb-2">🔥 What's Hot on Bluesky</h1>

            <div class="mb-4 text-sm text-gray-500">
              Fetched at: {currentTime} • {data.feed.length} trending posts
            </div>

            <div class="space-y-4">
              {data.feed.slice(0, 5).map((item, index) => {
                const post = item.post
                const timeAgo = new Date(post.record.createdAt).toLocaleDateString()

                return (
                  <div
                    key={post.uri}
                    class="border-l-4 border-blue-500 pl-4 py-3 bg-gray-50 rounded-r"
                  >
                    <div class="flex items-start justify-between mb-2">
                      <div class="flex items-center space-x-2">
                        {post.author.avatar && (
                          <img
                            src={post.author.avatar}
                            alt={post.author.displayName || post.author.handle}
                            class="w-8 h-8 rounded-full"
                          />
                        )}
                        <div>
                          <div class="font-semibold text-gray-800">
                            {post.author.displayName || post.author.handle}
                          </div>
                          <div class="text-sm text-gray-500">@{post.author.handle}</div>
                        </div>
                      </div>
                      <div class="text-xs text-gray-400">#{index + 1}</div>
                    </div>

                    <p class="text-gray-700 mb-3 leading-relaxed">
                      {post.record.text.length > 200
                        ? `${post.record.text.substring(0, 200)}...`
                        : post.record.text}
                    </p>

                    <div class="flex items-center space-x-4 text-sm text-gray-500">
                      <span class="flex items-center space-x-1">
                        <span>💬</span>
                        <span>{post.replyCount}</span>
                      </span>
                      <span class="flex items-center space-x-1">
                        <span>🔄</span>
                        <span>{post.repostCount}</span>
                      </span>
                      <span class="flex items-center space-x-1">
                        <span>❤️</span>
                        <span>{post.likeCount}</span>
                      </span>
                      <span class="ml-auto text-xs">{timeAgo}</span>
                    </div>
                  </div>
                )
              })}
            </div>

            <div class="mt-6 text-xs text-gray-400 border-t pt-4">
              <p>Data fetched from Bluesky's public API using the "What's Hot" algorithmic feed.</p>
              <p>
                This demonstrates server-side rendering with external API calls - no authentication
                required!
              </p>
            </div>
          </div>
        )
      }}
      onRejected={error => {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error'

        return (
          <div
            class="p-5 bg-red-50 border border-red-200 rounded-lg shadow-sm"
            data-component-id="whatshot-error"
          >
            <h1 class="text-2xl font-bold text-red-700 mb-2">🔥 What's Hot on Bluesky</h1>

            <div class="bg-red-100 border border-red-300 rounded p-4">
              <h2 class="text-lg font-semibold text-red-800 mb-2">Failed to load trending posts</h2>
              <p class="text-red-700 mb-3">
                Error:
                {errorMessage}
              </p>

              <div class="text-sm text-red-600">
                <p>Possible reasons:</p>
                <ul class="list-disc list-inside mt-2 space-y-1">
                  <li>Network connectivity issues</li>
                  <li>Bluesky API temporarily unavailable</li>
                  <li>Rate limiting or API changes</li>
                </ul>
              </div>
            </div>

            <div class="mt-4 text-xs text-gray-500">
              This component demonstrates error handling in server components.
            </div>
          </div>
        )
      }}
    />
  )
})
