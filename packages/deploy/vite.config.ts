import { fmt, lint } from '@rari/lint/vite'
import { defineConfig } from 'vite-plus'

export default defineConfig({
  fmt,
  lint,
  pack: {
    entry: ['src/index.ts', 'src/railway.ts', 'src/render.ts'],
    minify: true,
  },
})
