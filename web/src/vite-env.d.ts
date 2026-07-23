/// <reference types="vite-plus/client" />

interface ImportMetaEnv {
  readonly VITE_POSTHOG_KEY?: string
  readonly VITE_POSTHOG_HOST?: string
  readonly VITE_SENTRY_DSN?: string
  readonly VITE_SENTRY_ENV?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
