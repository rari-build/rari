import type { Metadata } from 'rari'

export default async function AboutPage() {
  return (
    <div>
      <h1>About Page</h1>
      <p>This is the about page for testing navigation.</p>

      {/* Add enough content to make the page scrollable */}
      <div className="space-y-4 mt-8">
        {Array.from({ length: 50 }, (_, i) => (
          <p key={i} className="text-gray-600">
            This is paragraph
            {' '}
            {i + 1}
            . Adding content to make the page scrollable for testing scroll position restoration.
          </p>
        ))}
      </div>

      <a href="/">Back to Home</a>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'About',
  description: 'About page',
}
