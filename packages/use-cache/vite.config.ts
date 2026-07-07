import { defineConfig } from 'vite-plus'

export default defineConfig({
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
