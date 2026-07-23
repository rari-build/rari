'use client'

import type * as React from 'react'
import type { NavigationError } from './error-handler'
import type { NavigationOptions } from './types'
import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { isHistoryState, isRecord, parseJsonRecord } from '@/shared/utils/type-guards'
import { debounce } from './debounce'
import { NavigationErrorHandler } from './error-handler'
import { extractPathname, isExternalUrl, normalizePath } from './match'
import { deregisterNavigate, registerNavigate } from './navigate'
import { routeInfoCache } from './route-info'
import { StatePreserver } from './state-preserver'

interface PageMetadata {
  readonly title?: string
  readonly description?: string
  readonly keywords?: readonly string[]
  readonly viewport?: string
  readonly canonical?: string
  readonly openGraph?: {
    readonly title?: string
    readonly description?: string
    readonly url?: string
    readonly siteName?: string
    readonly images?: readonly string[]
    readonly type?: string
  }
  readonly twitter?: {
    readonly card?: string
    readonly site?: string
    readonly creator?: string
    readonly title?: string
    readonly description?: string
    readonly images?: readonly string[]
  }
  readonly robots?: {
    readonly index?: boolean
    readonly follow?: boolean
    readonly nocache?: boolean
  }
}

function updateOrCreateMetaTag(
  selector: string,
  attributes: Readonly<{ readonly [key: string]: string }>,
) {
  let element = document.querySelector(selector)
  if (!element) {
    element = document.createElement('meta')
    for (const [key, value] of Object.entries(attributes)) element.setAttribute(key, value)

    document.head.appendChild(element)
  } else {
    if (attributes.content) element.setAttribute('content', attributes.content)
  }
}

function removeMetaTag(selector: string) {
  const element = document.querySelector(selector)
  if (element) element.remove()
}

function updateBasicMetadata(metadata: PageMetadata): void {
  if (metadata.title != null && metadata.title !== '') document.title = metadata.title

  if (metadata.description != null && metadata.description !== '') {
    updateOrCreateMetaTag('meta[name="description"]', {
      name: 'description',
      content: metadata.description,
    })
  } else {
    removeMetaTag('meta[name="description"]')
  }

  if (metadata.keywords && metadata.keywords.length > 0) {
    updateOrCreateMetaTag('meta[name="keywords"]', {
      name: 'keywords',
      content: metadata.keywords.join(', '),
    })
  } else {
    removeMetaTag('meta[name="keywords"]')
  }

  if (metadata.viewport != null && metadata.viewport !== '') {
    updateOrCreateMetaTag('meta[name="viewport"]', {
      name: 'viewport',
      content: metadata.viewport,
    })
  }
}

function updateCanonicalLink(canonical: string | undefined): void {
  const canonicalEl = document.querySelector('link[rel="canonical"]')

  if (canonical === undefined) {
    if (canonicalEl) canonicalEl.remove()

    return
  }

  if (!canonicalEl) {
    const newCanonicalEl = document.createElement('link')
    newCanonicalEl.setAttribute('rel', 'canonical')
    newCanonicalEl.setAttribute('href', canonical)
    document.head.appendChild(newCanonicalEl)
  } else {
    canonicalEl.setAttribute('href', canonical)
  }
}

function updateRobotsMetadata(robots: PageMetadata['robots']): void {
  if (robots === undefined) {
    removeMetaTag('meta[name="robots"]')
    return
  }

  const robotsContent: string[] = []
  if (robots.index !== undefined) robotsContent.push(robots.index ? 'index' : 'noindex')
  if (robots.follow !== undefined) robotsContent.push(robots.follow ? 'follow' : 'nofollow')
  if (robots.nocache) robotsContent.push('nocache')

  if (robotsContent.length > 0) {
    updateOrCreateMetaTag('meta[name="robots"]', {
      name: 'robots',
      content: robotsContent.join(', '),
    })
  } else {
    removeMetaTag('meta[name="robots"]')
  }
}

