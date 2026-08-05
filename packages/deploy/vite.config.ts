import { defineConfig } from 'vite-plus'
import { monorepoFmt, monorepoLint } from '../../.config/lint/monorepo'

export default defineConfig({
  fmt: monorepoFmt,
  lint: monorepoLint,
  pack: {
    entry: ['src/index.ts', 'src/railway.ts', 'src/render.ts'],
    minify: true,
  },
})
