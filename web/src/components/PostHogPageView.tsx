'use client'

import type { PostHog } from 'posthog-js'
import { useEffect } from 'react'

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

export function PostHogPageView({
  pathname,
  posthog,
}: Readonly<{ pathname?: string; posthog: PostHog | null }>) {
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

  return null
}
