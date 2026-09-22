'use client'

import type { ReactNode } from 'react'
import { PostHog } from '@/components/analytics/PostHog'
import { Sentry } from '@/components/analytics/Sentry'
import { PackageManagerProvider } from './PackageManagerProvider'
import { ThemeProvider } from './ThemeProvider'

export function Providers({
  children,
  pathname,
}: Readonly<{ children: ReactNode; pathname?: string }>) {
  return (
    <>
      <PostHog pathname={pathname} />
      <Sentry />
      <ThemeProvider>
        <PackageManagerProvider>{children}</PackageManagerProvider>
      </ThemeProvider>
    </>
  )
}
