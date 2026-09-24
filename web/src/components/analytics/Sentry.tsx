'use client'

import { useEffect } from 'react'

const EXTENSION_PATTERN =
  /chrome-extension:\/\/|moz-extension:\/\/|safari-extension:|edge-extension:|extensions::/
const SENTRY_SDK_PATTERN = /@sentry|node_modules\/@sentry/

let sentryInitPromise: Promise<void> | null = null

async function initSentry(dsn: string) {
  if (import.meta.env.DEV) return
  if (sentryInitPromise != null) return sentryInitPromise

  sentryInitPromise = (async () => {
    try {
      const Sentry = await import('@sentry/react')
      if (Sentry.getClient() != null) return

      Sentry.init({
        dsn,
        tracesSampleRate: 0.1,
        environment: import.meta.env.VITE_SENTRY_ENV ?? import.meta.env.MODE,
        integrations: [
          Sentry.browserTracingIntegration(),
          Sentry.replayIntegration({
            maskAllText: false,
            blockAllMedia: false,
          }),
        ],
        replaysSessionSampleRate: 0.1,
        replaysOnErrorSampleRate: 1.0,
        beforeSend(event, hint) {
          const error = hint.originalException
          if (error instanceof Error) {
            if (error.message === 'Illegal invocation') {
              const stack = error.stack ?? ''
              if (SENTRY_SDK_PATTERN.test(stack) || EXTENSION_PATTERN.test(stack)) {
                console.warn('[Sentry] Skipping error caused by browser extension interference')
                return null
              }
            }

            if (error.message.includes('Unexpected non-whitespace character after JSON')) {
              const stack = error.stack ?? ''
              if (stack.includes('parseRscFlightProtocol') || stack.includes('AppRouter')) {
                console.warn('[Sentry] Skipping RSC parsing error (likely userscript corruption)')
                return null
              }
            }
          }

          if (
            typeof error === 'string' &&
            error.includes('Object Not Found Matching Id:') &&
            error.includes('MethodName:')
          ) {
            console.warn('[Sentry] Skipping bot-related promise rejection')
            return null
          }

          return event
        },
      })
    } catch (error) {
      sentryInitPromise = null
      console.warn('[Sentry] Failed to initialize:', error)
    }
  })()

  return sentryInitPromise
}

export function Sentry() {
  useEffect(() => {
    const dsn = import.meta.env.VITE_SENTRY_DSN
    if (typeof dsn !== 'string' || dsn === '') return undefined

    let timer: ReturnType<typeof setTimeout> | undefined
    let started = false

    const loadSentry = () => {
      if (started) return
      started = true
      if (timer != null) {
        clearTimeout(timer)
        timer = undefined
      }
      void initSentry(dsn).catch((error: unknown) => {
        console.warn('[Sentry] Failed to load:', error)
      })
      document.removeEventListener('click', loadSentry)
      document.removeEventListener('scroll', loadSentry)
      document.removeEventListener('keydown', loadSentry)
    }

    document.addEventListener('click', loadSentry, { once: true, passive: true })
    document.addEventListener('scroll', loadSentry, { once: true, passive: true })
    document.addEventListener('keydown', loadSentry, { once: true, passive: true })
    timer = setTimeout(loadSentry, 5000)

    return () => {
      if (timer != null) clearTimeout(timer)
      document.removeEventListener('click', loadSentry)
      document.removeEventListener('scroll', loadSentry)
      document.removeEventListener('keydown', loadSentry)
    }
  }, [])

  return null
}
