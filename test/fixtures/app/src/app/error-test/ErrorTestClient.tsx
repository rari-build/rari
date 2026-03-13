'use client'

import { useState } from 'react'

export default function ErrorTestClient() {
  const [shouldThrow, setShouldThrow] = useState(false)

  if (shouldThrow) {
    throw new Error('Test error from component')
  }

  return (
    <div data-testid="error-test-page">
      <h1>Error Test Page</h1>
      <p>This page can trigger errors for testing error boundaries.</p>
      <button
        type="button"
        data-testid="trigger-error-button"
        onClick={() => setShouldThrow(true)}
      >
        Trigger Error
      </button>
    </div>
  )
}
