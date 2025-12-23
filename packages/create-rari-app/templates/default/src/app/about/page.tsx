import type { PageProps } from 'rari/client'

export default function AboutPage(_params: PageProps) {
  return (
    <div className="space-y-6">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <h1 className="text-4xl font-bold text-gray-900 mb-4">About</h1>
        <p className="text-lg text-gray-600 mb-4">
          This is a Rari application using the app router.
        </p>
        <p className="text-gray-600">
          Rari is a performance-first React framework powered by Rust, featuring:
        </p>
        <ul className="list-disc list-inside text-gray-600 mt-4 space-y-2">
          <li>React Server Components</li>
          <li>File-based routing</li>
          <li>Server Actions</li>
          <li>Zero-config setup</li>
          <li>Fast development experience</li>
        </ul>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'About | {{PROJECT_NAME}}',
  description: 'Learn more about this Rari application',
}
