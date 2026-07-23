import path from 'node:path'
import { fmt, lint } from '@rari/lint/vite'
import { defineConfig } from 'vite-plus'

export default defineConfig({
  fmt,
  lint,
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, 'src'),
    },
  },
  pack: {
    entry: {
      'index': 'src/index.ts',
      'runtime/cache-wrapper': 'src/runtime/cache-wrapper.ts',
      'runtime/cache-dynamic-context': 'src/runtime/cache-dynamic-context.ts',
    },
    deps: {
      neverBundle: ['react-server-dom-webpack/client'],
    },
    minify: true,
  },
})
