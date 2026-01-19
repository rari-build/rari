import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { rari } from 'rari/vite'
import { defineConfig } from 'rolldown-vite'

export default defineConfig({
  plugins: [
    rari({
      csp: {
        scriptSrc: ['\'self\'', '\'unsafe-inline\'', 'https://us-assets.i.posthog.com'],
        connectSrc: ['\'self\'', 'ws:', 'wss:', 'https://us.i.posthog.com', 'https://us-assets.i.posthog.com'],
      },
    }),
    tailwindcss(),
  ],
  build: {
    rolldownOptions: {
      output: {
        advancedChunks: {
          groups: [
            {
              name: (moduleId) => {
                if (moduleId.includes('node_modules')) {
                  if (moduleId.includes('react') || moduleId.includes('react-dom'))
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
