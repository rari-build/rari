import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { rari, rariRouter } from 'rari/vite'
import { defineConfig } from 'rolldown-vite'

export default defineConfig({
  plugins: [rari(), rariRouter(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
    },
  },
})
