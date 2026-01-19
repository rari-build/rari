'use client'

import { usePostHog } from '@posthog/react'
import { useEffect } from 'react'

export function PostHogPageView({ pathname }: { pathname?: string }) {
  const posthog = usePostHog()

  useEffect(() => {
    if (pathname && posthog) {
      const url = window.origin + pathname
      posthog.capture('$pageview', {
        $current_url: url,
      })
    }
  }, [pathname, posthog])

  useEffect(() => {
    if (!posthog)
      return

    const handleNavigate = (event: Event) => {
      const customEvent = event as CustomEvent<{ to: string }>
      if (customEvent.detail?.to) {
        const url = window.origin + customEvent.detail.to
        posthog.capture('$pageview', {
          $current_url: url,
        })
      }
    }

    window.addEventListener('rari:navigate', handleNavigate)
    return () => window.removeEventListener('rari:navigate', handleNavigate)
  }, [posthog])

  return null
}
