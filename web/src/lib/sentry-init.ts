import * as Sentry from '@sentry/react'

declare global {
  interface Window {
    Sentry: typeof Sentry
  }
}

const dsn = import.meta.env.VITE_SENTRY_DSN

if (dsn) {
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
}

if (typeof window !== 'undefined') {
  window.Sentry = Sentry
}
