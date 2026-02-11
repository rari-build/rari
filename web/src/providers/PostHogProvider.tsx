'use client'

import type { ReactNode } from 'react'
import { PostHogProvider as PHProvider } from '@posthog/react'
import posthog from 'posthog-js'
import { useEffect } from 'react'

export function PostHogProvider({ children }: { children: ReactNode }) {
  useEffect(() => {
    const key = import.meta.env.VITE_POSTHOG_KEY
    const host = import.meta.env.VITE_POSTHOG_HOST

    if (!key || !host)
      return

    const initOnInteraction = () => {
      if (!posthog.__loaded) {
        posthog.init(key, {
          api_host: host,
          person_profiles: 'always',
          capture_pageview: false,
          capture_pageleave: true,
        })
      }
      document.removeEventListener('click', initOnInteraction)
      document.removeEventListener('scroll', initOnInteraction)
      document.removeEventListener('keydown', initOnInteraction)
    }

    document.addEventListener('click', initOnInteraction, { once: true, passive: true })
    document.addEventListener('scroll', initOnInteraction, { once: true, passive: true })
    document.addEventListener('keydown', initOnInteraction, { once: true, passive: true })
    const timer = setTimeout(initOnInteraction, 3000)

    return () => {
      clearTimeout(timer)
      document.removeEventListener('click', initOnInteraction)
      document.removeEventListener('scroll', initOnInteraction)
      document.removeEventListener('keydown', initOnInteraction)
    }
  }, [])

  return <PHProvider client={posthog}>{children}</PHProvider>
}
