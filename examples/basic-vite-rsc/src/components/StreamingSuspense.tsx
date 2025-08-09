'use server'

import { Suspense } from 'react'

async function fetchDashboardStats(delay = 1000) {
  await new Promise(resolve => setTimeout(resolve, delay))
  return {
    totalUsers: 12847,
    activeToday: 3492,
    revenue: 45678,
    conversionRate: 3.24,
  }
}

async function fetchRecentActivity(delay = 1500) {
  await new Promise(resolve => setTimeout(resolve, delay))
  return [
    {
      id: 1,
      user: 'Alice Johnson',
      action: 'Created new post',
      time: '2 min ago',
    },
    {
      id: 2,
      user: 'Bob Smith',
      action: 'Updated profile',
      time: '5 min ago',
    },
    {
      id: 3,
      user: 'Carol Williams',
      action: 'Deleted comment',
      time: '8 min ago',
    },
  ]
}

async function fetchAnalytics(delay = 2000) {
  await new Promise(resolve => setTimeout(resolve, delay))
  return {
    pageViews: 45320,
    uniqueVisitors: 8945,
    bounceRate: '2.45%',
    sessionDuration: '285s',
  }
}

async function fetchNotifications(delay = 800) {
  await new Promise(resolve => setTimeout(resolve, delay))
  return [
    {
      id: 1,
      title: 'System Update',
      message: 'New features have been deployed successfully.',
      type: 'info',
    },
    {
      id: 2,
      title: 'Security Alert',
      message: 'Please update your password for enhanced security.',
      type: 'warning',
    },
    {
      id: 3,
      title: 'New Feature',
      message: 'Advanced analytics dashboard is now available.',
      type: 'success',
    },
  ]
}

async function StatsCard() {
  const stats = await fetchDashboardStats()

  return (
    <div className="bg-white p-6 rounded-lg shadow-md">
      <h3 className="text-lg font-semibold mb-4 text-gray-800">
        Dashboard Stats
      </h3>
      <div className="grid grid-cols-2 gap-4">
        <div className="text-center">
          <p className="text-2xl font-bold text-blue-600">{stats.totalUsers}</p>
          <p className="text-sm text-gray-600">Total Users</p>
        </div>
        <div className="text-center">
          <p className="text-2xl font-bold text-green-600">
            {stats.activeToday}
          </p>
          <p className="text-sm text-gray-600">Active Today</p>
        </div>
        <div className="text-center">
          <p className="text-2xl font-bold text-purple-600">{stats.revenue}</p>
          <p className="text-sm text-gray-600">Revenue ($)</p>
        </div>
        <div className="text-center">
          <p className="text-2xl font-bold text-orange-600">
            {stats.conversionRate}
          </p>
          <p className="text-sm text-gray-600">Conversion Rate (%)</p>
        </div>
      </div>
    </div>
  )
}

