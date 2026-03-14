'use client'

export default function NestedErrorBoundary({
  error,
  reset,
}: {
  error: Error
  reset: () => void
}) {
  return (
    <div data-testid="nested-error-boundary">
      <h2>Nested Error Caught!</h2>
      <p data-testid="nested-error-message">{error.message}</p>
      <button type="button" data-testid="nested-reset-button" onClick={reset}>
        Reset Nested
      </button>
    </div>
  )
}
