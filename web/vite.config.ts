import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { rari } from 'rari/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    rari({
      spamBlocker: {
        enabled: true,
      },
      csp: {
        scriptSrc: ['\'self\'', '\'unsafe-inline\'', 'https://t.rari.build', 'https://js.sentry-cdn.com', 'https://static.cloudflareinsights.com'],
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
  optimizeDeps: {
    include: [
      'posthog-js',
      '@posthog/react',
      '@sentry/react',
    ],
  },
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
