import path from 'node:path'
import { defineConfig } from 'vite-plus'
import { monorepoFmt, monorepoLint } from '../../.config/lint/monorepo'

export default defineConfig({
  fmt: monorepoFmt,
  lint: monorepoLint,
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
