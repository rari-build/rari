'use client'

export default function LayoutErrorBoundary({
  error,
  reset,
}: {
  error: Error
  reset: () => void
}) {
  return (
    <div data-testid="layout-error-boundary">
      <h2>Layout Error Caught!</h2>
      <p data-testid="layout-error-message">{error.message}</p>
      <button data-testid="layout-reset-button" onClick={reset}>
        Reset Layout
      </button>
    </div>
  )
}
