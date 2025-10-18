export default function AboutPage() {
  return (
    <div className="bg-white rounded-xl p-12 shadow-2xl">
      <h1 className="text-4xl font-bold mb-4 text-blue-600">
        About This Example
      </h1>

      <p className="text-lg text-gray-600 mb-6 leading-relaxed">
        This example demonstrates the Rari app router, a server-first routing
        system inspired by Next.js App Router.
      </p>

      <h2 className="text-3xl font-bold mb-4 mt-8 text-cyan-600">
        Key Features
      </h2>

      <div className="grid grid-cols-[repeat(auto-fit,minmax(250px,1fr))] gap-6 mt-6">
        <div className="p-6 bg-gray-50 rounded-lg border-2 border-gray-200">
          <h3 className="text-blue-600 mb-2 font-semibold">Server Components</h3>
          <p className="text-gray-600 text-sm">
            React Server Components rendered on the server for better performance
          </p>
        </div>

        <div className="p-6 bg-gray-50 rounded-lg border-2 border-gray-200">
          <h3 className="text-blue-600 mb-2 font-semibold">Nested Layouts</h3>
          <p className="text-gray-600 text-sm">
            Share UI across routes with nested layout composition
          </p>
        </div>

        <div className="p-6 bg-gray-50 rounded-lg border-2 border-gray-200">
          <h3 className="text-blue-600 mb-2 font-semibold">File-Based Routing</h3>
          <p className="text-gray-600 text-sm">
            Automatic routing based on your file structure
          </p>
        </div>
      </div>
    </div>
  )
}

export const metadata = {
  title: 'About | Rari App Router',
  description: 'Learn about the Rari app router implementation',
}
