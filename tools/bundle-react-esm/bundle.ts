/* eslint-disable no-console */
import { Buffer } from 'node:buffer'
import fs from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import { build } from 'rolldown'

const require = createRequire(import.meta.url)
const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url))
const ROOT = path.resolve(SCRIPT_DIR, '../..')
const OUT_DIR = path.join(ROOT, 'crates/rari/src/runtime/ext/rari/react/vendor')

function resolveReactCjs(pkg: string, cjsFile: string): string {
  const pkgJson = require.resolve(`${pkg}/package.json`)
  const pkgDir = path.dirname(pkgJson)
  return path.join(pkgDir, 'cjs', `${cjsFile}.production.js`)
}

interface BundleEntry {
  name: string
  cjsFile?: string
  source?: string
  namedExports?: string[]
  banner?: string
  externals?: Record<string, string>
  shimDescription?: string
}

/** Client-only react-dom exports stubbed for SSR module evaluation. */
const REACT_DOM_CLIENT_STUBS = ['createPortal'] as const

function createReactDomShimSource(): string {
  const stubExports = REACT_DOM_CLIENT_STUBS.map(name => `export function ${name}() {
  return null
}`).join('\n\n')

  const defaultFields = REACT_DOM_CLIENT_STUBS.join(',\n  ')

  return `/** Client-only APIs - safe no-ops during SSR module evaluation. */
${stubExports}

export default {
  ${defaultFields},
}`
}

const entries: BundleEntry[] = [
  {
    name: 'react',
    cjsFile: resolveReactCjs('react', 'react'),
    namedExports: [
      'Children',
      'Component',
      'Fragment',
      'Profiler',
      'PureComponent',
      'StrictMode',
      'Suspense',
      'cache',
      'cloneElement',
      'createContext',
      'createElement',
      'createRef',
      'forwardRef',
      'isValidElement',
      'lazy',
      'memo',
      'startTransition',
      'use',
      'useActionState',
      'useCallback',
      'useContext',
      'useDebugValue',
      'useDeferredValue',
      'useEffect',
      'useId',
      'useImperativeHandle',
      'useInsertionEffect',
      'useLayoutEffect',
      'useMemo',
      'useOptimistic',
      'useReducer',
      'useRef',
      'useState',
      'useSyncExternalStore',
      'useTransition',
      'version',
      // Internals required by react-dom / react-dom/server to share a single
      // React instance (hook dispatcher + shared internals live here).
      '__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE',
      '__COMPILER_RUNTIME',
    ],
  },
  {
    name: 'react-server',
    cjsFile: path.join(
      path.dirname(require.resolve('react/package.json')),
      'react.react-server.js',
    ),
    namedExports: [
      'Children',
      'Fragment',
      'Profiler',
      'StrictMode',
      'Suspense',
      'cache',
      'cloneElement',
      'createContext',
      'createElement',
      'createRef',
      'forwardRef',
      'isValidElement',
      'lazy',
      'memo',
      'startTransition',
      'use',
      'useId',
      'version',
      '__SERVER_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE',
    ],
  },
  {
    name: 'react-jsx-runtime',
    cjsFile: resolveReactCjs('react', 'react-jsx-runtime'),
    namedExports: ['Fragment', 'jsx', 'jsxs'],
    externals: { react: 'ext:rari/react/vendor/react.js' },
  },
  {
    name: 'react-dom-server',
    cjsFile: resolveReactCjs('react-dom', 'react-dom-server.browser'),
    namedExports: ['renderToReadableStream', 'renderToString', 'renderToStaticMarkup', 'resume', 'version'],
    // `react` is externalized to 'ext:rari/react/vendor/react.js' so the server renderer
    // and the client-component SSR bundles share ONE React instance. React's
    // hook dispatcher and context state are per-instance, so this is required
    // for real useState/useContext/createContext to work during SSR.
    // `react-dom` stays inlined (its own require('react') is also redirected
    // to the shared ext:rari/react/vendor/react.js by the external pattern below).
    externals: { react: 'ext:rari/react/vendor/react.js' },
  },
  {
    name: 'react-dom',
    source: createReactDomShimSource(),
    shimDescription: 'SSR shim for bare react-dom imports from client components',
  },
  {
    name: 'react-server-dom-webpack-client',
    cjsFile: resolveReactCjs('react-server-dom-webpack', 'react-server-dom-webpack-client.browser'),
    namedExports: ['createFromFetch', 'createFromReadableStream', 'encodeReply'],
    // Stub out webpack-specific module loading since we're bundling to a single ESM file
    banner: `
// Stub webpack's module loading system (not needed in our bundled ESM context)
globalThis.__rari_rsc_require__ = function(id) {
  // Module resolution is handled by our SSR module registry
  const ssrModules = globalThis['~rari']?.ssrModules || {};
  return ssrModules[id] || {};
};
globalThis.__rari_rsc_require__.u = function(chunkId) {
  // Chunk loading not needed - everything is bundled
  return '';
};
`,
    // Only externalize react - let react-dom be inlined
    // The Flight client needs React's internals. react-dom is optional in browser mode
    // and will be inlined with its required internals
    externals: {
      react: 'ext:rari/react/vendor/react.js',
    },
  },
  {
    name: 'react-server-dom-webpack-server',
    cjsFile: resolveReactCjs('react-server-dom-webpack', 'react-server-dom-webpack-server.browser'),
    namedExports: [
      'renderToReadableStream',
      'decodeReply',
      'decodeAction',
      'decodeFormState',
      'registerServerReference',
      'registerClientReference',
      'createClientModuleProxy',
    ],
    // Stub out webpack's chunk/module loading since we provide our own module registry
    banner: `
// Stub rari chunk/module loading (not needed in our bundled context)
globalThis.__rari_chunk_load__ = function(chunkId) {
  return Promise.resolve();
};
globalThis.__rari_rsc_require__ = function(id) {
  const ssrModules = globalThis['~rari']?.ssrModules || {};
  return ssrModules[id] || {};
};
`,
    // Use react-server build for Flight server (it needs the react-server export condition)
    externals: {
      react: 'ext:rari/react/vendor/react-server.js',
    },
  },
]

