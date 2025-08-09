'use server'

import { Suspense } from 'react'

async function fetchUserProfile(delay = 300) {
  await new Promise(resolve => setTimeout(resolve, delay))
  return {
    name: 'Sarah Connor',
    role: 'Software Engineer',
    avatar: '(dev)',
    location: 'San Francisco, CA',
  }
}

async function fetchUserPosts(delay = 600) {
  await new Promise(resolve => setTimeout(resolve, delay))
  return [
    {
      id: 1,
      title: 'Understanding React Suspense',
      likes: 42,
      comments: 8,
      date: '2 days ago',
    },
    {
      id: 2,
      title: 'Building Scalable Web Apps',
      likes: 67,
      comments: 12,
      date: '1 week ago',
    },
    {
      id: 3,
      title: 'The Future of Server Components',
      likes: 89,
      comments: 15,
      date: '2 weeks ago',
    },
  ]
}

async function fetchUserActivity(delay = 900) {
  await new Promise(resolve => setTimeout(resolve, delay))
  return [
    {
      id: 1,
      action: 'Commented on "React Best Practices"',
      time: '5 min ago',
      type: 'comment',
    },
    {
      id: 2,
      action: 'Starred repository "awesome-react"',
      time: '1 hour ago',
      type: 'star',
    },
    { id: 3, action: 'Followed @reactjs', time: '3 hours ago', type: 'follow' },
    {
      id: 4,
      action: 'Created pull request #42',
      time: '1 day ago',
      type: 'pr',
    },
  ]
}

async function fetchUserStats(delay = 450) {
  await new Promise(resolve => setTimeout(resolve, delay))
  return {
    repositories: 23,
    followers: 156,
    following: 89,
    contributions: 847,
  }
}

