export default function NotFound() {
  return (
    <div className="bg-white rounded-xl p-12 shadow-2xl text-center">
      <h1 className="text-6xl  mb-4 font-bold">
        404
      </h1>
      <h2 className="text-3xl mb-4 font-semibold">
        Page Not Found
      </h2>
      <p className="text-lg text-gray-600 mb-8">
        The page you're looking for doesn't exist.
      </p>
      <a
        href="/"
        className="inline-block px-6 py-3 text-white no-underline rounded-lg font-semibold transition-all bg-indigo-600 hover:bg-indigo-700 active:bg-indigo-800 shadow-sm"
      >
        Go Home
      </a>
    </div>
  )
}
