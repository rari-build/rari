'use client'

import type { ReactNode } from 'react'
import { ViewTransition } from 'react'

const navEnterExit = {
  'nav-forward': 'rari-page-vt',
  'nav-traverse': 'rari-page-vt',
  'nav-replace': 'rari-page-vt',
  'default': 'none',
} as const

export function LoadingReveal({ children }: { readonly children: ReactNode }): ReactNode {
  return (
    <ViewTransition exit="rari-reveal-exit" default="none">
      {children}
    </ViewTransition>
  )
}

export function PageTransition({ children }: { readonly children: ReactNode }): ReactNode {
  return (
    <ViewTransition default="none" enter={navEnterExit} exit={navEnterExit}>
      <ViewTransition enter="rari-reveal-enter" default="none">
        <div className="rari-page-shell">{children}</div>
      </ViewTransition>
    </ViewTransition>
  )
}