async function UserActivity() {
  const activities = await fetchUserActivity(900)

  const getActivityIcon = (type: string) => {
    switch (type) {
      case 'comment':
        return '(comment)'
      case 'star':
        return '(star)'
      case 'follow':
        return '(follow)'
      case 'pr':
        return '(pr)'
      default:
        return '(note)'
    }
  }

  return (
    <div className="bg-purple-50 border border-purple-200 rounded-lg p-4">
      <h4 className="text-sm font-semibold text-purple-900 mb-3 flex items-center">
        <span className="w-2 h-2 bg-purple-500 rounded-full mr-2"></span>
        Recent Activity (Level 3 - Deepest)
      </h4>
      <div className="space-y-2">
        {activities.map(activity => (
          <div key={activity.id} className="flex items-start space-x-3 text-sm">
            <span className="text-lg">{getActivityIcon(activity.type)}</span>
            <div className="flex-1">
              <div className="text-purple-800">{activity.action}</div>
              <div className="text-purple-600 text-xs">{activity.time}</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

async function UserPosts() {
  const posts = await fetchUserPosts(600)

  return (
    <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
      <h4 className="text-sm font-semibold text-blue-900 mb-3 flex items-center">
        <span className="w-2 h-2 bg-blue-500 rounded-full mr-2"></span>
        Recent Posts (Level 2 - Middle)
      </h4>
      <div className="space-y-3">
        {posts.map(post => (
          <div
            key={post.id}
            className="bg-white p-3 rounded border border-blue-100"
          >
            <h5 className="font-medium text-blue-900 text-sm mb-1">
              {post.title}
            </h5>
            <div className="flex items-center space-x-4 text-xs text-blue-600">
              <span>
                (likes)
                {post.likes}
              </span>
              <span>
                (comments)
                {post.comments}
              </span>
              <span>
                (date)
                {post.date}
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

async function UserStats() {
  const stats = await fetchUserStats(450)

  return (
    <div className="bg-green-50 border border-green-200 rounded-lg p-4">
      <h4 className="text-sm font-semibold text-green-900 mb-3 flex items-center">
        <span className="w-2 h-2 bg-green-500 rounded-full mr-2"></span>
        GitHub Stats (Level 2 - Middle)
      </h4>
      <div className="grid grid-cols-2 gap-3">
        <div className="text-center">
          <div className="text-lg font-bold text-green-800">
            {stats.repositories}
          </div>
          <div className="text-xs text-green-600">Repos</div>
        </div>
        <div className="text-center">
          <div className="text-lg font-bold text-green-800">
            {stats.followers}
          </div>
          <div className="text-xs text-green-600">Followers</div>
        </div>
        <div className="text-center">
          <div className="text-lg font-bold text-green-800">
            {stats.following}
          </div>
          <div className="text-xs text-green-600">Following</div>
        </div>
        <div className="text-center">
          <div className="text-lg font-bold text-green-800">
            {stats.contributions}
          </div>
          <div className="text-xs text-green-600">Contributions</div>
        </div>
      </div>
    </div>
  )
}

async function UserProfile() {
  const user = await fetchUserProfile(300)

  return (
    <div className="bg-orange-50 border border-orange-200 rounded-lg p-6">
      <h3 className="text-lg font-semibold text-orange-900 mb-4 flex items-center">
        <span className="w-3 h-3 bg-orange-500 rounded-full mr-3"></span>
        User Profile (Level 1 - Root)
      </h3>
      <div className="flex items-center space-x-4 mb-6">
        <div className="text-4xl">{user.avatar}</div>
        <div>
          <h4 className="text-xl font-semibold text-orange-900">{user.name}</h4>
          <p className="text-orange-700">{user.role}</p>
          <p className="text-orange-600 text-sm">
            (location)
            {user.location}
          </p>
        </div>
      </div>

      <div className="grid md:grid-cols-2 gap-4 mb-6">
        <Suspense
          fallback={<Level2Skeleton color="blue" title="Loading Posts..." />}
        >
          <UserPosts />
        </Suspense>

        <Suspense
          fallback={<Level2Skeleton color="green" title="Loading Stats..." />}
        >
          <UserStats />
        </Suspense>
      </div>

      <Suspense fallback={<Level3Skeleton />}>
        <UserActivity />
      </Suspense>
    </div>
  )
}

function Level1Skeleton() {
  return (
    <div className="bg-orange-50 border border-orange-200 rounded-lg p-6 animate-pulse">
      <div className="flex items-center space-x-2 mb-4">
        <div className="w-3 h-3 bg-orange-300 rounded-full"></div>
        <div className="h-6 bg-orange-200 rounded w-48"></div>
      </div>
      <div className="flex items-center space-x-4 mb-6">
        <div className="w-16 h-16 bg-orange-200 rounded-full"></div>
        <div className="space-y-2">
          <div className="h-6 bg-orange-200 rounded w-32"></div>
          <div className="h-4 bg-orange-200 rounded w-24"></div>
          <div className="h-4 bg-orange-200 rounded w-36"></div>
        </div>
      </div>
      <div className="grid md:grid-cols-2 gap-4 mb-6">
        <div className="h-32 bg-orange-200 rounded-lg"></div>
        <div className="h-32 bg-orange-200 rounded-lg"></div>
      </div>
      <div className="h-24 bg-orange-200 rounded-lg"></div>
    </div>
  )
}

function Level2Skeleton({ color, title }: { color: string, title: string }) {
  const colorClasses = {
    blue: 'bg-blue-50 border-blue-200 bg-blue-200',
    green: 'bg-green-50 border-green-200 bg-green-200',
  }

  return (
    <div
      className={`${colorClasses[color as keyof typeof colorClasses].split(' ').slice(0, 2).join(' ')} rounded-lg p-4 animate-pulse`}
    >
      <div className="flex items-center space-x-2 mb-3">
        <div
          className={`w-2 h-2 ${colorClasses[color as keyof typeof colorClasses].split(' ')[2]} rounded-full`}
        >
        </div>
        <div className="text-xs text-gray-600">{title}</div>
      </div>
      <div className="space-y-2">
        <div
          className={`h-4 ${colorClasses[color as keyof typeof colorClasses].split(' ')[2]} rounded`}
        >
        </div>
        <div
          className={`h-4 ${colorClasses[color as keyof typeof colorClasses].split(' ')[2]} rounded w-3/4`}
        >
        </div>
        <div
          className={`h-4 ${colorClasses[color as keyof typeof colorClasses].split(' ')[2]} rounded w-1/2`}
        >
        </div>
      </div>
    </div>
  )
}

function Level3Skeleton() {
  return (
    <div className="bg-purple-50 border border-purple-200 rounded-lg p-4 animate-pulse">
      <div className="flex items-center space-x-2 mb-3">
        <div className="w-2 h-2 bg-purple-300 rounded-full"></div>
        <div className="text-xs text-gray-600">Loading Activity...</div>
      </div>
      <div className="space-y-2">
        {[1, 2, 3, 4].map(i => (
          <div key={i} className="flex items-center space-x-3">
            <div className="w-4 h-4 bg-purple-200 rounded"></div>
            <div className="flex-1 space-y-1">
              <div className="h-3 bg-purple-200 rounded w-3/4"></div>
              <div className="h-2 bg-purple-200 rounded w-1/2"></div>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

export default function NestedSuspense() {
  return (
    <div className="space-y-6">
      <div className="text-center mb-8">
        <h2 className="text-2xl font-bold text-gray-900 mb-2">
          Nested Suspense Demo
        </h2>
        <p className="text-gray-600">
          See how multiple Suspense boundaries work together in a hierarchy
        </p>
      </div>

      <div className="bg-gray-50 border border-gray-200 rounded-lg p-6 mb-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">
          Suspense Boundary Hierarchy
        </h3>
        <div className="space-y-3 text-sm">
          <div className="flex items-center space-x-3">
            <div className="w-6 h-6 bg-orange-500 rounded-full flex items-center justify-center text-white text-xs font-bold">
              1
            </div>
            <div className="text-orange-700">
              Level 1 (Root): User Profile (~300ms)
            </div>
          </div>
          <div className="flex items-center space-x-3 ml-6">
            <div className="w-6 h-6 bg-blue-500 rounded-full flex items-center justify-center text-white text-xs font-bold">
              2
            </div>
            <div className="text-blue-700">
              Level 2: User Posts (~600ms) & Stats (~450ms)
            </div>
          </div>
          <div className="flex items-center space-x-3 ml-12">
            <div className="w-6 h-6 bg-purple-500 rounded-full flex items-center justify-center text-white text-xs font-bold">
              3
            </div>
            <div className="text-purple-700">
              Level 3 (Deepest): User Activity (~900ms)
            </div>
          </div>
        </div>
      </div>

      <Suspense fallback={<Level1Skeleton />}>
        <UserProfile />
      </Suspense>

      <div className="mt-8 p-6 bg-gradient-to-br from-indigo-50 to-purple-50 border border-indigo-200 rounded-lg">
        <h3 className="text-lg font-semibold text-indigo-900 mb-3">
          How Nested Suspense Works
        </h3>
        <div className="grid md:grid-cols-2 gap-6">
          <div className="text-sm text-indigo-800 space-y-2">
            <div>
              <strong>Hierarchy:</strong>
              {' '}
              Each level has its own Suspense
              boundary
            </div>
            <div>
              <strong>Independence:</strong>
              {' '}
              Inner boundaries don't affect outer
              ones
            </div>
            <div>
              <strong>Granular Control:</strong>
              {' '}
              Different loading states for
              different data
            </div>
            <div>
              <strong>Progressive Enhancement:</strong>
              {' '}
              Content appears as it's
              ready
            </div>
          </div>
          <div className="text-sm text-indigo-800 space-y-2">
            <div>
              <strong>Loading Order:</strong>
            </div>
            <div className="ml-4 space-y-1">
              <div>1. Profile data (300ms) &gt; Shows basic info</div>
              <div>2. Stats data (450ms) &gt; Stats section appears</div>
              <div>3. Posts data (600ms) &gt; Posts section appears</div>
              <div>4. Activity data (900ms) &gt; Activity section appears</div>
            </div>
          </div>
        </div>

        <div className="mt-4 p-4 bg-indigo-100 rounded-lg border border-indigo-300">
          <div className="text-xs font-mono text-indigo-900">
            <div>
              &lt;Suspense fallback=&#123;&lt;Level1Loading /&gt;&#125;&gt;
            </div>
            <div className="ml-2">&lt;UserProfile&gt;</div>
            <div className="ml-4">
              &lt;Suspense fallback=&#123;&lt;Level2Loading /&gt;&#125;&gt;
            </div>
            <div className="ml-6">&lt;UserPosts /&gt;</div>
            <div className="ml-4">&lt;/Suspense&gt;</div>
            <div className="ml-4">
              &lt;Suspense fallback=&#123;&lt;Level3Loading /&gt;&#125;&gt;
            </div>
            <div className="ml-6">&lt;UserActivity /&gt;</div>
            <div className="ml-4">&lt;/Suspense&gt;</div>
            <div className="ml-2">&lt;/UserProfile&gt;</div>
            <div>&lt;/Suspense&gt;</div>
          </div>
        </div>
      </div>
    </div>
  )
}
