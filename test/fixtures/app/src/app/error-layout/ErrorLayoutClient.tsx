'use client'

import { useState } from 'react'

export default function ErrorLayoutClient({
  children,
}: {
  children: React.ReactNode
}) {
  const [shouldThrow, setShouldThrow] = useState(false)

  if (shouldThrow) {
    throw new Error('Test error from layout')
  }

  return (
    <div data-testid="error-layout">
      <div data-testid="layout-header">
        <h2>Layout with Error Test</h2>
        <button
          type="button"
          data-testid="trigger-layout-error-button"
          onClick={() => setShouldThrow(true)}
        >
          Trigger Layout Error
        </button>
      </div>
      {children}
    </div>
  )
}
