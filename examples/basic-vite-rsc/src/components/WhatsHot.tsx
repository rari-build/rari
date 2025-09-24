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

async function WhatsHot() {
  try {
    const response = await fetch(
      'https://public.api.bsky.app/xrpc/app.bsky.feed.getFeed?feed=at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.generator/whats-hot&limit=10',
      {
        headers: {
          Accept: 'application/json',
        },
      },
    )

    if (!response.ok) {
      throw new Error(`Failed to fetch Bluesky feed: ${response.status}`)
    }

    const data: BlueskyFeedResponse = await response.json()
    const currentTime = new Date().toLocaleTimeString()

    return (
      <div className="p-5 bg-white border rounded-lg shadow-sm" data-component-id="whatshot">
        <h1 className="text-2xl font-bold text-blue-600 mb-2">🔥 What's Hot on Bluesky</h1>

        <div className="mb-4 text-sm text-gray-500">
          Fetched at:
          {' '}
          {currentTime}
          {' '}
          •
          {' '}
          {data.feed.length}
          {' '}
          trending posts
        </div>

        <div className="space-y-4">
          {data.feed.slice(0, 5).map((item, index) => {
            const post = item.post
            const timeAgo = new Date(post.record.createdAt).toLocaleDateString()

            return (
              <div key={post.uri} className="border-l-4 border-blue-500 pl-4 py-3 bg-gray-50 rounded-r">
                <div className="flex items-start justify-between mb-2">
                  <div className="flex items-center space-x-2">
                    {post.author.avatar && (
                      <img
                        src={post.author.avatar}
                        alt={post.author.displayName || post.author.handle}
                        className="w-8 h-8 rounded-full"
                      />
                    )}
                    <div>
                      <div className="font-semibold text-gray-800">
                        {post.author.displayName || post.author.handle}
                      </div>
                      <div className="text-sm text-gray-500">
                        @
                        {post.author.handle}
                      </div>
                    </div>
                  </div>
                  <div className="text-xs text-gray-400">
                    #
                    {index + 1}
                  </div>
                </div>

                <p className="text-gray-700 mb-3 leading-relaxed">
                  {post.record.text.length > 200
                    ? `${post.record.text.substring(0, 200)}...`
                    : post.record.text}
                </p>

                <div className="flex items-center space-x-4 text-sm text-gray-500">
                  <span className="flex items-center space-x-1">
                    <span>💬</span>
                    <span>{post.replyCount}</span>
                  </span>
                  <span className="flex items-center space-x-1">
                    <span>🔄</span>
                    <span>{post.repostCount}</span>
                  </span>
                  <span className="flex items-center space-x-1">
                    <span>❤️</span>
                    <span>{post.likeCount}</span>
                  </span>
                  <span className="ml-auto text-xs">{timeAgo}</span>
                </div>
              </div>
            )
          })}
        </div>

        <div className="mt-6 text-xs text-gray-400 border-t pt-4">
          <p>Data fetched from Bluesky's public API using the "What's Hot" algorithmic feed.</p>
          <p>This demonstrates server-side rendering with external API calls - no authentication required!</p>
        </div>
      </div>
    )
  }
  catch (error) {
    const errorMessage = error instanceof Error ? error.message : 'Unknown error'

    return (
      <div className="p-5 bg-red-50 border border-red-200 rounded-lg shadow-sm" data-component-id="whatshot-error">
        <h1 className="text-2xl font-bold text-red-700 mb-2">🔥 What's Hot on Bluesky</h1>

        <div className="bg-red-100 border border-red-300 rounded p-4">
          <h2 className="text-lg font-semibold text-red-800 mb-2">Failed to load trending posts</h2>
          <p className="text-red-700 mb-3">
            Error:
            {errorMessage}
          </p>

          <div className="text-sm text-red-600">
            <p>Possible reasons:</p>
            <ul className="list-disc list-inside mt-2 space-y-1">
              <li>Network connectivity issues</li>
              <li>Bluesky API temporarily unavailable</li>
              <li>Rate limiting or API changes</li>
            </ul>
          </div>
        </div>

        <div className="mt-4 text-xs text-gray-500">
          This component demonstrates error handling in server components.
        </div>
      </div>
    )
  }
}

export default WhatsHot
