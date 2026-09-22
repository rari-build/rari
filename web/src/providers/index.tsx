'use client'

import type { ReactNode } from 'react'
import { PostHogInit } from '@/components/analytics/PostHogInit'
import { SentryInit } from '@/components/analytics/SentryInit'
import { PackageManagerProvider } from './PackageManagerProvider'
import { ThemeProvider } from './ThemeProvider'

export function Providers({
  children,
  pathname,
}: Readonly<{ children: ReactNode; pathname?: string }>) {
  return (
    <>
      <PostHogInit pathname={pathname} />
      <SentryInit />
      <ThemeProvider>
        <PackageManagerProvider>{children}</PackageManagerProvider>
      </ThemeProvider>
    </>
  )
}
