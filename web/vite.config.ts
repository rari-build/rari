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
        layers: {
          response: {
            handler: 'memory',
            maxEntries: 40,
            maxBytes: 8 * 1024 * 1024,
          },
          layout: {
            handler: 'memory',
            maxEntries: 40,
            maxBytes: 4 * 1024 * 1024,
          },
          image: {
            handler: 'memory',
            maxEntries: 20,
            maxBytes: 8 * 1024 * 1024,
          },
          og: {
            handler: 'memory',
            maxEntries: 20,
            maxBytes: 4 * 1024 * 1024,
          },
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
