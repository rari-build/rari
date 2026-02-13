'use client'

import type { ReactNode } from 'react'
import { PackageManagerProvider } from '@/providers/PackageManagerProvider'
import { PostHogProvider } from '@/providers/PostHogProvider'

export function ClientProviders({ children }: { children: ReactNode }) {
  return (
    <PostHogProvider>
      <PackageManagerProvider>
        {children}
      </PackageManagerProvider>
    </PostHogProvider>
  )
}