function updateOpenGraphMetadata(og: PageMetadata['openGraph']): void {
  if (og === undefined) {
    removeMetaTag('meta[property="og:title"]')
    removeMetaTag('meta[property="og:description"]')
    removeMetaTag('meta[property="og:url"]')
    removeMetaTag('meta[property="og:site_name"]')
    removeMetaTag('meta[property="og:type"]')
    document.querySelectorAll('meta[property="og:image"]').forEach(el => {
      el.remove()
    })
    return
  }

  if (og.title != null && og.title !== '') {
    updateOrCreateMetaTag('meta[property="og:title"]', {
      property: 'og:title',
      content: og.title,
    })
  } else {
    removeMetaTag('meta[property="og:title"]')
  }

  if (og.description != null && og.description !== '') {
    updateOrCreateMetaTag('meta[property="og:description"]', {
      property: 'og:description',
      content: og.description,
    })
  } else {
    removeMetaTag('meta[property="og:description"]')
  }

  if (og.url != null && og.url !== '') {
    updateOrCreateMetaTag('meta[property="og:url"]', {
      property: 'og:url',
      content: og.url,
    })
  } else {
    removeMetaTag('meta[property="og:url"]')
  }

  if (og.siteName != null && og.siteName !== '') {
    updateOrCreateMetaTag('meta[property="og:site_name"]', {
      property: 'og:site_name',
      content: og.siteName,
    })
  } else {
    removeMetaTag('meta[property="og:site_name"]')
  }

  if (og.type != null && og.type !== '') {
    updateOrCreateMetaTag('meta[property="og:type"]', {
      property: 'og:type',
      content: og.type,
    })
  } else {
    removeMetaTag('meta[property="og:type"]')
  }

  if (og.images && og.images.length > 0) {
    document.querySelectorAll('meta[property="og:image"]').forEach(el => {
      el.remove()
    })
    for (const image of og.images) {
      const meta = document.createElement('meta')
      meta.setAttribute('property', 'og:image')
      meta.setAttribute('content', image)
      document.head.appendChild(meta)
    }
  } else {
    document.querySelectorAll('meta[property="og:image"]').forEach(el => {
      el.remove()
    })
  }
}

function updateTwitterMetadata(twitter: PageMetadata['twitter']): void {
  if (twitter === undefined) {
    removeMetaTag('meta[name="twitter:card"]')
    removeMetaTag('meta[name="twitter:site"]')
    removeMetaTag('meta[name="twitter:creator"]')
    removeMetaTag('meta[name="twitter:title"]')
    removeMetaTag('meta[name="twitter:description"]')
    document.querySelectorAll('meta[name="twitter:image"]').forEach(el => {
      el.remove()
    })
    return
  }

  if (twitter.card != null && twitter.card !== '') {
    updateOrCreateMetaTag('meta[name="twitter:card"]', {
      name: 'twitter:card',
      content: twitter.card,
    })
  } else {
    removeMetaTag('meta[name="twitter:card"]')
  }

  if (twitter.site != null && twitter.site !== '') {
    updateOrCreateMetaTag('meta[name="twitter:site"]', {
      name: 'twitter:site',
      content: twitter.site,
    })
  } else {
    removeMetaTag('meta[name="twitter:site"]')
  }

  if (twitter.creator != null && twitter.creator !== '') {
    updateOrCreateMetaTag('meta[name="twitter:creator"]', {
      name: 'twitter:creator',
      content: twitter.creator,
    })
  } else {
    removeMetaTag('meta[name="twitter:creator"]')
  }

  if (twitter.title != null && twitter.title !== '') {
    updateOrCreateMetaTag('meta[name="twitter:title"]', {
      name: 'twitter:title',
      content: twitter.title,
    })
  } else {
    removeMetaTag('meta[name="twitter:title"]')
  }

  if (twitter.description != null && twitter.description !== '') {
    updateOrCreateMetaTag('meta[name="twitter:description"]', {
      name: 'twitter:description',
      content: twitter.description,
    })
  } else {
    removeMetaTag('meta[name="twitter:description"]')
  }

  if (twitter.images && twitter.images.length > 0) {
    document.querySelectorAll('meta[name="twitter:image"]').forEach(el => {
      el.remove()
    })
    for (const image of twitter.images) {
      const meta = document.createElement('meta')
      meta.setAttribute('name', 'twitter:image')
      meta.setAttribute('content', image)
      document.head.appendChild(meta)
    }
  } else {
    document.querySelectorAll('meta[name="twitter:image"]').forEach(el => {
      el.remove()
    })
  }
}

