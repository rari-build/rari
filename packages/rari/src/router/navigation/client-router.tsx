'use client'

import type * as React from 'react'
import type { NavigationError } from './error-handler'
import type { NavigationOptions } from './types'
import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { serializeRouterState } from '@/runtime/flight/serialize-router-state'
import { normalizePath } from '@/shared/utils/path'
import { getCustomEventDetail, isError, isHistoryState, isRecord } from '@/shared/utils/type-guards'
import { debounce } from './debounce'
import { NavigationErrorHandler } from './error-handler'
import { extractPathname, isExternalUrl } from './match'
import { deregisterNavigate, registerNavigate } from './navigate'
import { StatePreserver } from './state-preserver'

function isNavigateCommittedDetail(detail: unknown): detail is { readonly navigationId: number } {
  return isRecord(detail) && typeof detail.navigationId === 'number'
}

function isNavigateErrorDetail(
  detail: unknown,
): detail is { readonly navigationId: number; readonly error: unknown } {
  return isRecord(detail) && typeof detail.navigationId === 'number' && 'error' in detail
}

async function waitForNavigationSettlement(
  navigationId: number,
  signal: AbortSignal,
): Promise<'committed' | { readonly error: unknown }> {
  if (signal.aborted) {
    return { error: new DOMException('Aborted', 'AbortError') }
  }

  return new Promise(resolve => {
    function cleanup() {
      window.removeEventListener('rari:navigate-committed', onCommitted)
      window.removeEventListener('rari:navigate-error', onError)
      signal.removeEventListener('abort', onAbort)
    }

    function onCommitted(event: Event) {
      const detail = getCustomEventDetail(event, isNavigateCommittedDetail)
      if (detail?.navigationId !== navigationId) return
      cleanup()
      resolve('committed')
    }

    function onError(event: Event) {
      const detail = getCustomEventDetail(event, isNavigateErrorDetail)
      if (detail?.navigationId !== navigationId) return
      cleanup()
      resolve({ error: detail.error })
    }

    function onAbort() {
      cleanup()
      resolve({ error: new DOMException('Aborted', 'AbortError') })
    }

    window.addEventListener('rari:navigate-committed', onCommitted)
    window.addEventListener('rari:navigate-error', onError)
    signal.addEventListener('abort', onAbort, { once: true })
  })
}

export interface ClientRouterProps {
  readonly children: React.ReactNode
  readonly initialRoute: string
}

interface NavigationState {
  currentRoute: string
  navigationId: number
  error: NavigationError | null
}

interface PendingNavigation {
  targetPath: string
  navigationId: number
  promise: Promise<void>
  abortController: AbortController
}

interface HistoryState {
  route: string
  navigationId: number
  scrollPosition?: { x: number; y: number }
  timestamp: number
  key: string
}

