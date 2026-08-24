import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { rari } from 'rari/vite'
import { defineConfig } from 'vite-plus'
import { monorepoFmt, monorepoLint } from '../.config/lint/monorepo'

export default defineConfig({
  fmt: monorepoFmt,
  lint: monorepoLint,
  plugins: [
    rari({
      compiler: true,
      csp: {
        scriptSrc: [
          "'self'",
          "'unsafe-inline'",
          'https://t.rari.build',
          'https://js.sentry-cdn.com',
        ],
        connectSrc: [
          "'self'",
          'ws:',
          'wss:',
          'https://t.rari.build',
          'https://*.ingest.us.sentry.io',
        ],
        workerSrc: ["'self'", 'blob:'],
      },
      cacheControl: {
        routes: {
          '/*': 'public, max-age=7200, stale-while-revalidate=86400',
        },
      },
      cache: {
        maxBytes: 24 * 1024 * 1024,
        layers: {
          response: { maxEntries: 40 },
          layout: { maxEntries: 40 },
          image: { maxEntries: 20 },
          og: { maxEntries: 20 },
          fetch: { maxEntries: 32 },
        },
      },
    }),
    tailwindcss(),
  ],
  build: {
    chunkSizeWarningLimit: 2000,
    rolldownOptions: {
      output: {
        codeSplitting: {
          includeDependenciesRecursively: false,
          minShareCount: 10,
          groups: [
            {
              name: moduleId => {
                if (moduleId.includes('node_modules')) {
                  if (moduleId.includes('posthog')) return 'posthog'
                  if (moduleId.includes('@sentry')) return 'sentry'
                  if (moduleId.includes('react-dom')) return 'react-dom'
                }

                return null
              },
            },
          ],
        },
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, 'src'),
    },
  },
})
