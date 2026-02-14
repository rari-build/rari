'use client'

import type { NavigationError } from './navigation-error-handler'
import type { NavigationOptions } from './navigation-types'
import type { RouteInfoResponse } from './route-info-types'
import * as React from 'react'
import { useEffect, useRef, useState } from 'react'
import { debounce } from './debounce'
import { NavigationErrorHandler } from './navigation-error-handler'
import { extractPathname, isExternalUrl, normalizePath } from './navigation-utils'
import { NavigationErrorOverlay } from './NavigationErrorOverlay'
import { routeInfoCache } from './route-info-client'
import { StatePreserver } from './StatePreserver'

interface PageMetadata {
  title?: string
  description?: string
  keywords?: string[]
  viewport?: string
  canonical?: string
  openGraph?: {
    title?: string
    description?: string
    url?: string
    siteName?: string
    images?: string[]
    type?: string
  }
  twitter?: {
    card?: string
    site?: string
    creator?: string
    title?: string
    description?: string
    images?: string[]
  }
  robots?: {
    index?: boolean
    follow?: boolean
    nocache?: boolean
  }
}

function updateDocumentMetadata(metadata: PageMetadata): void {
  if (metadata.title)
    document.title = metadata.title

  const updateOrCreateMetaTag = (selector: string, attributes: Record<string, string>) => {
    let element = document.querySelector(selector) as HTMLMetaElement | null
    if (!element) {
      element = document.createElement('meta')
      for (const [key, value] of Object.entries(attributes))
        element.setAttribute(key, value)

      document.head.appendChild(element)
    }
    else {
      if (attributes.content)
        element.setAttribute('content', attributes.content)
    }
  }

  if (metadata.description) {
    updateOrCreateMetaTag('meta[name="description"]', {
      name: 'description',
      content: metadata.description,
    })
  }

  if (metadata.keywords && metadata.keywords.length > 0) {
    updateOrCreateMetaTag('meta[name="keywords"]', {
      name: 'keywords',
      content: metadata.keywords.join(', '),
    })
  }

  if (metadata.viewport) {
    updateOrCreateMetaTag('meta[name="viewport"]', {
      name: 'viewport',
      content: metadata.viewport,
    })
  }

  if (metadata.canonical) {
    let canonical = document.querySelector('link[rel="canonical"]') as HTMLLinkElement | null
    if (!canonical) {
      canonical = document.createElement('link')
      canonical.setAttribute('rel', 'canonical')
      document.head.appendChild(canonical)
    }
    canonical.setAttribute('href', metadata.canonical)
  }

  if (metadata.robots) {
    const robotsContent: string[] = []
    if (metadata.robots.index !== undefined)
      robotsContent.push(metadata.robots.index ? 'index' : 'noindex')
    if (metadata.robots.follow !== undefined)
      robotsContent.push(metadata.robots.follow ? 'follow' : 'nofollow')
    if (metadata.robots.nocache)
      robotsContent.push('nocache')
    if (robotsContent.length > 0) {
      updateOrCreateMetaTag('meta[name="robots"]', {
        name: 'robots',
        content: robotsContent.join(', '),
      })
    }
  }

  if (metadata.openGraph) {
    const og = metadata.openGraph
    if (og.title) {
      updateOrCreateMetaTag('meta[property="og:title"]', {
        property: 'og:title',
        content: og.title,
      })
    }
    if (og.description) {
      updateOrCreateMetaTag('meta[property="og:description"]', {
        property: 'og:description',
        content: og.description,
      })
    }
    if (og.url) {
      updateOrCreateMetaTag('meta[property="og:url"]', {
        property: 'og:url',
        content: og.url,
      })
    }
    if (og.siteName) {
      updateOrCreateMetaTag('meta[property="og:site_name"]', {
        property: 'og:site_name',
        content: og.siteName,
      })
    }
    if (og.type) {
      updateOrCreateMetaTag('meta[property="og:type"]', {
        property: 'og:type',
        content: og.type,
      })
    }
    if (og.images && og.images.length > 0) {
      document.querySelectorAll('meta[property="og:image"]').forEach(el => el.remove())
      for (const image of og.images) {
        const meta = document.createElement('meta')
        meta.setAttribute('property', 'og:image')
        meta.setAttribute('content', image)
        document.head.appendChild(meta)
      }
    }
  }

  if (metadata.twitter) {
    const twitter = metadata.twitter
    if (twitter.card) {
      updateOrCreateMetaTag('meta[name="twitter:card"]', {
        name: 'twitter:card',
        content: twitter.card,
      })
    }
    if (twitter.site) {
      updateOrCreateMetaTag('meta[name="twitter:site"]', {
        name: 'twitter:site',
        content: twitter.site,
      })
    }
    if (twitter.creator) {
      updateOrCreateMetaTag('meta[name="twitter:creator"]', {
        name: 'twitter:creator',
        content: twitter.creator,
      })
    }
    if (twitter.title) {
      updateOrCreateMetaTag('meta[name="twitter:title"]', {
        name: 'twitter:title',
        content: twitter.title,
      })
    }
    if (twitter.description) {
      updateOrCreateMetaTag('meta[name="twitter:description"]', {
        name: 'twitter:description',
        content: twitter.description,
      })
    }
    if (twitter.images && twitter.images.length > 0) {
      document.querySelectorAll('meta[name="twitter:image"]').forEach(el => el.remove())
      for (const image of twitter.images) {
        const meta = document.createElement('meta')
        meta.setAttribute('name', 'twitter:image')
        meta.setAttribute('content', image)
        document.head.appendChild(meta)
      }
    }
  }
}

