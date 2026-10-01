'use client'

import type { ReactNode } from 'react'
import { ViewTransition } from 'react'

const navEnterExit = {
  'nav-forward': 'rari-page-vt',
  'nav-traverse': 'rari-page-vt',
  'nav-replace': 'rari-page-vt',
  'default': 'rari-page-vt',
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
  return (
    <ViewTransition
      name="rari-page"
      default="rari-page-vt"
      enter={navEnterExit}
      exit={navEnterExit}
      share={navEnterExit}
      update={navEnterExit}
    >
      <ViewTransition enter="rari-reveal-enter" default="none">
        <div className="rari-page-shell">{children}</div>
      </ViewTransition>
    </ViewTransition>
  )
}
