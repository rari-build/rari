'use client'

import type { ReactNode } from 'react'
import { PostHogProvider as PHProvider } from '@posthog/react'
import posthog from 'posthog-js'
import { useEffect } from 'react'

export function PostHogProvider({ children }: { children: ReactNode }) {
  useEffect(() => {
    if (typeof window !== 'undefined') {
      const key = import.meta.env.VITE_POSTHOG_KEY
      const host = import.meta.env.VITE_POSTHOG_HOST

      if (key && host) {
        posthog.init(key, {
          api_host: host,
          person_profiles: 'identified_only',
          capture_pageview: false,
          capture_pageleave: true,
        })
      }
    }
  }, [])

  return <PHProvider client={posthog}>{children}</PHProvider>
}
