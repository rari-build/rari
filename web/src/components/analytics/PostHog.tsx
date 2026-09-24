'use client'

import type { PostHog as PostHogClient } from 'posthog-js'
import { useEffect, useState } from 'react'
import { useIdleLoad } from '@/lib/hooks/use-idle-load'

function isRariNavigateEvent(event: Event): event is CustomEvent<{ to: string }> {
  if (!(event instanceof CustomEvent)) return false

  const detail: unknown = event.detail
  return (
    typeof detail === 'object' &&
    detail !== null &&
    'to' in detail &&
    typeof detail.to === 'string' &&
    detail.to !== ''
  )
}

async function loadAndInitPostHog(key: string, host: string): Promise<PostHogClient> {
  const { default: posthog } = await import('posthog-js')
  if (!posthog.__loaded) {
    posthog.init(key, {
      api_host: host,
      person_profiles: 'always',
      capture_pageview: false,
      capture_pageleave: true,
      defaults: '2026-01-30',
      disable_beacon: true,
    })
  }
  return posthog
}

function usePostHogPageViews(pathname: string | undefined, posthog: PostHogClient | null) {
  useEffect(() => {
    if (pathname != null && pathname !== '' && posthog != null) {
      posthog.capture('$pageview', { $current_url: window.origin + pathname })
    }
  }, [pathname, posthog])

  useEffect(() => {
    if (!posthog) return undefined

    const handleNavigate = (event: Event) => {
      if (isRariNavigateEvent(event)) {
        posthog.capture('$pageview', { $current_url: window.origin + event.detail.to })
      }
    }
    window.addEventListener('rari:navigate', handleNavigate)
    return () => {
      window.removeEventListener('rari:navigate', handleNavigate)
    }
  }, [posthog])
}

export function PostHog({ pathname }: Readonly<{ pathname?: string }>) {
  const [client, setClient] = useState<PostHogClient | null>(null)

  useIdleLoad(() => {
    const key = import.meta.env.VITE_POSTHOG_KEY
    const host = import.meta.env.VITE_POSTHOG_HOST
    if (key == null || key === '' || host == null || host === '') return

    void loadAndInitPostHog(key, host)
      .then(setClient)
      .catch((error: unknown) => {
        console.warn('[PostHog] Failed to initialize:', error)
      })
  }, 3000)

  usePostHogPageViews(pathname, client)

  return null
}
