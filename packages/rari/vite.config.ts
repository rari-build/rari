import path from 'node:path'
import { fmt, lint } from '@rari/lint/vite'
import { defineConfig } from 'vite-plus'

export default defineConfig({
  fmt,
  lint,
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, 'src'),
    },
  },
  pack: {
    entry: {
      'index': 'src/index.ts',
      'router': 'src/router/index.ts',
      'vite': 'src/vite/index.ts',
      'cli': 'src/cli/index.ts',
      'platform': 'src/cli/platform.ts',
      'image': 'src/image/index.ts',
      'og': 'src/og/index.ts',
      'mdx': 'src/mdx/index.ts',
      'mdx/define': 'src/mdx/define.ts',
      'mdx/registry': 'src/mdx/registry.ts',
      'headers': 'src/headers.ts',
      'runtime/call-server': 'src/runtime/actions/call-server.ts',
      'runtime/action-flight-refresh': 'src/runtime/actions/flight-refresh.ts',
      'runtime/merge-flight-refresh': 'src/runtime/flight/merge-refresh.ts',
      'runtime/flight-route-cache': 'src/runtime/flight/route-cache.ts',
      'runtime/flight-router-state': 'src/runtime/flight/router-state.ts',
      'runtime/action-revalidation-kind': 'src/runtime/actions/revalidation-kind.ts',
      'runtime/entry-client': 'src/runtime/entry-client.ts',
      'runtime/rsc-references': 'src/runtime/rsc/references.ts',
      'runtime/rsc-client-runtime': 'src/runtime/rsc/client-runtime.ts',
      'runtime/AppRouterProvider': 'src/runtime/flight/app-router-provider.tsx',
      'runtime/LoadingErrorBoundary': 'src/runtime/boundaries/loading-error-boundary.tsx',
      'runtime/ErrorBoundaryWrapper': 'src/runtime/boundaries/error-boundary-wrapper.tsx',
      'proxy/runtime-executor': 'src/proxy/runtime/runtime-executor.ts',
      'proxy/RariRequest': 'src/proxy/http/request.ts',
      'proxy/RariResponse': 'src/proxy/http/response.ts',
    },
    minify: true,
    deps: {
      neverBundle: [
        '@mdx-js/mdx',
        'lightningcss',
        'vite',
        'vite-plus',
        'rolldown',
        'react-server-dom-webpack',
        'react-server-dom-webpack/client',
        'react-server-dom-webpack/server',
        'virtual:app-router-provider',
        'virtual:app-router-provider.tsx',
        'virtual:react-flight-client',
        'virtual:react-flight-client.ts',
        'virtual:rsc-integration.ts',
        'rari/router',
        'rari/mdx/registry',
        'rari/mdx/define',
      ],
    },
  },
})
