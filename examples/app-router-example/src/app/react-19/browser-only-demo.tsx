'use client'

import { Suspense, use } from 'react'
import { browser } from 'react-dom'

function BrowserTimeZone() {
  use(browser())
  const timeZone = new Intl.DateTimeFormat().resolvedOptions().timeZone
  return <p className="text-lg font-medium text-gray-900">{timeZone}</p>
}

export function BrowserOnlyDemo() {
  return (
    <div className="rounded-lg border border-amber-200 bg-amber-50 p-4">
      <p className="text-sm text-amber-900 mb-2">
        SSR shows the Suspense fallback; after hydration, <code>use(browser())</code> resolves and
        the local timezone renders.
      </p>
      <Suspense fallback={<p className="text-amber-800">Detecting timezone…</p>}>
        <BrowserTimeZone />
      </Suspense>
    </div>
  )
}
