'use client'

import { useState } from 'react'

export default function NestedErrorChildClient() {
  const [shouldThrow, setShouldThrow] = useState(false)

  if (shouldThrow) {
    throw new Error('Error from nested child page')
  }

  return (
    <div data-testid="nested-error-child-page">
      <h1>Nested Child Page</h1>
      <button
        type="button"
        data-testid="trigger-nested-error-button"
        onClick={() => setShouldThrow(true)}
      >
        Trigger Nested Error
      </button>
    </div>
  )
}
