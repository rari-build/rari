import { defineConfig } from 'vite-plus'
import { monorepoFmt, monorepoLint } from '../../.config/lint/monorepo'

// `rari` is the CLI (and the Rust binary's npm distribution) plus a
// backwards-compatible re-export of the React adapter, so existing apps keep
// importing from 'rari' while the implementation lives in @rari/react.
const REACT_SUBPATHS = [
  'image',
  'image/static',
  'font',
  'font/local',
  'font/google',
  'mdx',
  'mdx/define',
  'mdx/registry',
  'og',
  'router',
  'headers',
  'runtime/call-server',
  'runtime/action-flight-refresh',
  'runtime/merge-flight-refresh',
  'runtime/action-revalidation-kind',
  'runtime/entry-client',
  'runtime/rsc-references',
  'runtime/rsc-client-runtime',
  'runtime/AppRouterProvider',
  'runtime/ClientRouter',
  'runtime/ErrorBoundaryWrapper',
  'proxy/executor',
  'proxy/RariRequest',
  'proxy/RariResponse',
  'vite',
]

export default defineConfig({
  fmt: monorepoFmt,
  lint: monorepoLint,
  pack: {
    entry: {
      index: 'src/index.ts',
      cli: 'src/cli/index.ts',
      platform: 'src/platform.ts',
      ...Object.fromEntries(
        REACT_SUBPATHS.map(subpath => [
          subpath,
          `src/${subpath === 'image' || subpath === 'font' || subpath === 'mdx' ? `${subpath}/index` : subpath}.ts`,
        ]),
      ),
    },
    minify: true,
    deps: {
      neverBundle: [
        '@rari/react',
        ...REACT_SUBPATHS.map(subpath => `@rari/react/${subpath}`),
        '@rari/core',
        '@rari/core/router',
        '@rari/core/platform',
        '@rari/core/regex-constants',
        '@rari/core/utils/path',
        '@rari/core/utils/dist-paths',
        '@rari/core/utils/type-guards',
        '@rari/core/utils/xml',
        '@rari/core/utils/regexp',
      ],
    },
  },
})
