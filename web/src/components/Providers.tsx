'use client'

import type { ReactNode } from 'react'
import { PostHogInit } from '@/components/PostHogInit'
import { SentryInit } from '@/components/SentryInit'
import { PackageManagerProvider } from '@/providers/PackageManagerProvider'
import { ThemeProvider } from '@/providers/ThemeProvider'

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
