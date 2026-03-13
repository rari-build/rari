export default function NestedErrorPage() {
  return (
    <div data-testid="nested-error-page">
      <h1>Nested Error Test</h1>
      <p>This page tests error propagation through nested routes.</p>
      <a href="/nested-error/child">Go to Child Page</a>
    </div>
  )
}
