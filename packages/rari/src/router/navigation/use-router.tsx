'use client'

import type { NavigationOptions } from './types'
import { createContext, use, useEffect, useMemo, useRef, useState } from 'react'
import { getCustomEventDetail, isRecord } from '@/shared/utils/type-guards'
import { getNavigate } from './navigate'

function isNavigateDetail(detail: unknown): detail is { to: string } {
  return isRecord(detail) && typeof detail.to === 'string'
}

function isRegisterNavigateDetail(
  detail: unknown,
): detail is { navigate: (href: string, options?: NavigationOptions) => Promise<void> } {
  return isRecord(detail) && typeof detail.navigate === 'function'
}

export interface RouterContextValue {
  pathname: string
  params: Record<string, string | string[]>
  searchParams: URLSearchParams
  push: (href: string, options?: NavigationOptions) => Promise<void>
  replace: (href: string, options?: NavigationOptions) => Promise<void>
  back: () => void
  forward: () => void
  refresh: () => void
  prefetch: (href: string) => Promise<void>
}

const RouterContext = createContext<RouterContextValue | null>(null)

export interface RouterProviderProps {
  readonly children: React.ReactNode
  readonly initialPathname: string
}

export function RouterProvider({ children, initialPathname }: RouterProviderProps) {
  const [pathname, setPathname] = useState(initialPathname)
  const [searchParams, setSearchParams] = useState(() =>
    typeof window !== 'undefined'
      ? new URLSearchParams(window.location.search)
      : new URLSearchParams(),
  )
  const navigateRef = useRef<((href: string, options?: NavigationOptions) => Promise<void>) | null>(
    null,
  )

  useEffect(() => {
    const handleNavigate = (event: Event) => {
      const detail = getCustomEventDetail(event, isNavigateDetail)
      if (detail) {
        setPathname(detail.to)
        setSearchParams(new URLSearchParams(window.location.search))
      }
    }
    window.addEventListener('rari:navigate', handleNavigate)

    return () => {
      window.removeEventListener('rari:navigate', handleNavigate)
    }
  }, [])

  useEffect(() => {
    const existingNavigate = getNavigate()
    if (existingNavigate) navigateRef.current = existingNavigate

    const handleRegisterNavigate = (event: Event) => {
      const detail = getCustomEventDetail(event, isRegisterNavigateDetail)
      if (detail) navigateRef.current = detail.navigate
    }

    const handleDeregisterNavigate = () => {
      navigateRef.current = null
    }

    window.addEventListener('rari:register-navigate', handleRegisterNavigate)
    window.addEventListener('rari:deregister-navigate', handleDeregisterNavigate)

    return () => {
      window.removeEventListener('rari:register-navigate', handleRegisterNavigate)
      window.removeEventListener('rari:deregister-navigate', handleDeregisterNavigate)
    }
  }, [])

  const value = useMemo<RouterContextValue>(
    () => ({
      pathname,
      params: {},
      searchParams,
      push: async (href: string, options?: NavigationOptions) => {
        if (navigateRef.current) {
          await navigateRef.current(href, options)
        } else {
          console.warn('[rari] Router not ready, falling back to window.location')
          window.location.href = href
        }
      },
      replace: async (href: string, options?: NavigationOptions) => {
        if (navigateRef.current) {
          await navigateRef.current(href, { ...options, replace: true })
        } else {
          console.warn('[rari] Router not ready, falling back to window.location')
          window.location.replace(href)
        }
      },
      back: () => {
        window.history.back()
      },
      forward: () => {
        window.history.forward()
      },
      refresh: () => {
        window.dispatchEvent(new CustomEvent('rari:app-router-rerender'))
      },
      prefetch: async (href: string) => {
        try {
          const url = new URL(href, window.location.origin)
          await fetch(url.pathname + url.search, {
            headers: { Accept: 'text/x-component' },
            priority: 'low',
          })
        } catch (error) {
          console.warn('[rari] Prefetch failed:', error)
        }
      },
    }),
    [pathname, searchParams],
  )

  return <RouterContext value={value}>{children}</RouterContext>
}

// eslint-disable-next-line react-refresh/only-export-components
export function useRouter(): RouterContextValue {
  const context = use(RouterContext)

  if (!context) throw new Error('useRouter must be used within a RouterProvider')

  return context
}

// eslint-disable-next-line react-refresh/only-export-components
export function usePathname(): string {
  const router = useRouter()
  return router.pathname
}

// eslint-disable-next-line react-refresh/only-export-components
export function useParams(): Record<string, string | string[]> {
  const router = useRouter()
  return router.params
}

// eslint-disable-next-line react-refresh/only-export-components
export function useSearchParams(): URLSearchParams {
  const router = useRouter()
  return router.searchParams
}
