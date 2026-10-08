'use client'

import type { NavigationOptions } from './types'
import { createContext, use, useSyncExternalStore } from 'react'
import {
  getNavigationTransitionServerSnapshot,
  getNavigationTransitionSnapshot,
  subscribeNavigationTransition,
} from './navigation-transition-store'

export interface RouterContextValue {
  pathname: string
  searchParams: URLSearchParams
  push: (href: string, options?: NavigationOptions) => Promise<void>
  replace: (href: string, options?: NavigationOptions) => Promise<void>
  back: () => void
  forward: () => void
  refresh: () => void
  prefetch: (href: string) => Promise<void>
}

export const RouterContext = createContext<RouterContextValue | null>(null)

export function useRouter(): RouterContextValue {
  const context = use(RouterContext)

  if (!context) throw new Error('useRouter must be used within a RouterProvider')

  return context
}

export function usePathname(): string {
  const router = useRouter()
  return router.pathname
}

export function useSearchParams(): URLSearchParams {
  const router = useRouter()
  return router.searchParams
}

export function useViewTransitionKey(): number {
  return useSyncExternalStore(
    subscribeNavigationTransition,
    () => getNavigationTransitionSnapshot().generation,
    () => getNavigationTransitionServerSnapshot().generation,
  )
}
