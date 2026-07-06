import { defineConfig } from 'vite-plus'

export default defineConfig({
  pack: {
    entry: {
      'index': 'src/index.ts',
      'runtime/cache-wrapper': 'src/runtime/cache-wrapper.ts',
    },
    minify: true,
  },
})
