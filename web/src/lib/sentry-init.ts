if (typeof window !== 'undefined') {
  const dsn = import.meta.env.VITE_SENTRY_DSN

  if (dsn) {
    import('@sentry/react').then((Sentry) => {
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
      })

      ;(window as any).Sentry = Sentry
    })
  }
}

export {}
