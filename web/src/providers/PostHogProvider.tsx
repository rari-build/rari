'use client'

import type { ReactNode } from 'react'
import { useEffect, useState } from 'react'
import { PostHogPageView } from '@/components/PostHogPageView'

export function PostHogProvider({ children, pathname }: { children: ReactNode, pathname?: string }) {
  const [client, setClient] = useState<any>(null)

  useEffect(() => {
    const key = import.meta.env.VITE_POSTHOG_KEY
    const host = import.meta.env.VITE_POSTHOG_HOST
    if (!key || !host)
      return

    const load = async () => {
      const { default: posthog } = await import('posthog-js')
      if (posthog.__loaded)
        return
      posthog.init(key, {
        api_host: host,
        person_profiles: 'always',
        capture_pageview: false,
        capture_pageleave: true,
        defaults: '2026-01-30',
        __preview_disable_beacon: true,
      })
      setClient(posthog)
    }

    document.addEventListener('click', load, { once: true, passive: true })
    document.addEventListener('scroll', load, { once: true, passive: true })
    document.addEventListener('keydown', load, { once: true, passive: true })
    const timer = setTimeout(load, 3000)

    return () => {
      clearTimeout(timer)
      document.removeEventListener('click', load)
      document.removeEventListener('scroll', load)
      document.removeEventListener('keydown', load)
    }
  }, [])

  if (!client)
    return <>{children}</>

  return (
    <>
      <PostHogPageView pathname={pathname} posthog={client} />
      {children}
    </>
  )
}