function createEntrySource(entry: BundleEntry): string {
  if (!entry.cjsFile)
    throw new Error(`Entry ${entry.name} is missing cjsFile`)

  const lines: string[] = []

  lines.push(`globalThis.process = globalThis.process || { env: { NODE_ENV: 'production' } };`)
  lines.push(`if (!globalThis.process.env) globalThis.process.env = {};`)
  lines.push(`globalThis.process.env.NODE_ENV = 'production';`)
  lines.push('')

  const importPath = entry.cjsFile.replace(/\\/g, '/')
  lines.push(`import * as __mod from '${importPath}';`)
  lines.push('')

  if (entry.namedExports) {
    const names = entry.namedExports.join(', ')
    lines.push(`const { ${names} } = __mod;`)
    lines.push(`export { ${names} };`)
  }
  else {
    lines.push(`export * from '${importPath}';`)
  }

  lines.push(`export default __mod;`)

  return lines.join('\n')
}

function createVendorHeader(entry: BundleEntry): string {
  const kind = entry.source
    ? (entry.shimDescription ?? 'ESM shim')
    : `Auto-generated ESM bundle of React ${entry.name} (production)`

  return [
    `/* eslint-disable eslint-comments/no-unlimited-disable */`,
    `/* eslint-disable */`,
    `// oxlint-disable`,
    `/**`,
    ` * ${kind}.`,
    ` * Source: react@19 / react-dom@19`,
    ` * Generated by: tools/bundle-react-esm/bundle.ts`,
    ` *`,
    ` * Do not edit manually. Re-generate with:`,
    ` *   just bundle-react-esm`,
    ` */`,
    '',
  ].join('\n')
}

function writeVendorBundle(entry: BundleEntry, body: string): void {
  const outPath = path.join(OUT_DIR, `${entry.name}.js`)
  const header = createVendorHeader(entry)
  const finalCode = entry.banner ? `${entry.banner}\n${body}` : body

  fs.writeFileSync(outPath, header + finalCode, 'utf-8')
  const sizeKb = (Buffer.byteLength(finalCode) / 1024).toFixed(1)
  console.log(`  ${entry.name}.js (${sizeKb} KB)`)
}

async function bundleShimEntry(entry: BundleEntry): Promise<void> {
  if (!entry.source)
    throw new Error(`Entry ${entry.name} is missing source`)

  writeVendorBundle(entry, entry.source)
}

