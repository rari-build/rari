'use client'

import type { ReactNode } from 'react'
import { PostHogProvider as PHProvider } from '@posthog/react'
import posthog from 'posthog-js'
import { useEffect } from 'react'

export function PostHogProvider({ children }: { children: ReactNode }) {
  useEffect(() => {
    const key = import.meta.env.VITE_POSTHOG_KEY
    const host = import.meta.env.VITE_POSTHOG_HOST

    if (key && host && !posthog.__loaded) {
      posthog.init(key, {
        api_host: host,
        person_profiles: 'always',
        capture_pageview: false,
        capture_pageleave: true,
        session_recording: {
          maskAllInputs: true,
          maskTextSelector: '*',
        },
      })
    }
  }, [])

  return <PHProvider client={posthog}>{children}</PHProvider>
}
