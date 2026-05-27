'use client'

import { useEffect } from 'react'

export function PostHogPageView({ pathname, posthog }: { pathname?: string, posthog: any }) {
  useEffect(() => {
    if (pathname && posthog) {
      posthog.capture('$pageview', { $current_url: window.origin + pathname })
    }
  }, [pathname, posthog])

  useEffect(() => {
    if (!posthog)
      return
    const handleNavigate = (event: Event) => {
      const customEvent = event as CustomEvent<{ to: string }>
      if (customEvent.detail?.to) {
        posthog.capture('$pageview', { $current_url: window.origin + customEvent.detail.to })
      }
    }
    window.addEventListener('rari:navigate', handleNavigate)
    return () => window.removeEventListener('rari:navigate', handleNavigate)
  }, [posthog])

  return null
}
