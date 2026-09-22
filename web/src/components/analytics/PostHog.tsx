'use client'

import type { PostHog as PostHogClient } from 'posthog-js'
import { useEffect, useState } from 'react'

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

  useEffect(() => {
    const key = import.meta.env.VITE_POSTHOG_KEY
    const host = import.meta.env.VITE_POSTHOG_HOST
    if (key == null || key === '' || host == null || host === '') return undefined

    const load = async () => {
      const { default: posthog } = await import('posthog-js')
      if (posthog.__loaded) return
      posthog.init(key, {
        api_host: host,
        person_profiles: 'always',
        capture_pageview: false,
        capture_pageleave: true,
        defaults: '2026-01-30',
        disable_beacon: true,
      })
      setClient(posthog)
    }

    const onInteraction = () => {
      void load()
    }

    document.addEventListener('click', onInteraction, { once: true, passive: true })
    document.addEventListener('scroll', onInteraction, { once: true, passive: true })
    document.addEventListener('keydown', onInteraction, { once: true, passive: true })
    const timer = setTimeout(onInteraction, 3000)

    return () => {
      clearTimeout(timer)
      document.removeEventListener('click', onInteraction)
      document.removeEventListener('scroll', onInteraction)
      document.removeEventListener('keydown', onInteraction)
    }
  }, [])

  usePostHogPageViews(pathname, client)

  return null
}
