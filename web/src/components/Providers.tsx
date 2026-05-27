'use client'

import type { ReactNode } from 'react'
import { PackageManagerProvider } from '@/providers/PackageManagerProvider'
import { PostHogProvider } from '@/providers/PostHogProvider'

export function Providers({ children, pathname }: { children: ReactNode, pathname?: string }) {
  return (
    <PostHogProvider pathname={pathname}>
      <PackageManagerProvider>
        {children}
      </PackageManagerProvider>
    </PostHogProvider>
  )
}
