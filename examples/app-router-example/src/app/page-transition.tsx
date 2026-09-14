'use client'

import type { ReactNode } from 'react'
import { useEffect, ViewTransition } from 'react'

export function PageTransition({ children }: { readonly children: ReactNode }): ReactNode {
  useEffect(() => {
    const pending = () => {
      document.documentElement.classList.add('rari-nav-pending')
    }
    const clear = () => {
      document.documentElement.classList.remove('rari-nav-pending')
    }

    window.addEventListener('rari:navigation-start', pending)
    window.addEventListener('rari:navigate-committed', clear)
    window.addEventListener('rari:navigate-error', clear)

    return () => {
      window.removeEventListener('rari:navigation-start', pending)
      window.removeEventListener('rari:navigate-committed', clear)
      window.removeEventListener('rari:navigate-error', clear)
      clear()
    }
  }, [])

  return (
    <ViewTransition
      default="none"
      update={{
        'nav-forward': 'rari-page-vt',
        'nav-traverse': 'rari-page-vt',
        'nav-replace': 'rari-page-vt',
        'default': 'none',
      }}
      enter={{
        'nav-forward': 'rari-page-vt',
        'nav-traverse': 'rari-page-vt',
        'nav-replace': 'rari-page-vt',
        'default': 'none',
      }}
      exit={{
        'nav-forward': 'rari-page-vt',
        'nav-traverse': 'rari-page-vt',
        'nav-replace': 'rari-page-vt',
        'default': 'none',
      }}
    >
      <div className="rari-page-shell">{children}</div>
    </ViewTransition>
  )
}
