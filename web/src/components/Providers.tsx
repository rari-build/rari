'use client'

import type { ReactNode } from 'react'
import { PackageManagerProvider } from '@/providers/PackageManagerProvider'
import { PostHogProvider } from '@/providers/PostHogProvider'

export function Providers({ children }: { children: ReactNode }) {
  return (
    <PostHogProvider>
      <PackageManagerProvider>
        {children}
      </PackageManagerProvider>
    </PostHogProvider>
  )
}
