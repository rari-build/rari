import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { rari } from 'rari/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    rari({
      cacheControl: {
        routes: {
          '/headers-test': 'no-store',
          '/use-cache-revalidate': 'no-store',
        },
      },
      experimental: {
        useCache: true,
        useCacheRemote: {
          handler: 'test',
        },
      },
    }),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, 'src'),
    },
  },
})
