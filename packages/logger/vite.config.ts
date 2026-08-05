import { defineConfig } from 'vite-plus'
import { monorepoFmt, monorepoLint } from '../../.config/lint/monorepo'

export default defineConfig({
  fmt: monorepoFmt,
  lint: monorepoLint,
  pack: {
    entry: ['src/index.ts'],
    minify: true,
  },
})