async function RecentActivity() {
  const activities = await fetchRecentActivity()

  return (
    <div className="bg-white p-6 rounded-lg shadow-md">
      <h3 className="text-lg font-semibold mb-4 text-gray-800">
        Recent Activity
      </h3>
      <div className="space-y-3">
        {activities.map(activity => (
          <div
            key={activity.id}
            className="flex items-center justify-between p-3 bg-gray-50 rounded-md"
          >
            <div>
              <p className="font-medium text-gray-800">{activity.user}</p>
              <p className="text-sm text-gray-600">{activity.action}</p>
            </div>
            <span className="text-xs text-gray-500">{activity.time}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

async function Analytics() {
  const analytics = await fetchAnalytics()

  return (
    <div className="bg-white p-6 rounded-lg shadow-md">
      <h3 className="text-lg font-semibold mb-4 text-gray-800">Analytics</h3>
      <div className="space-y-4">
        <div className="flex justify-between items-center p-3 bg-blue-50 rounded-md">
          <span className="font-medium text-blue-800">Page Views</span>
          <span className="font-bold text-blue-600">{analytics.pageViews}</span>
        </div>
        <div className="flex justify-between items-center p-3 bg-blue-50 rounded-md">
          <span className="font-medium text-blue-800">Unique Visitors</span>
          <span className="font-bold text-blue-600">
            {analytics.uniqueVisitors}
          </span>
        </div>
        <div className="flex justify-between items-center p-3 bg-blue-50 rounded-md">
          <span className="font-medium text-blue-800">Bounce Rate</span>
          <span className="font-bold text-blue-600">
            {analytics.bounceRate}
          </span>
        </div>
        <div className="flex justify-between items-center p-3 bg-blue-50 rounded-md">
          <span className="font-medium text-blue-800">
            Avg Session Duration
          </span>
          <span className="font-bold text-blue-600">
            {analytics.sessionDuration}
          </span>
        </div>
      </div>
    </div>
  )
}

async function Notifications() {
  const notifications = await fetchNotifications()

  return (
    <div className="bg-white p-6 rounded-lg shadow-md">
      <h3 className="text-lg font-semibold mb-4 text-gray-800">
        Notifications
      </h3>
      <div className="space-y-3">
        {notifications.map(notification => (
          <div
            key={notification.id}
            className={`p-3 rounded-md border-l-4 ${
              notification.type === 'success'
                ? 'bg-green-50 border-green-400'
                : notification.type === 'warning'
                  ? 'bg-yellow-50 border-yellow-400'
                  : 'bg-blue-50 border-blue-400'
            }`}
          >
            <p className="font-medium text-gray-800">{notification.title}</p>
            <p className="text-sm text-gray-600">{notification.message}</p>
          </div>
        ))}
      </div>
    </div>
  )
}

function DashboardHeader() {
  return (
    <div className="bg-gradient-to-r from-blue-600 to-purple-600 text-white p-6 rounded-lg shadow-lg mb-6">
      <h1 className="text-3xl font-bold mb-2">Executive Dashboard</h1>
      <p className="text-blue-100">
        Real-time insights and performance metrics
      </p>
    </div>
  )
}

function StatsLoadingFallback() {
  return (
    <div className="bg-white p-6 rounded-lg shadow-md">
      <h3 className="text-lg font-semibold mb-4 text-gray-800">
        Dashboard Stats
      </h3>
      <div className="grid grid-cols-2 gap-4">
        <div className="text-center">
          <div className="h-8 bg-gray-200 rounded animate-pulse mb-2"></div>
          <div className="h-4 bg-gray-100 rounded animate-pulse"></div>
        </div>
        <div className="text-center">
          <div className="h-8 bg-gray-200 rounded animate-pulse mb-2"></div>
          <div className="h-4 bg-gray-100 rounded animate-pulse"></div>
        </div>
        <div className="text-center">
          <div className="h-8 bg-gray-200 rounded animate-pulse mb-2"></div>
          <div className="h-4 bg-gray-100 rounded animate-pulse"></div>
        </div>
        <div className="text-center">
          <div className="h-8 bg-gray-200 rounded animate-pulse mb-2"></div>
          <div className="h-4 bg-gray-100 rounded animate-pulse"></div>
        </div>
      </div>
    </div>
  )
}

function SimpleLoadingFallback() {
  return (
    <div className="bg-white p-6 rounded-lg shadow-md">
      <div className="animate-pulse space-y-4">
        <div className="h-6 bg-gray-200 rounded w-1/3"></div>
        <div className="space-y-2">
          <div className="h-4 bg-gray-200 rounded"></div>
          <div className="h-4 bg-gray-200 rounded w-4/5"></div>
          <div className="h-4 bg-gray-200 rounded w-3/5"></div>
        </div>
      </div>
    </div>
  )
}

export default function StreamingSuspense() {
  return (
    <div className="min-h-screen bg-gray-100 p-8">
      <DashboardHeader />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Suspense fallback={<StatsLoadingFallback />}>
          <StatsCard />
        </Suspense>

        <Suspense fallback={<SimpleLoadingFallback />}>
          <Notifications />
        </Suspense>

        <Suspense fallback={<SimpleLoadingFallback />}>
          <RecentActivity />
        </Suspense>

        <Suspense fallback={<SimpleLoadingFallback />}>
          <Analytics />
        </Suspense>
      </div>

      <div className="mt-8 bg-white p-6 rounded-lg shadow-md">
        <h2 className="text-xl font-semibold mb-4 text-gray-800">
          How Streaming Suspense Works
        </h2>
        <div className="text-sm text-gray-600 space-y-2">
          <p>
            <strong>Independent Boundaries:</strong>
            {' '}
            Each section has its own
            Suspense boundary
          </p>
          <p>
            <strong>Progressive Loading:</strong>
            {' '}
            Sections load as data becomes
            available
          </p>
          <p>
            <strong>Non-blocking:</strong>
            {' '}
            Slow sections don't prevent fast
            sections from displaying
          </p>
        </div>

        <div className="mt-6 p-4 bg-blue-50 rounded-md">
          <h3 className="font-semibold text-blue-800 mb-2">
            Expected Loading Order:
          </h3>
          <ol className="list-decimal list-inside space-y-1 text-blue-700 text-sm">
            <li>Dashboard Header (instant - no async data)</li>
            <li>Notifications (~800ms)</li>
            <li>Dashboard Stats (~1000ms)</li>
            <li>Recent Activity (~1500ms)</li>
            <li>Analytics (~2000ms)</li>
          </ol>
        </div>
      </div>
    </div>
  )
}
