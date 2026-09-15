'use client'

import { useEffect } from 'react'

export function SentryInit() {
  useEffect(() => {
    const dsn = import.meta.env.VITE_SENTRY_DSN
    if (typeof dsn !== 'string' || dsn === '') return undefined

    const loadSentry = () => {
      void import('@/lib/sentry-init')
      document.removeEventListener('click', loadSentry)
      document.removeEventListener('scroll', loadSentry)
      document.removeEventListener('keydown', loadSentry)
    }

    document.addEventListener('click', loadSentry, { once: true, passive: true })
    document.addEventListener('scroll', loadSentry, { once: true, passive: true })
    document.addEventListener('keydown', loadSentry, { once: true, passive: true })
    const timer = setTimeout(loadSentry, 5000)

    return () => {
      clearTimeout(timer)
      document.removeEventListener('click', loadSentry)
      document.removeEventListener('scroll', loadSentry)
      document.removeEventListener('keydown', loadSentry)
    }
  }, [])

  return null
}
