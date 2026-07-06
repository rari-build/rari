import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { rari } from 'rari/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    rari({
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
      '@rari/use-cache': path.resolve(import.meta.dirname, '../../../packages/use-cache/src'),
    },
  },
})
