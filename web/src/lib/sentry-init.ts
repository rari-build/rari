if (typeof window !== 'undefined') {
  const dsn = import.meta.env.VITE_SENTRY_DSN

  if (dsn) {
    import('@sentry/react').then((Sentry) => {
      try {
        Sentry.init({
          dsn,
          sendDefaultPii: true,
          tracesSampleRate: import.meta.env.DEV ? 1.0 : 0.1,
          environment: import.meta.env.DEV ? 'development' : 'production',
          integrations: [
            Sentry.browserTracingIntegration(),
            Sentry.replayIntegration({
              maskAllText: false,
              blockAllMedia: false,
            }),
          ],
          replaysSessionSampleRate: import.meta.env.DEV ? 1.0 : 0.1,
          replaysOnErrorSampleRate: 1.0,
          beforeSend(event, hint) {
            const error = hint.originalException
            if (error instanceof Error) {
              if (error.message === 'Illegal invocation') {
                const stack = error.stack || ''
                if (stack.includes('sentry') || stack.includes('Proxy')) {
                  console.warn('[Sentry] Skipping error caused by browser extension interference')
                  return null
                }
              }

              if (error.message.includes('Unexpected non-whitespace character after JSON')) {
                const stack = error.stack || ''
                if (stack.includes('parseRscWireFormat') || stack.includes('AppRouter')) {
                  console.warn('[Sentry] Skipping RSC parsing error (likely userscript corruption)')
                  return null
                }
              }

              if (error.message === 'Load failed') {
                const stack = error.stack || ''
                if (stack.includes('fetchRouteInfo') || stack.includes('route-info')) {
                  console.warn('[Sentry] Skipping Safari iOS response.json() error (handled with fallback)')
                  return null
                }
              }
            }

            if (typeof error === 'string') {
              if (error.includes('Object Not Found Matching Id:') && error.includes('MethodName:')) {
                console.warn('[Sentry] Skipping bot-related promise rejection')
                return null
              }
            }

            return event
          },
        })

        ;(window as any).Sentry = Sentry
      }
      catch (error) {
        console.warn('[Sentry] Failed to initialize:', error)
      }
    }).catch((error) => {
      console.warn('[Sentry] Failed to load:', error)
    })
  }
}

export {}
