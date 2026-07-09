import path from 'node:path'
import { defineConfig } from 'vite-plus'

export default defineConfig({
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
      'runtime/call-server': 'src/runtime/call-server.ts',
      'runtime/entry-client': 'src/runtime/entry-client.ts',
      'runtime/rsc-references': 'src/runtime/rsc-references.ts',
      'runtime/rsc-client-runtime': 'src/runtime/rsc-client-runtime.ts',
      'runtime/AppRouterProvider': 'src/runtime/AppRouterProvider.tsx',
      'runtime/LoadingErrorBoundary': 'src/runtime/LoadingErrorBoundary.tsx',
      'runtime/ErrorBoundaryWrapper': 'src/runtime/ErrorBoundaryWrapper.tsx',
      'proxy/runtime-executor': 'src/proxy/runtime-executor.ts',
      'proxy/RariRequest': 'src/proxy/RariRequest.ts',
      'proxy/RariResponse': 'src/proxy/RariResponse.ts',
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
