export default function NotFound() {
  return (
    <div className="bg-white rounded-xl p-12 shadow-2xl text-center">
      <h1 className="text-6xl text-blue-600 mb-4 font-bold">
        404
      </h1>
      <h2 className="text-3xl text-cyan-600 mb-4 font-semibold">
        Page Not Found
      </h2>
      <p className="text-lg text-gray-600 mb-8">
        The page you're looking for doesn't exist.
      </p>
      <a
        href="/"
        className="inline-block px-6 py-3 bg-gradient-to-r from-blue-600 to-cyan-600 text-white no-underline rounded-lg font-semibold hover:from-blue-700 hover:to-cyan-700 transition-all"
      >
        Go Home
      </a>
    </div>
  )
}
