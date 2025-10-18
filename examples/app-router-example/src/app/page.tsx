import type { PageProps } from 'rari/client'

export default function HomePage({ params, searchParams }: PageProps) {
  return (
    <div className="bg-white rounded-xl p-12 shadow-2xl">
      <h1 className="text-5xl font-bold mb-4 bg-gradient-to-r from-blue-600 to-cyan-600 bg-clip-text text-transparent">
        Welcome to Rari App Router! 🚀
      </h1>

      <p className="text-xl text-gray-600 mb-8 leading-relaxed">
        This is a demonstration of the new Next.js-style app router implementation
        in Rari. Features include:
      </p>

      <ul className="text-lg text-gray-700 leading-loose ml-8 mb-8 space-y-1">
        <li>✅ File-based routing with app directory</li>
        <li>✅ Nested layouts with composition</li>
        <li>✅ Server-side rendering (SSR)</li>
        <li>✅ Loading states and error boundaries</li>
        <li>✅ Dynamic routes with params</li>
        <li>✅ Server-side data fetching</li>
        <li>✅ React Server Actions</li>
        <li>✅ Metadata generation for SEO</li>
      </ul>

      <div className="bg-blue-50 p-6 rounded-lg border-2 border-blue-200 mb-8">
        <h3 className="mb-4 text-blue-700 text-2xl font-semibold">
          🎯 Try the Examples:
        </h3>
        <div className="grid gap-4">
          <a
            href="/actions"
            className="block p-4 bg-white rounded-md no-underline text-blue-600 font-semibold border-2 border-blue-600 hover:bg-blue-50 transition-all"
          >
            → Server Actions Demo - Interactive todo app with useActionState
          </a>
          <a
            href="/server-data"
            className="block p-4 bg-white rounded-md no-underline text-blue-600 font-semibold border-2 border-blue-600 hover:bg-blue-50 transition-all"
          >
            → Server Data Fetching - Async server components
          </a>
          <a
            href="/interactive"
            className="block p-4 bg-white rounded-md no-underline text-blue-600 font-semibold border-2 border-blue-600 hover:bg-blue-50 transition-all"
          >
            → Interactive Components - Client components with RSC
          </a>
        </div>
      </div>

      <div className="bg-gray-50 p-6 rounded-lg border border-gray-200">
        <h3 className="mb-2 text-blue-600 font-semibold">Debug Info:</h3>
        <pre className="text-sm text-gray-700">
          {JSON.stringify({ params, searchParams }, null, 2)}
        </pre>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'Home | Rari App Router',
  description: 'Welcome to the Rari app router example',
}