export interface ClientRouterProps {
  children: React.ReactNode
  initialRoute: string
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
  scrollPosition?: { x: number, y: number }
  timestamp: number
  key: string
}

export function ClientRouter({ children, initialRoute }: ClientRouterProps) {
  const [navigationState, setNavigationState] = useState<NavigationState>(() => ({
    currentRoute: normalizePath(initialRoute),
    navigationId: 0,
    error: null,
  }))

  const abortControllerRef = useRef<AbortController | null>(null)
  const isMountedRef = useRef(true)
  const currentRouteRef = useRef<string>(normalizePath(initialRoute))

  const errorHandlerRef = useRef<NavigationErrorHandler>(
    new NavigationErrorHandler({
      timeout: 10000,
      maxRetries: 3,
      onError: (error) => {
        console.error('[rari] Router: Navigation error:', error)
      },
      onRetry: () => {},
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
    for (const [, pending] of pendingNavigationsRef.current.entries())
      pending.abortController.abort()

    pendingNavigationsRef.current.clear()
  }

  const cleanupAbortedNavigation = (path: string, navigationId: number) => {
    pendingNavigationsRef.current.delete(path)

    if (isMountedRef.current && navigationState.navigationId === navigationId) {
      setNavigationState(prev => ({
        ...prev,
      }))
    }
  }

  const getRouteInfo = async (route: string): Promise<RouteInfoResponse> => {
    return routeInfoCache.get(route)
  }

  const processNavigationQueueRef = useRef<(() => Promise<void>) | null>(null)

  const navigate = async (href: string, options: NavigationOptions = {}) => {
    if (!href || typeof href !== 'string') {
      console.error('[rari] Router: Invalid navigation target:', href)
      return
    }

    const [pathWithoutHash, hash] = href.includes('#') ? href.split('#') : [href, '']
    const targetPath = normalizePath(pathWithoutHash)

    if (targetPath === currentRouteRef.current && !options.replace) {
      if (hash) {
        const element = document.getElementById(hash)
        if (element) {
          element.scrollIntoView({ behavior: 'smooth', block: 'start' })
          window.history.pushState(window.history.state, '', `${targetPath}#${hash}`)
        }
      }

      return
    }

    const existingPending = pendingNavigationsRef.current.get(targetPath)
    if (existingPending)
      return existingPending.promise

    const routeInfo = await getRouteInfo(targetPath)

    cancelAllPendingNavigations()
    cancelNavigation()

    const abortController = new AbortController()
    abortControllerRef.current = abortController

    const navigationId = navigationState.navigationId + 1

    const navigationPromise = (async () => {
      const fromRoute = currentRouteRef.current
      try {
        if (!options.historyKey)
          statePreserverRef.current.captureState(fromRoute)

        const historyKey = options.historyKey || generateHistoryKey()
        const historyState: HistoryState = {
          route: targetPath,
          navigationId,
          scrollPosition: { x: window.scrollX, y: window.scrollY },
          timestamp: Date.now(),
          key: historyKey,
        }

        const urlWithHash = hash ? `${targetPath}#${hash}` : targetPath

        if (options.replace) {
          window.history.replaceState(
            historyState,
            '',
            urlWithHash,
          )
        }
        else {
          window.history.pushState(
            historyState,
            '',
            urlWithHash,
          )
        }

        const fetchUrl = window.location.origin + targetPath

        const response = await fetch(fetchUrl, {
          headers: { Accept: 'text/x-component' },
          signal: abortController.signal,
        })

        if (!response.ok && response.status !== 404)
          throw new Error(`Failed to fetch: ${response.status}`)

        const finalUrl = new URL(response.url)
        const finalPath = finalUrl.pathname
        const actualTargetPath = finalPath !== targetPath ? finalPath : targetPath

        if (finalPath !== targetPath) {
          const finalUrlWithHash = hash ? `${finalPath}#${hash}` : finalPath
          window.history.replaceState(
            {
              route: finalPath,
              navigationId,
              scrollPosition: { x: window.scrollX, y: window.scrollY },
              timestamp: Date.now(),
              key: options.historyKey || generateHistoryKey(),
            },
            '',
            finalUrlWithHash,
          )
        }

        if (abortController.signal.aborted) {
          cleanupAbortedNavigation(actualTargetPath, navigationId)
          return
        }

        try {
          const metadataHeader = response.headers.get('x-rari-metadata')
          if (metadataHeader) {
            const decodedMetadata = decodeURIComponent(metadataHeader)
            const metadata = JSON.parse(decodedMetadata) as PageMetadata
            updateDocumentMetadata(metadata)
          }
        }
        catch {}

        const renderMode = response.headers.get('x-render-mode')
        const isStreaming = renderMode === 'streaming'

        if (isStreaming && response.body) {
          const reader = response.body.getReader()
          const decoder = new TextDecoder()
          let buffer = ''

          try {
            while (true) {
              const { done, value } = await reader.read()

              if (done)
                break

              if (abortController.signal.aborted) {
                await reader.cancel()
                cleanupAbortedNavigation(actualTargetPath, navigationId)
                return
              }

              buffer += decoder.decode(value, { stream: true })

              const lines = buffer.split('\n')
              buffer = lines.pop() || ''

              for (const line of lines) {
                if (line.trim()) {
                  window.dispatchEvent(new CustomEvent('rari:rsc-row', {
                    detail: { rscRow: line },
                  }))
                }
              }
            }

            if (buffer.trim()) {
              window.dispatchEvent(new CustomEvent('rari:rsc-row', {
                detail: { rscRow: buffer },
              }))
            }

            window.dispatchEvent(new CustomEvent('rari:navigate', {
              detail: {
                from: fromRoute,
                to: actualTargetPath,
                navigationId,
                options,
                routeInfo,
                abortSignal: abortController.signal,
                isStreaming: true,
              },
            }))
          }
          catch (streamError) {
            console.error('[rari] Router: Streaming error:', streamError)
            throw streamError
          }
        }
        else {
          const rscWireFormat = await response.text()

          window.dispatchEvent(new CustomEvent('rari:navigate', {
            detail: {
              from: fromRoute,
              to: actualTargetPath,
              navigationId,
              options,
              routeInfo,
              abortSignal: abortController.signal,
              rscWireFormat,
            },
          }))
        }

        if (abortController.signal.aborted) {
          cleanupAbortedNavigation(actualTargetPath, navigationId)
          return
        }

        if (isMountedRef.current) {
          currentRouteRef.current = actualTargetPath

          setNavigationState(prev => ({
            ...prev,
            currentRoute: actualTargetPath,
            error: null,
          }))

          errorHandlerRef.current.resetRetry(actualTargetPath)

          if (options.historyKey) {
            requestAnimationFrame(() => {
              statePreserverRef.current.restoreState(actualTargetPath)
            })
          }
          else if (hash) {
            requestAnimationFrame(() => {
              const scrollToHash = (attempts = 0) => {
                const element = document.getElementById(hash)
                if (element)
                  element.scrollIntoView({ behavior: 'smooth', block: 'start' })
                else if (attempts < 10)
                  setTimeout(scrollToHash, 50, attempts + 1)
              }
              scrollToHash()
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
            error: navError,
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
    if (navigationQueueRef.current.length === 0)
      return

    const lastNavigation = navigationQueueRef.current.at(-1)
    if (!lastNavigation)
      return

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
    if (event.button !== 0)
      return

    if (event.ctrlKey || event.shiftKey || event.altKey || event.metaKey)
      return

    let target = event.target as HTMLElement | null
    while (target && target.tagName !== 'A')
      target = target.parentElement

    if (!target || target.tagName !== 'A')
      return

    const anchor = target as HTMLAnchorElement

    if (anchor.target && anchor.target !== '_self')
      return

    if (anchor.hasAttribute('download'))
      return

    const href = anchor.getAttribute('href')
    if (!href)
      return

    if (isExternalUrl(href))
      return

    if (href.startsWith('#')) {
      event.preventDefault()
      const hash = href.slice(1)
      const element = document.getElementById(hash)
      if (element) {
        element.scrollIntoView({ behavior: 'smooth', block: 'start' })
        window.history.pushState(window.history.state, '', href)
      }

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

      isMountedRef.current = false

      cancelNavigation()
      cancelAllPendingNavigations()

      if (debouncedNavigate.cancel)
        debouncedNavigate.cancel()
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
