import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { rari } from 'rari/vite'
import { defineConfig } from 'rolldown-vite'

export default defineConfig({
  plugins: [
    rari({
      csp: {
        scriptSrc: ['\'self\'', '\'unsafe-inline\'', 'https://us-assets.i.posthog.com', 'https://js.sentry-cdn.com'],
        connectSrc: ['\'self\'', 'ws:', 'wss:', 'https://us.i.posthog.com', 'https://us-assets.i.posthog.com', 'https://*.ingest.us.sentry.io'],
        workerSrc: ['\'self\'', 'blob:'],
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
        advancedChunks: {
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
                  if (moduleId.includes('react'))
                    return 'react'
                  return 'vendor'
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
      '@': path.resolve(__dirname, 'src'),
    },
  },
})
