import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { rari } from 'rari/vite'
import { defineConfig } from 'vite-plus'

export default defineConfig({
  plugins: [
    rari({
      csp: {
        scriptSrc: ['\'self\'', '\'unsafe-inline\'', 'https://t.rari.build', 'https://js.sentry-cdn.com'],
        connectSrc: ['\'self\'', 'ws:', 'wss:', 'https://t.rari.build', 'https://*.ingest.us.sentry.io'],
        workerSrc: ['\'self\'', 'blob:'],
      },
      cacheControl: {
        routes: {
          '/': 'public, max-age=60, stale-while-revalidate=300',
          '/docs/*': 'public, max-age=3600, stale-while-revalidate=86400',
          '/blog': 'public, max-age=300, stale-while-revalidate=600',
          '/blog/*': 'public, max-age=1800, stale-while-revalidate=3600',
          '/enterprise': 'public, max-age=300, stale-while-revalidate=600',
          '/enterprise/*': 'public, max-age=300, stale-while-revalidate=600',
        },
      },
    }),
    tailwindcss(),
  ],
  build: {
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: (moduleId) => {
                if (moduleId.includes('node_modules')) {
                  if (moduleId.includes('posthog'))
                    return 'posthog'
                  if (moduleId.includes('@sentry'))
                    return 'sentry'
                  if (moduleId.includes('react-dom'))
                    return 'react-dom'
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
