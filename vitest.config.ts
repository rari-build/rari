import { defineConfig } from 'vitest/config'

export default defineConfig({
  resolve: {
    alias: {
      '@rari': new URL('./packages/rari/src', import.meta.url).pathname,
    },
  },
  test: {
    globals: true,
    include: ['test/**/*.test.ts'],
    setupFiles: ['./test/setup.ts'],
    coverage: {
      exclude: [
        'node_modules/',
        'test/',
        '**/*.config.ts',
        '**/dist/',
      ],
    },
  },
})
