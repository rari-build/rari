import { defineConfig } from 'vite-plus'
import { fmt, lint } from './src/vite'

export default defineConfig({
  fmt,
  lint,
  pack: {
    entry: ['src/vite.ts', 'src/eslint.ts'],
    minify: true,
    deps: {
      neverBundle: true,
    },
  },
})
