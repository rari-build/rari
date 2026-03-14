'use client'

export default function ErrorBoundary({
  error,
  reset,
}: {
  error: Error
  reset: () => void
}) {
  return (
    <div data-testid="error-boundary">
      <h2>Something went wrong!</h2>
      <p data-testid="error-message">{error.message}</p>
      <button type="button" data-testid="reset-button" onClick={reset}>
        Try again
      </button>
    </div>
  )
}
