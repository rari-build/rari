'use client'

import type { NavigationOptions } from './types'
import type { RouterContextValue } from './use-router'
import { useEffect, useMemo, useRef, useSyncExternalStore } from 'react'
import { getCustomEventDetail, isRecord } from '@/shared/utils/type-guards'
import { getNavigate } from './navigate'
import {
  getNavigationTransitionServerSnapshot,
  getNavigationTransitionSnapshot,
  subscribeNavigationTransition,
  syncNavigationTransitionFromWindow,
} from './navigation-transition-store'
import { RouterContext } from './use-router'

function isRegisterNavigateDetail(
  detail: unknown,
): detail is { navigate: (href: string, options?: NavigationOptions) => Promise<void> } {
  return isRecord(detail) && typeof detail.navigate === 'function'
}

export interface RouterProviderProps {
  readonly children: React.ReactNode
  readonly initialPathname: string
}

export function RouterProvider({ children, initialPathname }: RouterProviderProps) {
  const snapshot = useSyncExternalStore(
    subscribeNavigationTransition,
    getNavigationTransitionSnapshot,
    () => getNavigationTransitionServerSnapshot(initialPathname),
  )
  const navigateRef = useRef<((href: string, options?: NavigationOptions) => Promise<void>) | null>(
    null,
  )

  useEffect(() => {
    syncNavigationTransitionFromWindow()
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

  const searchParams = useMemo(
    () =>
      new URLSearchParams(
        snapshot.search.startsWith('?') ? snapshot.search.slice(1) : snapshot.search,
      ),
    [snapshot.search],
  )

  const value = useMemo<RouterContextValue>(
    () => ({
      pathname: snapshot.pathname,
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
    [snapshot.pathname, searchParams],
  )

  return <RouterContext value={value}>{children}</RouterContext>
}