async function bundleCjsEntry(entry: BundleEntry): Promise<void> {
  const virtualId = `\0virtual:${entry.name}`
  const entrySource = createEntrySource(entry)

  const externalPatterns: (string | RegExp)[] = [/^node:/]

  const resolveOverrides: Record<string, string> = {}
  if (entry.externals) {
    for (const [pkg, target] of Object.entries(entry.externals)) {
      externalPatterns.push(new RegExp(`^${pkg}$`), new RegExp(`^${pkg}/`))
      resolveOverrides[pkg] = target
    }
  }

  const result = await build({
    input: virtualId,
    platform: 'neutral',
    write: false,
    external: externalPatterns,
    output: {
      format: 'esm',
      minify: false,
      exports: 'named',
    },
    resolve: {
      conditionNames: ['production', 'default'],
    },
    plugins: [
      {
        name: 'virtual-entry',
        resolveId(source) {
          if (source === virtualId)
            return source
          for (const [pkg, target] of Object.entries(resolveOverrides)) {
            if (source === pkg || source.startsWith(`${pkg}/`))
              return { id: target, external: true }
          }

          return null
        },
        load(id) {
          if (id === virtualId)
            return entrySource

          return null
        },
      },
    ],
  })

  const output = result.output[0]
  if (!output)
    throw new Error(`No output generated for ${entry.name}`)

  // Rolldown emits external CJS deps as `__require("pkg")` calls, which throw
  // in deno_core's V8 (no `require`). Rewrite them into a static ESM import of
  // the shared vendor module so the whole runtime resolves to ONE React
  // instance (file:///react_vendor/react.js).
  let code = output.code
  if (entry.externals) {
    const importLines: string[] = []
    for (const [pkg, target] of Object.entries(entry.externals)) {
      const ident = `__ext_${pkg.replace(/\W/g, '_')}`
      const escaped = pkg.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      const requireRe = new RegExp(`__require\\((["'])${escaped}\\1\\)`, 'g')
      if (requireRe.test(code)) {
        importLines.push(`import * as ${ident} from '${target}';`)
        code = code.replace(requireRe, ident)
      }
    }
    if (importLines.length > 0)
      code = `${importLines.join('\n')}\n${code}`
  }

  if (code.includes('__webpack_chunk_load__'))
    code = code.replaceAll('__webpack_chunk_load__', '__rari_chunk_load__')

  if (code.includes('__webpack_require__')) {
    if (code.includes('__webpack_require__.u'))
      code = code.replaceAll('__webpack_require__.u', '({}).u')
    code = code.replaceAll('__webpack_require__', '__rari_rsc_require__')
  }

  writeVendorBundle(entry, code)
}

async function bundleEntry(entry: BundleEntry): Promise<void> {
  if (entry.source)
    return bundleShimEntry(entry)

  return bundleCjsEntry(entry)
}

async function main(): Promise<void> {
  console.log('Bundling React CJS → ESM for rari...')
  console.log(`  Output: ${path.relative(ROOT, OUT_DIR)}/`)
  console.log('')

  fs.mkdirSync(OUT_DIR, { recursive: true })

  for (const entry of entries)
    await bundleEntry(entry)

  const indexLines = [
    `/* eslint-disable eslint-comments/no-unlimited-disable */`,
    `/* eslint-disable */`,
    `// oxlint-disable`,
    `/**`,
    ` * Re-exports for rari's React server vendor bundle.`,
    ` * Auto-generated - do not edit manually.`,
    ` */`,
    `export { default as React } from './react.js'`,
    `export { renderToReadableStream } from './react-dom-server.js'`,
    `export { jsx, jsxs, Fragment } from './react-jsx-runtime.js'`,
    `export { createFromReadableStream } from './react-server-dom-webpack-client.js'`,
    `export * as ReactServerRenderer from './react-server-dom-webpack-server.js'`,
    '',
  ]
  fs.writeFileSync(path.join(OUT_DIR, 'index.js'), indexLines.join('\n'), 'utf-8')

  console.log('')
  console.log('Done.')
}

main().catch((err) => {
  console.error('Bundle failed:', err)
  process.exit(1)
})
