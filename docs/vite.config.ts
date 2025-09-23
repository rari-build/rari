import tailwindcss from '@tailwindcss/vite'
import { rari, rariRouter } from 'rari'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [rari(), rariRouter(), tailwindcss()],
})
