import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { rari } from 'rari/vite'
import { defineConfig } from 'rolldown-vite'

export default defineConfig({
  plugins: [
    rari({
      images: {
        deviceSizes: [1920],
        imageSizes: [384, 400, 600, 1200],
        qualityAllowlist: [25, 50, 75, 100],
        remotePatterns: [
          {
            hostname: 'images.unsplash.com',
          },
        ],
        localPatterns: [
          {
            pathname: '/images/**',
          },
        ],
      },
    }),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
    },
  },
})