function updateDocumentMetadata(metadata: PageMetadata): void {
  updateBasicMetadata(metadata)
  updateCanonicalLink(metadata.canonical)
  updateRobotsMetadata(metadata.robots)
  updateOpenGraphMetadata(metadata.openGraph)
  updateTwitterMetadata(metadata.twitter)
}

export interface ClientRouterProps {
  readonly children: React.ReactNode
  readonly initialRoute: string
  readonly staleWindowMs?: number
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

function isPageMetadata(value: unknown): value is PageMetadata {
  return isRecord(value)
}

export function ClientRouter({
  children,
  initialRoute,
  staleWindowMs = 30_000,
}: ClientRouterProps): React.ReactNode {
  const [navigationState, setNavigationState] = useState<NavigationState>(() => ({
    currentRoute: normalizePath(initialRoute),
    navigationId: 0,
    error: null,
  }))

  const abortControllerRef = useRef<AbortController | null>(null)
  const isMountedRef = useRef(true)
  const currentRouteRef = useRef<string>(normalizePath(initialRoute))
  const navigationIdCounterRef = useRef<number>(0)

  const errorHandlerRef = useRef<NavigationErrorHandler>(
    new NavigationErrorHandler({
      timeout: 10000,
      maxRetries: 3,
      onError: error => {
        console.error('[rari] Router: Navigation error:', error)
      },
      onRetry: () => {},
    }),
  )

  const pendingNavigationsRef = useRef<Map<string, PendingNavigation>>(new Map())
  const navigationQueueRef = useRef<Array<{ path: string; options: NavigationOptions }>>([])

  const statePreserverRef = useRef<StatePreserver>(
    new StatePreserver({
      maxHistorySize: 50,
    }),
  )

  const lastHiddenAtRef = useRef<number | null>(null)
  const staleWindowMsRef = useRef<number>(staleWindowMs)

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

  const cleanupAbortedNavigation = (pendingPath: string, navigationId: number) => {
    pendingNavigationsRef.current.delete(pendingPath)

    if (isMountedRef.current && navigationState.navigationId === navigationId) {
      setNavigationState(prev => ({
        ...prev,
      }))
    }
  }

  const processNavigationQueueRef = useRef<(() => Promise<void>) | null>(null)

  const handleSameRouteNavigation = (targetPath: string, hash: string) => {
    if (!hash) return

    const element = document.getElementById(hash)
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' })
      window.history.pushState(window.history.state, '', `${targetPath}#${hash}`)
    }
  }

  const processMetadata = (response: Response) => {
    try {
      const metadataHeader = response.headers.get('x-rari-metadata')
      if (metadataHeader != null && metadataHeader !== '') {
        const decodedMetadata = decodeURIComponent(metadataHeader)
        const metadataRecord = parseJsonRecord(decodedMetadata)
        if (metadataRecord && isPageMetadata(metadataRecord)) updateDocumentMetadata(metadataRecord)
      }
    } catch (error) {
      console.warn('[rari] Router: Failed to parse x-rari-metadata header:', error)
    }
  }

  const handleNonStreamingResponse = (
    responsePromise: Promise<Response>,
    fromRoute: string,
    actualTargetPath: string,
    navigationId: number,
    options: NavigationOptions,
    abortController: AbortController,
  ) => {
    if (navigationIdCounterRef.current !== navigationId) return

    window.dispatchEvent(
      new CustomEvent('rari:navigate', {
        detail: {
          from: fromRoute,
          to: actualTargetPath,
          navigationId,
          options,
          abortSignal: abortController.signal,
          rscResponsePromise: responsePromise,
        },
      }),
    )
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
  ) => {
    if (!isMountedRef.current) return

    if (navigationIdCounterRef.current !== navigationId) return

    currentRouteRef.current = actualTargetPath

    setNavigationState(prev => ({
      ...prev,
      currentRoute: actualTargetPath,
      navigationId,
      error: null,
    }))

    errorHandlerRef.current.resetRetry(actualTargetPath)
    handleScrollAfterNavigation(actualTargetPath, hash, options)
  }

  const handleNavigationError = (
    error: unknown,
    targetPath: string,
    navigationId: number,
    fromRoute: string,
  ) => {
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
      window.history.replaceState(window.history.state, '', fromRoute)
    }

    pendingNavigationsRef.current.delete(targetPath)

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