export function ClientRouter({ children, initialRoute }: ClientRouterProps): React.ReactNode {
  const [navigationState, setNavigationState] = useState<NavigationState>(() => ({
    currentRoute:
      typeof window !== 'undefined'
        ? `${window.location.pathname}${window.location.search}`
        : normalizePath(initialRoute),
    navigationId: 0,
    error: null,
  }))

  const abortControllerRef = useRef<AbortController | null>(null)
  const isMountedRef = useRef(true)
  const currentRouteRef = useRef<string>(
    typeof window !== 'undefined'
      ? `${window.location.pathname}${window.location.search}`
      : normalizePath(initialRoute),
  )
  const committedUrlRef = useRef<string>(
    typeof window !== 'undefined'
      ? `${window.location.pathname}${window.location.search}${window.location.hash}`
      : normalizePath(initialRoute),
  )
  const navigationIdCounterRef = useRef<number>(0)

  const errorHandlerRef = useRef<NavigationErrorHandler>(
    new NavigationErrorHandler({
      onError: error => {
        console.error('[rari] Router: Navigation error:', error)
      },
    }),
  )

  const pendingNavigationsRef = useRef<Map<string, PendingNavigation>>(new Map())

  const statePreserverRef = useRef<StatePreserver>(
    new StatePreserver({
      maxHistorySize: 50,
    }),
  )

  const NAVIGATION_DEBOUNCE_MS = 50
  const NAVIGATION_MAX_WAIT_MS = 200

  const generateHistoryKey = (): string => {
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
      return crypto.randomUUID()
    }

    return `hk-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 11)}`
  }

  const cancelNavigation = () => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort()
      abortControllerRef.current = null
    }
  }

  const cancelAllPendingNavigations = () => {
    for (const [, pending] of pendingNavigationsRef.current.entries())
      pending.abortController.abort()

    pendingNavigationsRef.current.clear()
  }

  const cleanupAbortedNavigation = (pendingPath: string, navigationId: number) => {
    pendingNavigationsRef.current.delete(pendingPath)

    if (isMountedRef.current && navigationState.navigationId === navigationId) {
      setNavigationState(prev => ({
        ...prev,
      }))
    }
  }

  const handleSameRouteNavigation = (
    hash: string,
    options: Readonly<{ replace?: boolean }> = {},
  ) => {
    const pathAndSearch = `${window.location.pathname}${window.location.search}`
    const nextUrl = hash ? `${pathAndSearch}#${hash}` : pathAndSearch

    if (hash) {
      const element = document.getElementById(hash)
      if (element) element.scrollIntoView({ behavior: 'smooth', block: 'start' })
    }

    const currentUrl = `${window.location.pathname}${window.location.search}${window.location.hash}`
    if (nextUrl !== currentUrl) {
      if (options.replace) {
        window.history.replaceState(window.history.state, '', nextUrl)
      } else {
        window.history.pushState(window.history.state, '', nextUrl)
      }
    }

    committedUrlRef.current = nextUrl
  }

  const handleScrollAfterNavigation = (
    actualTargetPath: string,
    hash: string,
    options: NavigationOptions,
  ) => {
    if (options.historyKey != null && options.historyKey !== '') {
      requestAnimationFrame(() => {
        statePreserverRef.current.restoreState(actualTargetPath)
      })
    } else if (hash) {
      requestAnimationFrame(() => {
        const scrollToHash = (attempts = 0) => {
          const element = document.getElementById(hash)
          if (element) element.scrollIntoView({ behavior: 'smooth', block: 'start' })
          else if (attempts < 10) setTimeout(scrollToHash, 50, attempts + 1)
        }
        scrollToHash()
      })
    }
  }

  const completeNavigation = (
    actualTargetPath: string,
    hash: string,
    options: NavigationOptions,
    navigationId: number,
    settledUrl: string,
  ) => {
    if (!isMountedRef.current) return

    if (navigationIdCounterRef.current !== navigationId) return

    currentRouteRef.current = actualTargetPath
    committedUrlRef.current = settledUrl

    setNavigationState(prev => ({
      ...prev,
      currentRoute: actualTargetPath,
      navigationId,
      error: null,
    }))

    handleScrollAfterNavigation(actualTargetPath, hash, options)
  }

  const handleNavigationError = (
    error: unknown,
    targetPath: string,
    navigationId: number,
    fromRoute: string,
    options: { readonly emitEvent?: boolean } = {},
  ) => {
    if (isError(error) && error.name === 'AbortError') {
      cleanupAbortedNavigation(targetPath, navigationId)
      return
    }

    const navError = errorHandlerRef.current.handleError(error, targetPath)

    if (isMountedRef.current) {
      setNavigationState(prev => ({
        ...prev,
        error: navError,
      }))
      window.history.replaceState(window.history.state, '', fromRoute)
    }

    pendingNavigationsRef.current.delete(targetPath)

    if (options.emitEvent !== false) {
      window.dispatchEvent(
        new CustomEvent('rari:navigate-error', {
          detail: {
            from: fromRoute,
            to: targetPath,
            error: navError,
            navigationId,
          },
        }),
      )
    }
  }

  const navigate = async (href: string, options: NavigationOptions = {}) => {
    if (!href || typeof href !== 'string') return

    const [pathWithoutHash, hash] = href.includes('#') ? href.split('#') : [href, '']
    const targetPath = normalizePath(pathWithoutHash)

    if (targetPath === currentRouteRef.current) {
      handleSameRouteNavigation(hash, { replace: options.replace === true })
      return
    }

    const existingPending = pendingNavigationsRef.current.get(targetPath)
    if (existingPending) return existingPending.promise

    cancelAllPendingNavigations()
    cancelNavigation()

    const abortController = new AbortController()
    abortControllerRef.current = abortController

    navigationIdCounterRef.current += 1
    const navigationId = navigationIdCounterRef.current

    window.dispatchEvent(
      new CustomEvent('rari:navigation-start', {
        detail: { navigationId, targetPath },
      }),
    )

    const navigationPromise = (async () => {
      const fromRoute = committedUrlRef.current
      try {
        if (options.historyKey == null || options.historyKey === '')
          statePreserverRef.current.captureState(currentRouteRef.current)

        const historyKey =
          options.historyKey != null && options.historyKey !== ''
            ? options.historyKey
            : generateHistoryKey()

        const fetchUrl = window.location.origin + targetPath

        const historyState: HistoryState = {
          route: targetPath,
          navigationId,
          scrollPosition: { x: window.scrollX, y: window.scrollY },
          timestamp: Date.now(),
          key: historyKey,
        }

        const response = await fetch(fetchUrl, {
          headers: {
            'Accept': 'text/x-component',
            'rari-navigation-id': String(navigationId),
            'rari-router-state': serializeRouterState(),
          },
          cache: 'no-store',
          signal: abortController.signal,
        })

        if (!response.ok && response.status !== 404) {
          handleNavigationError(
            new Error(`Failed to fetch: ${response.status}`),
            targetPath,
            navigationId,
            fromRoute,
          )
          return
        }

        if (abortController.signal.aborted) {
          cleanupAbortedNavigation(targetPath, navigationId)
          return
        }

        const finalUrl = new URL(response.url)
        const pathname = finalUrl.pathname
        const routeIdentity = `${finalUrl.pathname}${finalUrl.search}`
        const settledHistoryState: HistoryState = {
          ...historyState,
          route: routeIdentity,
        }
        const settledUrl = `${routeIdentity}${hash ? `#${hash}` : ''}`

        if (navigationIdCounterRef.current !== navigationId) return

        const settlement = waitForNavigationSettlement(navigationId, abortController.signal)

        window.dispatchEvent(
          new CustomEvent('rari:navigate', {
            detail: {
              from: fromRoute,
              to: pathname,
              navigationId,
              options,
              abortSignal: abortController.signal,
              rscResponse: response,
              pendingHistory: {
                url: settledUrl,
                state: settledHistoryState,
                replace: options.replace === true,
              },
            },
          }),
        )

        if (navigationIdCounterRef.current !== navigationId) {
          abortController.abort()
          return
        }

        const settlementResult = await settlement
        if (navigationIdCounterRef.current !== navigationId) return

        if (settlementResult !== 'committed') {
          handleNavigationError(settlementResult.error, targetPath, navigationId, fromRoute, {
            emitEvent: false,
          })
          return
        }

        completeNavigation(routeIdentity, hash, options, navigationId, settledUrl)

        pendingNavigationsRef.current.delete(targetPath)
      } catch (error) {
        handleNavigationError(error, targetPath, navigationId, fromRoute)
      }
    })()

    pendingNavigationsRef.current.set(targetPath, {
      targetPath,
      navigationId,
      promise: navigationPromise,
      abortController,
    })

    return navigationPromise
  }

  const navigateRef = useRef<typeof navigate | null>(navigate)

  useLayoutEffect(() => {
    navigateRef.current = navigate

    return () => {
      navigateRef.current = null
    }
  })

  const debouncedNavigateRef = useRef<ReturnType<typeof debounce> | null>(null)

  useLayoutEffect(() => {
    if (debouncedNavigateRef.current != null) return
    debouncedNavigateRef.current = debounce(
      (pathname: string, options: NavigationOptions) => {
        void navigateRef.current?.(pathname, options)
      },
      NAVIGATION_DEBOUNCE_MS,
      {
        leading: true,
        trailing: true,
        maxWait: NAVIGATION_MAX_WAIT_MS,
      },
    )
  }, [])

  const handleLinkClick = (event: MouseEvent) => {
    if (event.button !== 0) return

    if (event.ctrlKey || event.shiftKey || event.altKey || event.metaKey) return

    let target: Element | null = event.target instanceof Element ? event.target : null
    while (target && !(target instanceof HTMLAnchorElement)) target = target.parentElement

    if (!(target instanceof HTMLAnchorElement)) return

    const anchor = target

    if (anchor.target && anchor.target !== '_self') return

    if (anchor.hasAttribute('download')) return

    const href = anchor.getAttribute('href')
    if (href == null || href === '') return

    if (isExternalUrl(href)) return

    if (href.startsWith('#')) {
      event.preventDefault()
      const hash = href.slice(1)
      const element = document.getElementById(hash)
      if (element) {
        element.scrollIntoView({ behavior: 'smooth', block: 'start' })
        const nextUrl = `${window.location.pathname}${window.location.search}#${hash}`
        window.history.pushState(window.history.state, '', nextUrl)
        committedUrlRef.current = nextUrl
      }

      return
    }

    event.preventDefault()

    const pathname = extractPathname(href)

    debouncedNavigateRef.current?.(pathname, { replace: false })
  }

  const handlePopState = (event: PopStateEvent) => {
    const href = `${window.location.pathname}${window.location.search}${window.location.hash}`
    const historyState = isHistoryState(event.state) ? event.state : null

    if (navigateRef.current) {
      void navigateRef.current(href, {
        replace: true,
        scroll: false,
        historyKey: historyState?.key,
      })
    }
  }

  useEffect(() => {
    const currentHistoryState = isHistoryState(window.history.state) ? window.history.state : null

    if (currentHistoryState?.key == null || currentHistoryState.key === '') {
      const initialHistoryState: HistoryState = {
        route: `${window.location.pathname}${window.location.search}`,
        navigationId: 0,
        scrollPosition: { x: window.scrollX, y: window.scrollY },
        timestamp: Date.now(),
        key: generateHistoryKey(),
      }

      window.history.replaceState(
        initialHistoryState,
        '',
        window.location.pathname + window.location.search + window.location.hash,
      )
    }
  }, [initialRoute])

  const handlePageHide = (event: PageTransitionEvent) => {
    if (event.persisted) {
      const currentPath = window.location.pathname
      statePreserverRef.current.captureState(currentPath)
    }
  }

  const handlePageShow = (event: PageTransitionEvent) => {
    if (event.persisted) {
      const currentPath = window.location.pathname
      const historyState = isHistoryState(window.history.state) ? window.history.state : null

      requestAnimationFrame(() => {
        statePreserverRef.current.restoreState(currentPath)

        if (historyState?.scrollPosition)
          window.scrollTo(historyState.scrollPosition.x, historyState.scrollPosition.y)
      })
    }
  }

  useEffect(() => {
    document.addEventListener('click', handleLinkClick)
    window.addEventListener('popstate', handlePopState)
    window.addEventListener('pagehide', handlePageHide)
    window.addEventListener('pageshow', handlePageShow)

    return () => {
      document.removeEventListener('click', handleLinkClick)
      window.removeEventListener('popstate', handlePopState)
      window.removeEventListener('pagehide', handlePageHide)
      window.removeEventListener('pageshow', handlePageShow)
    }
  }, [])

  useEffect(() => {
    isMountedRef.current = true

    registerNavigate(async (href, options) => {
      return navigateRef.current?.(href, options) ?? Promise.resolve()
    })

    return () => {
      isMountedRef.current = false

      deregisterNavigate()
      cancelNavigation()
      cancelAllPendingNavigations()

      debouncedNavigateRef.current?.cancel()
    }
  }, [])

  return children
}
