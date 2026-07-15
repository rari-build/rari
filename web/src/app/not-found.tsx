import { container } from '@/lib/styles'

export default function NotFound() {
  return (
    <div className={container.base}>
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="text-center">
          <h1 className="text-4xl font-bold mb-4 text-fg">404 - Page Not Found</h1>
          <p className="text-fg-muted mb-8">The page you're looking for doesn't exist.</p>
          <a
            href="/"
            className="inline-block bg-accent hover:bg-accent-hover text-accent-fg! px-6 py-3 rounded-lg font-semibold transition-colors no-underline! hover:no-underline!"
          >
            Go Home
          </a>
        </div>
      </div>
    </div>
  )
}
