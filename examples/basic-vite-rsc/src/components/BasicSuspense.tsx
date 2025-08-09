'use server'

import { Suspense } from 'react'

async function fetchUserData(delay = 2000) {
  await new Promise(resolve => setTimeout(resolve, delay))
  return {
    name: 'John Doe',
    email: 'john@example.com',
    avatar: '👨‍💻',
    joinDate: '2024-01-15',
    posts: 42,
    followers: 128,
  }
}

async function UserProfile({ delay }: { delay?: number }) {
  const user = await fetchUserData(delay)

  return (
    <div className="bg-gradient-to-r from-blue-50 to-indigo-50 p-6 rounded-lg border border-blue-200">
      <div className="flex items-center space-x-4 mb-4">
        <div className="text-4xl">{user.avatar}</div>
        <div>
          <h3 className="text-xl font-semibold text-gray-900">{user.name}</h3>
          <p className="text-gray-600">{user.email}</p>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-4 text-center">
        <div className="bg-white p-3 rounded-lg">
          <div className="text-2xl font-bold text-blue-600">{user.posts}</div>
          <div className="text-sm text-gray-500">Posts</div>
        </div>
        <div className="bg-white p-3 rounded-lg">
          <div className="text-2xl font-bold text-green-600">
            {user.followers}
          </div>
          <div className="text-sm text-gray-500">Followers</div>
        </div>
        <div className="bg-white p-3 rounded-lg">
          <div className="text-sm font-medium text-gray-700">Joined</div>
          <div className="text-sm text-gray-500">{user.joinDate}</div>
        </div>
      </div>
    </div>
  )
}

function UserProfileSkeleton() {
  return (
    <div className="bg-gray-50 p-6 rounded-lg border border-gray-200 animate-pulse">
      <div className="flex items-center space-x-4 mb-4">
        <div className="w-16 h-16 bg-gray-200 rounded-full"></div>
        <div className="space-y-2">
          <div className="h-4 bg-gray-200 rounded w-32"></div>
          <div className="h-3 bg-gray-200 rounded w-48"></div>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-4">
        <div className="bg-white p-3 rounded-lg">
          <div className="h-6 bg-gray-200 rounded w-8 mx-auto mb-1"></div>
          <div className="h-3 bg-gray-200 rounded w-12 mx-auto"></div>
        </div>
        <div className="bg-white p-3 rounded-lg">
          <div className="h-6 bg-gray-200 rounded w-8 mx-auto mb-1"></div>
          <div className="h-3 bg-gray-200 rounded w-16 mx-auto"></div>
        </div>
        <div className="bg-white p-3 rounded-lg">
          <div className="h-4 bg-gray-200 rounded w-12 mx-auto mb-1"></div>
          <div className="h-3 bg-gray-200 rounded w-16 mx-auto"></div>
        </div>
      </div>
    </div>
  )
}

export default function BasicSuspense() {
  return (
    <div className="space-y-6">
      <div className="text-center mb-8">
        <h2 className="text-2xl font-bold text-gray-900 mb-2">
          Basic Suspense Demo
        </h2>
        <p className="text-gray-600">
          Watch how Suspense handles loading states automatically
        </p>
      </div>

      <div className="grid gap-6">
        <div>
          <h3 className="text-lg font-semibold text-gray-800 mb-3">
            ⚡ Fast Loading (200ms)
          </h3>
          <Suspense fallback={<UserProfileSkeleton />}>
            <UserProfile delay={200} />
          </Suspense>
        </div>

        <div>
          <h3 className="text-lg font-semibold text-gray-800 mb-3">
            🐌 Slow Loading (1s)
          </h3>
          <Suspense fallback={<UserProfileSkeleton />}>
            <UserProfile delay={1000} />
          </Suspense>
        </div>
      </div>

      <div className="mt-8 p-6 bg-yellow-50 border border-yellow-200 rounded-lg">
        <h3 className="text-lg font-semibold text-yellow-900 mb-3">
          🔍 How Basic Suspense Works
        </h3>
        <div className="text-sm text-yellow-800 space-y-2">
          <div>
            •
            {' '}
            <strong>Server Component</strong>
            : UserProfile is an async server
            component that fetches data
          </div>
          <div>
            •
            {' '}
            <strong>Suspense Boundary</strong>
            : Wraps the async component and
            catches thrown promises
          </div>
          <div>
            •
            {' '}
            <strong>Fallback UI</strong>
            : UserProfileSkeleton shows while data
            is loading
          </div>
          <div>
            •
            {' '}
            <strong>Automatic Resolution</strong>
            : When data arrives, fallback
            is replaced with actual content
          </div>
          <div>
            •
            {' '}
            <strong>No JavaScript Bundle</strong>
            : All rendering happens on
            the server
          </div>
        </div>

        <div className="mt-4 p-3 bg-yellow-100 rounded border border-yellow-300">
          <div className="text-xs font-mono text-yellow-900">
            <div>&lt;Suspense fallback=&#123;&lt;Loading /&gt;&#125;&gt;</div>
            <div className="ml-2">&lt;AsyncComponent /&gt;</div>
            <div>&lt;/Suspense&gt;</div>
          </div>
        </div>
      </div>
    </div>
  )
}
