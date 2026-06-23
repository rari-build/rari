import { defineConfig } from 'vite-plus'

export default defineConfig({
  pack: {
    entry: {
      'index': 'src/index.ts',
      'runtime/cache-wrapper': 'src/runtime/cache-wrapper.ts',
      'runtime/cache-storage-redb': 'src/runtime/cache-storage-redb.ts',
      'runtime/cache-storage-remote-ops': 'src/runtime/cache-storage-remote-ops.ts',
      'runtime/cache-storage-test': 'src/runtime/cache-storage-test.ts',
      'runtime/deterministic-stringify': 'src/runtime/deterministic-stringify.ts',
    },
    minify: true,
  },
})
