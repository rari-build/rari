'use client'

import type { AppRouteManifest, LayoutEntry, LoadingEntry } from './app-types'
import type { NavigationError } from './navigation-error-handler'
import type { NavigationOptions } from './navigation-types'
import React, { useEffect, useRef, useState } from 'react'
import { debounce } from './debounce'
import { LayoutDataManager } from './LayoutDataManager'
import { LayoutManager } from './LayoutManager'
import { NavigationErrorHandler } from './navigation-error-handler'
import { extractPathname, findLayoutChain, isExternalUrl, normalizePath } from './navigation-utils'
import { NavigationErrorOverlay } from './NavigationErrorOverlay'
import { StatePreserver } from './StatePreserver'

export interface ClientRouterProps {
  children: React.ReactNode
  manifest: AppRouteManifest
  initialRoute: string
}

interface NavigationState {
  currentRoute: string
  targetRoute: string | null
  isNavigating: boolean
  navigationId: number
  error: NavigationError | null
  showingLoadingComponent: boolean
  loadingComponentRoute: string | null
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
  scrollPosition?: { x: number, y: number }
  timestamp: number
  key: string
}

export function ClientRouter({ children, manifest, initialRoute }: ClientRouterProps) {
  const [navigationState, setNavigationState] = useState<NavigationState>(() => ({
    currentRoute: normalizePath(initialRoute),
    targetRoute: null,
    isNavigating: false,
    navigationId: 0,
    error: null,
    showingLoadingComponent: false,
    loadingComponentRoute: null,
  }))

  const abortControllerRef = useRef<AbortController | null>(null)
  const isMountedRef = useRef(true)
  const currentRouteRef = useRef<string>(normalizePath(initialRoute))
  const layoutManagerRef = useRef<LayoutManager>(new LayoutManager())
  const layoutDataManagerRef = useRef<LayoutDataManager>(new LayoutDataManager())

  const errorHandlerRef = useRef<NavigationErrorHandler>(
    new NavigationErrorHandler({
      timeout: 10000,
      maxRetries: 3,
      onError: (error) => {
        console.error('[ClientRouter] Navigation error:', error)
      },
      onRetry: (attempt, error) => {
        console.warn(`[ClientRouter] Retry attempt ${attempt} for ${error.url}`)
      },
    }),
  )

  const pendingNavigationsRef = useRef<Map<string, PendingNavigation>>(new Map())
  const navigationQueueRef = useRef<Array<{ path: string, options: NavigationOptions }>>([])

  const statePreserverRef = useRef<StatePreserver>(new StatePreserver({
    maxHistorySize: 50,
  }))

  const NAVIGATION_DEBOUNCE_MS = 50
  const NAVIGATION_MAX_WAIT_MS = 200

  const generateHistoryKey = (): string => {
    return `${Date.now()}-${Math.random().toString(36).substring(2, 9)}`
  }

  const cancelNavigation = () => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort()
      abortControllerRef.current = null
    }
  }

  const cancelAllPendingNavigations = () => {
    for (const [, pending] of pendingNavigationsRef.current.entries()) {
      pending.abortController.abort()
    }
    pendingNavigationsRef.current.clear()
  }

  const cleanupAbortedNavigation = (path: string, navigationId: number) => {
    pendingNavigationsRef.current.delete(path)

    if (isMountedRef.current && navigationState.navigationId === navigationId) {
      setNavigationState(prev => ({
        ...prev,
        targetRoute: null,
        isNavigating: false,
        showingLoadingComponent: false,
        loadingComponentRoute: null,
      }))
    }
  }

  const getLayoutChain = (route: string): LayoutEntry[] => {
    return findLayoutChain(route, manifest)
  }

  const findLoadingComponent = (targetPath: string): LoadingEntry | null => {
    if (!manifest.loading || manifest.loading.length === 0) {
      return null
    }

    const exactMatch = manifest.loading.find(loading => loading.path === targetPath)
    if (exactMatch) {
      return exactMatch
    }

    const segments = targetPath.split('/').filter(Boolean)
    for (let i = segments.length - 1; i >= 0; i--) {
      const parentPath = `/${segments.slice(0, i).join('/')}`
      const parentMatch = manifest.loading.find(loading => loading.path === parentPath)
      if (parentMatch) {
        return parentMatch
      }
    }

    const rootMatch = manifest.loading.find(loading => loading.path === '/')
    if (rootMatch) {
      return rootMatch
    }

    return null
  }

  const validateRoute = (targetPath: string): boolean => {
    if (targetPath === '/') {
      return true
    }

    const routeExists = manifest.routes.some((route) => {
      if (route.path === targetPath) {
        return true
      }

      if (targetPath.startsWith(`${route.path}/`)) {
        return true
      }
      return false
    })

    return routeExists
  }

  const processNavigationQueueRef = useRef<(() => Promise<void>) | null>(null)

  const queueNavigation = (path: string, options: NavigationOptions = {}) => {
    navigationQueueRef.current.push({ path, options })

    setTimeout(() => {
      processNavigationQueueRef.current?.()
    }, 50)
  }

  const navigate = async (href: string, options: NavigationOptions = {}) => {
    if (!href || typeof href !== 'string') {
      console.error('[ClientRouter] Invalid navigation target:', href)
      return
    }

    const targetPath = normalizePath(href)

    if (targetPath === currentRouteRef.current && !options.replace) {
      return
    }

    const existingPending = pendingNavigationsRef.current.get(targetPath)
    if (existingPending) {
      return existingPending.promise
    }

    if (navigationState.isNavigating && navigationState.targetRoute !== targetPath) {
      queueNavigation(targetPath, options)
      return
    }

    validateRoute(targetPath)

    cancelAllPendingNavigations()
    cancelNavigation()

    const abortController = new AbortController()
    abortControllerRef.current = abortController

    const navigationId = navigationState.navigationId + 1

    const loadingComponent = findLoadingComponent(targetPath)

    setNavigationState(prev => ({
      ...prev,
      targetRoute: targetPath,
      isNavigating: true,
      navigationId,
      error: null,
      showingLoadingComponent: !!loadingComponent,
      loadingComponentRoute: loadingComponent?.path || null,
    }))

    if (loadingComponent) {
      window.dispatchEvent(new CustomEvent('rari:show-loading', {
        detail: {
          route: targetPath,
          navigationId,
          loadingComponent,
        },
      }))
    }

    const navigationPromise = (async () => {
      const fromRoute = currentRouteRef.current
      try {
        const currentLayoutChain = getLayoutChain(fromRoute)
        const targetLayoutChain = getLayoutChain(targetPath)

        const layoutDiff = layoutManagerRef.current.computeLayoutDiff(
          currentLayoutChain,
          targetLayoutChain,
        )

        const layoutsNeedingRefetch = layoutDataManagerRef.current.getLayoutsNeedingRefetch(
          layoutDiff,
          fromRoute,
          targetPath,
        )

        if (!options.historyKey) {
          statePreserverRef.current.captureState(fromRoute)
        }

        const historyKey = options.historyKey || generateHistoryKey()
        const historyState: HistoryState = {
          route: targetPath,
          navigationId,
          scrollPosition: { x: window.scrollX, y: window.scrollY },
          timestamp: Date.now(),
          key: historyKey,
        }

        if (options.replace) {
          window.history.replaceState(
            historyState,
            '',
            targetPath,
          )
        }
        else {
          window.history.pushState(
            historyState,
            '',
            targetPath,
          )
        }

        const fetchUrl = window.location.origin + targetPath

        const response = await fetch(fetchUrl, {
          headers: { Accept: 'text/x-component' },
          signal: abortController.signal,
        })

        if (!response.ok) {
          throw new Error(`Failed to fetch: ${response.status}`)
        }

        if (abortController.signal.aborted) {
          cleanupAbortedNavigation(targetPath, navigationId)
          return
        }

        const rscWireFormat = await response.text()

        window.dispatchEvent(new CustomEvent('rari:navigate', {
          detail: {
            from: fromRoute,
            to: targetPath,
            navigationId,
            options,
            layoutDiff,
            currentLayoutChain,
            targetLayoutChain,
            layoutsNeedingRefetch,
            abortSignal: abortController.signal,
            rscWireFormat,
            loadingComponent,
            hasLoadingComponent: !!loadingComponent,
          },
        }))

        if (abortController.signal.aborted) {
          cleanupAbortedNavigation(targetPath, navigationId)
          return
        }

        if (isMountedRef.current) {
          currentRouteRef.current = targetPath

          setNavigationState(prev => ({
            ...prev,
            currentRoute: targetPath,
            targetRoute: null,
            isNavigating: false,
            error: null,
            showingLoadingComponent: false,
            loadingComponentRoute: null,
          }))

          errorHandlerRef.current.resetRetry(targetPath)

          if (options.historyKey) {
            requestAnimationFrame(() => {
              statePreserverRef.current.restoreState(targetPath)
            })
          }
        }

        pendingNavigationsRef.current.delete(targetPath)

        processNavigationQueueRef.current?.()
      }
      catch (error) {
        if (error instanceof Error && error.name === 'AbortError') {
          cleanupAbortedNavigation(targetPath, navigationId)
          return
        }

        const navError = errorHandlerRef.current.handleError(error, targetPath)

        if (isMountedRef.current) {
          setNavigationState(prev => ({
            ...prev,
            targetRoute: null,
            isNavigating: false,
            error: navError,
            showingLoadingComponent: false,
            loadingComponentRoute: null,
          }))
        }

        pendingNavigationsRef.current.delete(targetPath)

        window.dispatchEvent(new CustomEvent('rari:navigate-error', {
          detail: {
            from: fromRoute,
            to: targetPath,
            error: navError,
            navigationId,
          },
        }))

        processNavigationQueueRef.current?.()
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

  const processNavigationQueue = async () => {
    if (navigationQueueRef.current.length === 0 || navigationState.isNavigating) {
      return
    }

    const lastNavigation = navigationQueueRef.current[navigationQueueRef.current.length - 1]

    navigationQueueRef.current = []

    await navigate(lastNavigation.path, lastNavigation.options)
  }

  processNavigationQueueRef.current = processNavigationQueue

  const debouncedNavigateRef = useRef<ReturnType<typeof debounce> | null>(null)

  if (!debouncedNavigateRef.current) {
    debouncedNavigateRef.current = debounce(
      (pathname: string, options: NavigationOptions) => {
        navigate(pathname, options)
      },
      NAVIGATION_DEBOUNCE_MS,
      {
        leading: false,
        trailing: true,
        maxWait: NAVIGATION_MAX_WAIT_MS,
      },
    )
  }

  const debouncedNavigate = debouncedNavigateRef.current

  const handleLinkClick = (event: MouseEvent) => {
    if (event.button !== 0) {
      return
    }

    if (event.ctrlKey || event.shiftKey || event.altKey || event.metaKey) {
      return
    }

    let target = event.target as HTMLElement | null
    while (target && target.tagName !== 'A') {
      target = target.parentElement
    }

    if (!target || target.tagName !== 'A') {
      return
    }

    const anchor = target as HTMLAnchorElement

    if (anchor.target && anchor.target !== '_self') {
      return
    }

    if (anchor.hasAttribute('download')) {
      return
    }

    const href = anchor.getAttribute('href')
    if (!href) {
      return
    }

    if (isExternalUrl(href)) {
      return
    }

    event.preventDefault()

    const pathname = extractPathname(href)

    debouncedNavigate(pathname, { replace: false })
  }

  const handlePopState = (event: PopStateEvent) => {
    const pathname = window.location.pathname
    const historyState = event.state as HistoryState | null

    navigate(pathname, {
      replace: true,
      scroll: false,
      historyKey: historyState?.key,
    })
  }

  const handleRetry = () => {
    if (navigationState.error && navigationState.error.url) {
      const targetPath = navigationState.error.url
      errorHandlerRef.current.incrementRetry(targetPath)
      navigate(targetPath, { replace: false })
    }
  }

  const handleReload = () => {
    window.location.reload()
  }

  const handleDismiss = () => {
    setNavigationState(prev => ({
      ...prev,
      error: null,
    }))
  }

  useEffect(() => {
    const currentHistoryState = window.history.state as HistoryState | null

    if (!currentHistoryState || !currentHistoryState.key) {
      const initialHistoryState: HistoryState = {
        route: navigationState.currentRoute,
        navigationId: navigationState.navigationId,
        scrollPosition: { x: window.scrollX, y: window.scrollY },
        timestamp: Date.now(),
        key: generateHistoryKey(),
      }

      window.history.replaceState(
        initialHistoryState,
        '',
        window.location.pathname + window.location.search,
      )
    }
  }, [])

  const handlePageHide = (event: PageTransitionEvent) => {
    if (event.persisted) {
      const currentPath = window.location.pathname
      statePreserverRef.current.captureState(currentPath)
    }
  }

  const handlePageShow = (event: PageTransitionEvent) => {
    if (event.persisted) {
      const currentPath = window.location.pathname
      const historyState = window.history.state as HistoryState | null

      requestAnimationFrame(() => {
        statePreserverRef.current.restoreState(currentPath)

        if (historyState?.scrollPosition) {
          window.scrollTo(historyState.scrollPosition.x, historyState.scrollPosition.y)
        }
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

      isMountedRef.current = false

      cancelNavigation()
      cancelAllPendingNavigations()

      if (debouncedNavigate.cancel) {
        debouncedNavigate.cancel()
      }
    }
  }, [])

  return (
    <>
      {children}
      {navigationState.error && (
        <NavigationErrorOverlay
          error={navigationState.error}
          onRetry={handleRetry}
          onReload={handleReload}
          onDismiss={handleDismiss}
          retryCount={errorHandlerRef.current.getRetryCount(navigationState.error.url || '')}
          maxRetries={3}
        />
      )}
    </>
  )
}