    void processNavigationQueueRef.current?.()
  }

  const navigate = async (href: string, options: NavigationOptions = {}) => {
    if (!href || typeof href !== 'string') return

    const [pathWithoutHash, hash] = href.includes('#') ? href.split('#') : [href, '']
    const targetPath = normalizePath(pathWithoutHash)

    if (targetPath === currentRouteRef.current && !options.replace) {
      handleSameRouteNavigation(targetPath, hash)
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
      const fromRoute = currentRouteRef.current
      try {
        if (options.historyKey == null || options.historyKey === '')
          statePreserverRef.current.captureState(fromRoute)

        const historyKey =
          options.historyKey != null && options.historyKey !== ''
            ? options.historyKey
            : generateHistoryKey()

        const fetchUrl = window.location.origin + targetPath

        const urlWithHash = hash ? `${targetPath}#${hash}` : targetPath
        const historyState: HistoryState = {
          route: targetPath,
          navigationId,
          scrollPosition: { x: window.scrollX, y: window.scrollY },
          timestamp: Date.now(),
          key: historyKey,
        }

        if (options.replace) window.history.replaceState(historyState, '', urlWithHash)
        else window.history.pushState(historyState, '', urlWithHash)

        const fetchPromise = fetch(fetchUrl, {
          headers: {
            'Accept': 'text/x-component',
            'rari-navigation-id': String(navigationId),
          },
          signal: abortController.signal,
        })

        const rscFetchPromise = fetchPromise.then(response => {
          if (!response.ok && response.status !== 404)
            throw new Error(`Failed to fetch: ${response.status}`)

          return response
        })

        const response = await rscFetchPromise

        if (abortController.signal.aborted) {
          cleanupAbortedNavigation(targetPath, navigationId)
          return
        }

        const finalUrl = new URL(response.url)
        const actualTargetPath = finalUrl.pathname

        if (actualTargetPath !== targetPath) {
          const redirectUrl = hash ? `${actualTargetPath}#${hash}` : actualTargetPath
          window.history.replaceState({ ...historyState, route: actualTargetPath }, '', redirectUrl)
        }

        handleNonStreamingResponse(
          Promise.resolve(response),
          fromRoute,
          actualTargetPath,
          navigationId,
          options,
          abortController,
        )

        processMetadata(response)

        completeNavigation(actualTargetPath, hash, options, navigationId)

        pendingNavigationsRef.current.delete(targetPath)
        void processNavigationQueueRef.current?.()
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

  const processNavigationQueue = async () => {
    if (navigationQueueRef.current.length === 0) return

    const lastNavigation = navigationQueueRef.current.at(-1)
    if (!lastNavigation) return

    navigationQueueRef.current = []

    await navigate(lastNavigation.path, lastNavigation.options)
  }

  const navigateRef = useRef<typeof navigate | null>(navigate)

  useLayoutEffect(() => {
    processNavigationQueueRef.current = processNavigationQueue
    navigateRef.current = navigate

    return () => {
      processNavigationQueueRef.current = null
      navigateRef.current = null
    }
  })

  const debouncedNavigateRef = useRef<ReturnType<typeof debounce> | null>(null)

  debouncedNavigateRef.current ??= debounce(
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
        window.history.pushState(window.history.state, '', href)
      }

      return
    }

    event.preventDefault()

    const pathname = extractPathname(href)

    if (debouncedNavigateRef.current) debouncedNavigateRef.current(pathname, { replace: false })
  }

  const handlePopState = (event: PopStateEvent) => {
    const pathname = window.location.pathname
    const historyState = isHistoryState(event.state) ? event.state : null

    if (navigateRef.current) {
      void navigateRef.current(pathname, {
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
        route: normalizePath(initialRoute),
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
    staleWindowMsRef.current = staleWindowMs

    const handleVisibilityChange = () => {
      if (document.hidden) {
        lastHiddenAtRef.current = Date.now()
      } else {
        if (lastHiddenAtRef.current !== null) {
          const hiddenDuration = Date.now() - lastHiddenAtRef.current
          if (hiddenDuration > staleWindowMsRef.current) {
            routeInfoCache.clear()
          }
        }
      }
    }

    document.addEventListener('visibilitychange', handleVisibilityChange)

    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange)
    }
  }, [staleWindowMs])

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

      if (debouncedNavigateRef.current?.cancel) debouncedNavigateRef.current.cancel()
    }
  }, [])

  return children
}
