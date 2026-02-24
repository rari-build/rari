'use client'

import type { NavigationOptions } from './navigation-types'
import { createContext, use, useEffect, useMemo, useRef, useState } from 'react'
import { getNavigate } from './navigate'

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
  children: React.ReactNode
  initialPathname: string
}

export function RouterProvider({ children, initialPathname }: RouterProviderProps) {
  const [pathname, setPathname] = useState(initialPathname)
  const [searchParams, setSearchParams] = useState(() =>
    typeof window !== 'undefined'
      ? new URLSearchParams(window.location.search)
      : new URLSearchParams(),
  )
  const [params, setParams] = useState<Record<string, string | string[]>>({})
  const navigateRef = useRef<((href: string, options?: NavigationOptions) => Promise<void>) | null>(null)

  useEffect(() => {
    const handleNavigate = (event: Event) => {
      const customEvent = event as CustomEvent
      const detail = customEvent.detail

      if (detail?.to) {
        setPathname(detail.to)
        setSearchParams(new URLSearchParams(window.location.search))

        if (detail?.routeInfo?.extractedParams) {
          setParams(detail.routeInfo.extractedParams)
        }
        else {
          setParams({})
        }
      }
    }
    window.addEventListener('rari:navigate', handleNavigate)

    return () => {
      window.removeEventListener('rari:navigate', handleNavigate)
    }
  }, [])

  useEffect(() => {
    const existingNavigate = getNavigate()
    if (existingNavigate)
      navigateRef.current = existingNavigate

    const handleRegisterNavigate = (event: Event) => {
      const customEvent = event as CustomEvent<{ navigate: (href: string, options?: NavigationOptions) => Promise<void> }>
      navigateRef.current = customEvent.detail.navigate
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

  const value = useMemo<RouterContextValue>(() => ({
    pathname,
    params,
    searchParams,
    push: async (href: string, options?: NavigationOptions) => {
      if (navigateRef.current) {
        await navigateRef.current(href, options)
      }
      else {
        console.warn('[rari] Router not ready, falling back to window.location')
        window.location.href = href
      }
    },
    replace: async (href: string, options?: NavigationOptions) => {
      if (navigateRef.current) {
        await navigateRef.current(href, { ...options, replace: true })
      }
      else {
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
        } as RequestInit)
      }
      catch (error) {
        console.warn('[rari] Prefetch failed:', error)
      }
    },
  }), [pathname, params, searchParams])

  return (
    <RouterContext value={value}>
      {children}
    </RouterContext>
  )
}

export function useRouter(): RouterContextValue {
  const context = use(RouterContext)

  if (!context)
    throw new Error('useRouter must be used within a RouterProvider')

  return context
}

export function usePathname(): string {
  const router = useRouter()
  return router.pathname
}

export function useParams(): Record<string, string | string[]> {
  const router = useRouter()
  return router.params
}

export function useSearchParams(): URLSearchParams {
  const router = useRouter()
  return router.searchParams
}
