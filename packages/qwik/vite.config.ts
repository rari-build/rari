import { defineConfig } from 'vite-plus'
import { monorepoFmt, monorepoLint } from '../../.config/lint/monorepo'

export default defineConfig({
  fmt: monorepoFmt,
  lint: monorepoLint,
  pack: {
    entry: {
      index: 'src/index.ts',
      build: 'src/build.ts',
      vite: 'src/vite.ts',
      platform: 'src/platform.ts',
    },
    minify: true,
    deps: {
      neverBundle: [
        '@qwik.dev/core',
        '@qwik.dev/core/server',
        '@qwik.dev/core/optimizer',
        '@qwik.dev/router',
        '@qwik.dev/router/vite',
        '@qwik.dev/router/middleware/request-handler',
        '@rari/core',
        '@rari/core/router',
        'vite',
      ],
    },
  },
})
