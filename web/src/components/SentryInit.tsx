'use client'

import { useEffect } from 'react'

export function SentryInit() {
  useEffect(() => {
    const dsn = import.meta.env.VITE_SENTRY_DSN
    if (typeof dsn !== 'string' || dsn === '') return undefined

    let timer: ReturnType<typeof setTimeout> | undefined

    const loadSentry = () => {
      if (timer != null) {
        clearTimeout(timer)
        timer = undefined
      }
      void import('@/lib/sentry-init')
      document.removeEventListener('click', loadSentry)
      document.removeEventListener('scroll', loadSentry)
      document.removeEventListener('keydown', loadSentry)
    }

    document.addEventListener('click', loadSentry, { once: true, passive: true })
    document.addEventListener('scroll', loadSentry, { once: true, passive: true })
    document.addEventListener('keydown', loadSentry, { once: true, passive: true })
    timer = setTimeout(loadSentry, 5000)

    return () => {
      if (timer != null) clearTimeout(timer)
      document.removeEventListener('click', loadSentry)
      document.removeEventListener('scroll', loadSentry)
      document.removeEventListener('keydown', loadSentry)
    }
  }, [])

  return null
}
