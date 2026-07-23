'use client'

import type { PostHog } from 'posthog-js'
import type { ReactNode } from 'react'
import { useEffect, useState } from 'react'
import { PostHogPageView } from '@/components/PostHogPageView'

export function PostHogProvider({
  children,
  pathname,
}: Readonly<{ children: ReactNode; pathname?: string }>) {
  const [client, setClient] = useState<PostHog | null>(null)

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
        __preview_disable_beacon: true,
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

  return (
    <>
      {children}
      {client ? <PostHogPageView pathname={pathname} posthog={client} /> : null}
    </>
  )
}
