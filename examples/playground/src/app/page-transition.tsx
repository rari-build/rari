'use client'

import type { ReactNode } from 'react'
import { useViewTransitionKey } from 'rari/router'
import { ViewTransition } from 'react'

const navEnterExit = {
  'nav-forward': 'rari-page-vt',
  'nav-traverse': 'rari-page-vt',
  'nav-replace': 'rari-page-vt',
  'default': 'none',
} as const

const loadingExit = {
  'nav-forward': 'none',
  'nav-traverse': 'none',
  'nav-replace': 'none',
  'default': 'rari-reveal-exit',
} as const

export function LoadingReveal({ children }: { readonly children: ReactNode }): ReactNode {
  return (
    <ViewTransition exit={loadingExit} default="none">
      {children}
    </ViewTransition>
  )
}

export function PageTransition({ children }: { readonly children: ReactNode }): ReactNode {
  const transitionKey = useViewTransitionKey()

  return (
    <ViewTransition
      key={transitionKey}
      name="rari-page"
      default="none"
      enter={navEnterExit}
      exit={navEnterExit}
      share={navEnterExit}
    >
      <ViewTransition enter="rari-reveal-enter" default="none">
        <div className="rari-page-shell">{children}</div>
      </ViewTransition>
    </ViewTransition>
  )
}
