import type { Plugin } from 'vite-plus'
import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import { rari } from 'rari/vite'
import { defineConfig } from 'vite-plus'
import { monorepoFmt, monorepoLint } from '../.config/lint/monorepo'
import { siteUrl } from './src/lib/site'

const MERMAID_EXCLUDED = new Set([
  'katex',
  'cytoscape',
  'cytoscape-cose-bilkent',
  'cytoscape-fcose',
])
const MERMAID_KEEP = [
  'chunk-',
  'flowDiagram-',
  'sequenceDiagram-',
  'dagre-',
  'elk-',
  'sizeCapture-',
] as const
const MERMAID_STUB = '\0mermaid-stub:'

function mermaidStubPlugin(): Plugin {
  return {
    name: 'mermaid-stub',
    enforce: 'pre',
    resolveId(id) {
      if (MERMAID_EXCLUDED.has(id) || id.startsWith('katex/')) {
        return { id: MERMAID_STUB + id, moduleSideEffects: false }
      }
      return null
    },
    load(id) {
      if (id.startsWith(MERMAID_STUB)) {
        const name = id.slice(MERMAID_STUB.length)
        return [
          `const e = () => { throw new Error("[mermaid-stub] '${name}' is excluded from this build"); };`,
          'const stub = new Proxy(e, { get: () => e });',
          'export default stub;',
          'export const use = e;',
        ].join('\n')
      }

      const normalized = id.replaceAll('\\', '/')
      if (!normalized.includes('/mermaid/dist/chunks/mermaid.core/') || id.endsWith('.map')) {
        return null
      }

      const name = (normalized.split('/').pop() ?? id).replace(/\.map$/, '')
      if (MERMAID_KEEP.some(prefix => name.startsWith(prefix))) return null

      return `throw new Error(${JSON.stringify(
        `Mermaid diagram "${name}" is not included in this build. Add its prefix to MERMAID_KEEP in vite.config.ts if needed.`,
      )})
`
    },
  }
}

export default defineConfig({
  fmt: monorepoFmt,
  lint: monorepoLint,
  plugins: [
    mermaidStubPlugin(),
    rari({
      compiler: true,
      origin: siteUrl,
      csp: {
        scriptSrc: [
          "'self'",
          "'unsafe-inline'",
          'https://t.rari.build',
          'https://js.sentry-cdn.com',
        ],
        connectSrc: [
          "'self'",
          'ws:',
          'wss:',
          'https://t.rari.build',
          'https://*.ingest.us.sentry.io',
        ],
        workerSrc: ["'self'", 'blob:', 'https://t.rari.build'],
      },
      cacheControl: {
        routes: {
          '/*': 'public, max-age=7200, stale-while-revalidate=86400',
        },
      },
      cache: {
        maxBytes: 24 * 1024 * 1024,
        layers: {
          response: { maxEntries: 40 },
          layout: { maxEntries: 40 },
          image: { maxEntries: 20 },
          og: { maxEntries: 20 },
          fetch: { maxEntries: 32 },
        },
      },
    }),
    tailwindcss(),
  ],
  build: {
    chunkSizeWarningLimit: 2000,
    rolldownOptions: {
      output: {
        codeSplitting: {
          includeDependenciesRecursively: false,
          minShareCount: 10,
          groups: [
            {
              name: moduleId => {
                if (moduleId.includes('node_modules')) {
                  if (moduleId.includes('posthog')) return 'posthog'
                  if (moduleId.includes('@sentry')) return 'sentry'
                  if (moduleId.includes('react-dom')) return 'react-dom'
                }

                return null
              },
            },
          ],
        },
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, 'src'),
    },
  },
})
