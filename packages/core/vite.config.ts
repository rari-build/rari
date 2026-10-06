import { defineConfig } from 'vite-plus'
import { monorepoFmt, monorepoLint } from '../../.config/lint/monorepo'

export default defineConfig({
  fmt: monorepoFmt,
  lint: monorepoLint,
  pack: {
    entry: {
      'index': 'src/index.ts',
      'router': 'src/router/index.ts',
      'platform': 'src/platform.ts',
      'guest': 'src/guest.ts',
      'regex-constants': 'src/regex-constants.ts',
      'utils/path': 'src/utils/path.ts',
      'utils/dist-paths': 'src/utils/dist-paths.ts',
      'utils/type-guards': 'src/utils/type-guards.ts',
      'utils/xml': 'src/utils/xml.ts',
      'utils/regexp': 'src/utils/regexp.ts',
    },
    minify: true,
  },
})
