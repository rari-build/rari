export default function UserProfilePage({
  params = {},
  searchParams = {}
}: {
  params?: { [key: string]: string | undefined }
  searchParams?: { [key: string]: string | string[] | undefined }
}) {
  const { id } = params

  const users = {
    1: {
      id: '1',
      name: 'Alice Johnson',
      email: 'alice@example.com',
      avatar: '🚀',
      role: 'Framework Creator',
      bio: 'Building the future of React frameworks with Rari. Passionate about performance, developer experience, and open source.',
      location: 'San Francisco, CA',
      joinDate: '2023-01-15',
      stats: {
        projects: 12,
        commits: 1547,
        followers: 234,
      },
      skills: ['React', 'TypeScript', 'Rust', 'Performance Optimization'],
    },
    2: {
      id: '2',
      name: 'Sarah Johnson',
      email: 'sarah@example.com',
      avatar: '👩‍💻',
      role: 'Full Stack Developer',
      bio: 'Love building scalable web applications. Early adopter of Rari framework and contributor to the ecosystem.',
      location: 'Austin, TX',
      joinDate: '2023-03-22',
      stats: {
        projects: 8,
        commits: 892,
        followers: 156,
      },
      skills: ['React', 'Node.js', 'GraphQL', 'Database Design'],
    },
    3: {
      id: '3',
      name: 'Alex Chen',
      email: 'alex@example.com',
      avatar: '🎨',
      role: 'UI/UX Designer',
      bio: 'Designing beautiful and functional user interfaces. Advocate for accessibility and inclusive design.',
      location: 'Seattle, WA',
      joinDate: '2023-05-10',
      stats: {
        projects: 15,
        commits: 432,
        followers: 312,
      },
      skills: ['UI Design', 'Figma', 'Accessibility', 'Design Systems'],
    },
    123: {
      id: '123',
      name: 'Demo User',
      email: 'demo@example.com',
      avatar: '🔧',
      role: 'Demo Account',
      bio: 'This is a demo user account used to showcase the dynamic routing capabilities of Rari framework.',
      location: 'Virtual Space',
      joinDate: '2024-01-01',
      stats: {
        projects: 3,
        commits: 127,
        followers: 42,
      },
      skills: ['Demo', 'Testing', 'Examples', 'Documentation'],
    },
  }

  // Safely handle the id parameter which could be string or string[]
  const userId = Array.isArray(id) ? id[0] : id

  // Type-safe user lookup without casting
  function getUser(id: string | undefined) {
    if (!id)
      return null
    const userEntry = Object.entries(users).find(([key]) => key === id)
    return userEntry ? userEntry[1] : null
  }

  const user = getUser(userId)

  if (!user) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-red-50 to-orange-100 py-8 px-4">
        <div className="max-w-4xl mx-auto text-center">
          <div className="mb-8">
            <a
              href="/"
              className="inline-flex items-center text-red-600 hover:text-red-800 transition-colors"
            >
              <svg
                className="w-5 h-5 mr-2"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M10 19l-7-7m0 0l7-7m-7 7h18"
                />
              </svg>
              Back to Home
            </a>
          </div>

          <h1 className="text-4xl font-bold text-gray-900 mb-4">
            User Not Found
          </h1>
          <p className="text-xl text-gray-600 mb-8">
            The user with ID "
            {id}
            " doesn't exist.
          </p>

          <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">
              🗺️ Route Information
            </h2>
            <div className="grid gap-4 md:grid-cols-2">
              <div>
                <div className="text-sm text-gray-500 mb-1">
                  Requested User ID
                </div>
                <code className="text-sm font-mono text-gray-800">{id}</code>
              </div>
              <div>
                <div className="text-sm text-gray-500 mb-1">Route Pattern</div>
                <code className="text-sm font-mono text-gray-800">
                  /users/[id]
                </code>
              </div>
            </div>

            <div className="mt-6 p-4 bg-blue-50 rounded-lg">
              <h3 className="font-semibold text-blue-900 mb-2">
                Try these users:
              </h3>
              <div className="space-y-2">
                {Object.keys(users).map(userId => (
                  <a
                    key={userId}
                    href={`/users/${userId}`}
                    className="block text-blue-600 hover:text-blue-800 transition-colors"
                  >
                    /users/
                    {userId}
                  </a>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-indigo-50 to-cyan-100 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        <div className="mb-8">
            <a
              href="/"
              className="inline-flex items-center text-indigo-600 hover:text-indigo-800 transition-colors"
            >
            <svg
              className="w-5 h-5 mr-2"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M10 19l-7-7m0 0l7-7m-7 7h18"
              />
            </svg>
            Back to Home
          </a>
        </div>

        <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200 mb-8">
          <div className="flex items-start space-x-6">
            <div className="text-6xl">{user.avatar}</div>
            <div className="flex-1">
              <h1 className="text-3xl font-bold text-gray-900 mb-2">
                {user.name}
              </h1>
              <p className="text-xl text-indigo-600 mb-4">{user.role}</p>
              <p className="text-gray-600 mb-4">{user.bio}</p>

              <div className="flex items-center space-x-6 text-sm text-gray-500">
                <div className="flex items-center">
                  <svg
                    className="w-4 h-4 mr-1"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M3 8l7.89 4.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
                    />
                  </svg>
                  {user.email}
                </div>
                <div className="flex items-center">
                  <svg
                    className="w-4 h-4 mr-1"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"
                    />
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M15 11a3 3 0 11-6 0 3 3 0 016 0z"
                    />
                  </svg>
                  {user.location}
                </div>
                <div className="flex items-center">
                  <svg
                    className="w-4 h-4 mr-1"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v14a2 2 0 002 2z"
                    />
                  </svg>
                  Joined
                  {' '}
                  {new Date(user.joinDate).toLocaleDateString('en-US', {
                    year: 'numeric',
                    month: 'long',
                    day: 'numeric',
                  })}
                </div>
              </div>
            </div>
          </div>
        </div>

        <div className="grid gap-8 md:grid-cols-3 mb-8">
          <div className="bg-white rounded-xl p-6 shadow-sm border border-gray-200">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">
              📊 Stats
            </h3>
            <div className="space-y-4">
              <div className="flex justify-between">
                <span className="text-gray-600">Projects</span>
                <span className="font-semibold text-gray-900">
                  {user.stats.projects}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Commits</span>
                <span className="font-semibold text-gray-900">
                  {user.stats.commits.toLocaleString()}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Followers</span>
                <span className="font-semibold text-gray-900">
                  {user.stats.followers}
                </span>
              </div>
            </div>
          </div>

          <div className="bg-white rounded-xl p-6 shadow-sm border border-gray-200">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">
              🛠️ Skills
            </h3>
            <div className="flex flex-wrap gap-2">
              {user.skills.map(skill => (
                <span
                  key={skill}
                  className="px-3 py-1 text-sm font-medium text-indigo-700 bg-indigo-100 rounded-full"
                >
                  {skill}
                </span>
              ))}
            </div>
          </div>

          <div className="bg-white rounded-xl p-6 shadow-sm border border-gray-200">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">
              🔗 Quick Actions
            </h3>
            <div className="space-y-3">
              <button
                type="button"
                className="w-full px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors"
              >
                Follow
              </button>
              <button
                type="button"
                className="w-full px-4 py-2 border border-gray-300 text-gray-700 rounded-lg hover:bg-gray-50 transition-colors"
              >
                Message
              </button>
              <button
                type="button"
                className="w-full px-4 py-2 border border-gray-300 text-gray-700 rounded-lg hover:bg-gray-50 transition-colors"
              >
                View Projects
              </button>
            </div>
          </div>
        </div>

        <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200 mb-8">
          <h2 className="text-2xl font-bold text-gray-900 mb-6">
            🗺️ Dynamic Routing Demo
          </h2>

          <div className="grid gap-6 md:grid-cols-2">
            <div>
              <h3 className="text-lg font-semibold text-gray-900 mb-3">
                How it works
              </h3>
              <p className="text-gray-600 mb-4">
                This page demonstrates dynamic routing using the [id] parameter.
                The file structure automatically creates the route pattern.
              </p>
              <div className="space-y-2 text-sm">
                <div className="p-3 bg-gray-50 rounded-lg">
                  <div className="font-mono text-gray-700 mb-1">
                    pages/users/[id].tsx
                  </div>
                  <div className="text-gray-500">→ /users/:id</div>
                </div>
                <div className="p-3 bg-gray-50 rounded-lg">
                  <div className="font-mono text-gray-700 mb-1">
                    Current route:
                  </div>
                  <div className="text-indigo-600">
                    /users/
                    {id}
                  </div>
                </div>
              </div>
            </div>

            <div>
              <h3 className="text-lg font-semibold text-gray-900 mb-3">
                Try other users
              </h3>
              <div className="space-y-2">
                {Object.values(users)
                  .filter(u => u.id !== id)
                  .slice(0, 3)
                  .map(otherUser => (
                    <a
                      key={otherUser.id}
                      href={`/users/${otherUser.id}`}
                      className="flex items-center p-3 border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors"
                    >
                      <span className="text-2xl mr-3">{otherUser.avatar}</span>
                      <div>
                        <div className="font-medium text-gray-900">
                          {otherUser.name}
                        </div>
                        <div className="text-sm text-gray-500">
                          /users/
                          {otherUser.id}
                        </div>
                      </div>
                      <svg
                        className="w-4 h-4 ml-auto text-gray-400"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M9 5l7 7-7 7"
                        />
                      </svg>
                    </a>
                  ))}
              </div>
            </div>
          </div>
        </div>

        <div className="bg-gradient-to-r from-indigo-50 to-cyan-50 rounded-xl p-8 border border-gray-200">
          <h2 className="text-2xl font-bold text-gray-900 mb-4">
            🎯 Route Parameters
          </h2>
          <div className="grid gap-4 md:grid-cols-3">
            <div className="bg-white p-4 rounded-lg">
              <div className="text-sm text-gray-500 mb-1">File Path</div>
              <code className="text-sm font-mono text-gray-800">
                pages/users/[id].tsx
              </code>
            </div>
            <div className="bg-white p-4 rounded-lg">
              <div className="text-sm text-gray-500 mb-1">Route Pattern</div>
              <code className="text-sm font-mono text-gray-800">
                /users/:id
              </code>
            </div>
            <div className="bg-white p-4 rounded-lg">
              <div className="text-sm text-gray-500 mb-1">Current ID</div>
              <code className="text-sm font-mono text-indigo-600">{id}</code>
            </div>
          </div>

          <div className="mt-4 grid gap-4 md:grid-cols-2">
            <div className="bg-white p-4 rounded-lg">
              <div className="text-sm text-gray-500 mb-1">Route Parameters</div>
              <code className="text-sm font-mono text-gray-800">
                {JSON.stringify(params, null, 2)}
              </code>
            </div>
            <div className="bg-white p-4 rounded-lg">
              <div className="text-sm text-gray-500 mb-1">
                Search Parameters
              </div>
              <code className="text-sm font-mono text-gray-800">
                {Object.keys(searchParams).length > 0
                  ? JSON.stringify(searchParams, null, 2)
                  : 'None'}
              </code>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
