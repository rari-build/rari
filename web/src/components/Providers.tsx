'use client'

import type { ReactNode } from 'react'
import { PackageManagerProvider } from '@/providers/PackageManagerProvider'
import { PostHogProvider } from '@/providers/PostHogProvider'
import { ThemeProvider } from '@/providers/ThemeProvider'

export function Providers({
  children,
  pathname,
}: Readonly<{ children: ReactNode; pathname?: string }>) {
  return (
    <PostHogProvider pathname={pathname}>
      <ThemeProvider>
        <PackageManagerProvider>{children}</PackageManagerProvider>
      </ThemeProvider>
    </PostHogProvider>
  )
}
