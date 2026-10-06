import type { ExperimentalFeatures } from '@qwik.dev/core/optimizer'
import type { AppRouteManifest, RouteConventions } from '@rari/core/router'
import type { Plugin } from 'vite'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { qwikVite } from '@qwik.dev/core/optimizer'
import { qwikRouter } from '@qwik.dev/router/vite'
import { generateAppRouteManifest } from '@rari/core/router'
import { build } from 'vite'

/** Where the server bundle lands, relative to the project root. The Rust host expects this path. */
export const SERVER_ENTRY_FILE = 'qwik-server-entry.mjs'

/** Qwik optimizer experimental feature flags (e.g. `pendingBoundary`). */
export type QwikExperimentalFeature = keyof typeof ExperimentalFeatures

/**
 * Qwik Router's file conventions mapped onto rari's scanner: only the base
 * names differ. Qwik treats `index` and `layout` files (plus `404`) as special;
 * the Next-style roles rari knows are disabled so a Qwik route file named e.g.
 * `error.tsx` is not mistaken for an error boundary. `index@name`, `index!`,
 * `layout-name` and `layout!` are rari's own route-file grammar and need no
 * mapping.
 */
export const QWIK_ROUTE_CONVENTIONS: RouteConventions = {
  page: 'index',
  layout: 'layout',
  notFound: '404',
  loading: '',
  error: '',
  template: '',
  route: '',
  ogImage: '',
}

/** Qwik Router page/endpoint/layout file extensions (`index.md` is a page too). */
export const QWIK_ROUTE_EXTENSIONS = ['.tsx', '.ts', '.jsx', '.js', '.mdx', '.md'] as const

export interface BuildQwikServerOptions {
  /** Project root. Defaults to cwd. */
  readonly root?: string
  /** Source dir. Defaults to `src`. */
  readonly srcDir?: string
  /** Routes dir. Defaults to `<srcDir>/routes`. */
  readonly routesDir?: string
  /** The app's SSR entry (exports the `createRenderer` result). Defaults to `<srcDir>/entry.ssr`. */
  readonly entrySsr?: string
  /** Output dir. Defaults to `dist`. */
  readonly outDir?: string
  /** Qwik optimizer experimental features (e.g. `['pendingBoundary']`). */
  readonly experimental?: readonly QwikExperimentalFeature[]
}

export type BuildQwikOptions = BuildQwikServerOptions

export interface BuildQwikResult {
  /** Self-contained SSR bundle that installs `globalThis.__rariQwikHandle`. */
  readonly serverEntry: string
  /** Client output dir (qwikloader + segment bundles), served by rari. */
  readonly clientDir: string
  /** rari route manifest written to `dist/server/routes.json`. */
  readonly routeManifest: AppRouteManifest
}

const ENTRY_ID = 'virtual:rari-qwik-entry'
const RESOLVED_ENTRY_ID = `\0${ENTRY_ID}`

interface ResolvedPaths {
  root: string
  srcDir: string
  routesDir: string
  entrySsr: string
  outDir: string
  clientDir: string
  serverDir: string
  experimental: QwikExperimentalFeature[]
}

function resolvePaths(options: BuildQwikServerOptions): ResolvedPaths {
  const root = path.resolve(options.root ?? process.cwd())
  const srcDir = options.srcDir ?? 'src'
  const outDir = options.outDir ?? 'dist'
  return {
    root,
    srcDir,
    routesDir: path.resolve(root, options.routesDir ?? path.join(srcDir, 'routes')),
    entrySsr: path.resolve(root, options.entrySsr ?? path.join(srcDir, 'entry.ssr')),
    outDir,
    clientDir: path.resolve(root, outDir, 'client'),
    serverDir: path.resolve(root, outDir, 'server'),
    experimental: [...(options.experimental ?? [])],
  }
}

/**
 * The server entry rari loads: registers the generated router config, wires
 * the client manifest, and installs the request handler with the app's own
 * `entry.ssr` renderer. Generated as a virtual module so nothing is written
 * into the app's source tree.
 */
function serverEntryPlugin(entrySsr: string): Plugin {
  return {
    name: 'rari-qwik-server-entry',
    enforce: 'post',
    configResolved(config) {
      // qwikRouter adds `@qwik-router-config` as a second SSR input (its
      // adapters emit multi-file servers). rari loads one self-contained
      // module, and our entry already imports the config, so keep the single
      // input; that also lets rolldown inline the lazy route modules.
      const single = { input: ENTRY_ID }
      Object.assign(config.build.rolldownOptions, single)
      Object.assign(config.environments.ssr.build.rolldownOptions, single)
    },
    resolveId(source) {
      if (source === ENTRY_ID) return RESOLVED_ENTRY_ID
      return null
    },
    load(id) {
      if (id !== RESOLVED_ENTRY_ID) return null
      return [
        // Importing the config registers it with the router (`_setRouterConfig`).
        `import '@qwik-router-config'`,
        `import { manifest } from '@qwik-client-manifest'`,
        `import { installRariQwikHandler } from '@rari/qwik/platform'`,
        `import render from ${JSON.stringify(entrySsr)}`,
        `installRariQwikHandler({ render, manifest })`,
      ].join('\n')
    },
  }
}

/**
 * qwikVite removes Vite's `vite:build-import-analysis` plugin (Qwik ships its own
 * preloader). On rolldown-based Vite the import analysis is split, so removing it
 * by name strips the pass that *replaces* `__VITE_PRELOAD__` while the pass that
 * *emits* it stays, leaving the literal tokens in the client bundles (a
 * `ReferenceError` at runtime that kills resumption). Replace them with inert
 * values; Qwik's own preloader still drives real preloading.
 */
function replaceVitePreloadTokens(): Plugin {
  return {
    name: 'rari-qwik-replace-vite-preload-tokens',
    enforce: 'post',
    renderChunk(code) {
      if (!code.includes('__VITE_')) return null
      return {
        code: code
          .replaceAll('__VITE_PRELOAD__', 'void 0')
          .replaceAll('__VITE_IS_MODERN__', 'true')
          .replaceAll('__VITE_PRELOAD_BASE__', '""'),
        map: null,
      }
    },
  }
}

async function writeJson(file: string, value: unknown): Promise<void> {
  await mkdir(path.dirname(file), { recursive: true })
  await writeFile(file, `${JSON.stringify(value, null, 2)}\n`, 'utf8')
}

function isJsonRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

async function readJsonIfExists(file: string): Promise<Record<string, unknown>> {
  try {
    const parsed: unknown = JSON.parse(await readFile(file, 'utf8'))
    return isJsonRecord(parsed) ? parsed : {}
  } catch {
    return {}
  }
}

/**
 * Build the server side of a Qwik app for the rari host. Assumes the client
 * build already ran into `<outDir>/client` (qwikVite reads its manifest from
 * there). Produces the server bundle, rari's route manifest and the server
 * config that tells `rari start` which guest to load.
 */
export async function buildQwikServer(
  options: BuildQwikServerOptions = {},
): Promise<BuildQwikResult> {
  const paths = resolvePaths(options)

  await build({
    root: paths.root,
    // Self-contained: the app's vite.config.ts wires `rariQwik()` for the
    // client pass; loading it here would add a second optimizer instance.
    configFile: false,
    logLevel: 'warn',
    plugins: [
      qwikRouter({ routesDir: paths.routesDir }),
      qwikVite({
        srcDir: paths.srcDir,
        experimental: paths.experimental,
        client: { outDir: paths.clientDir },
        ssr: { input: ENTRY_ID, outDir: paths.serverDir },
      }),
      serverEntryPlugin(paths.entrySsr),
    ],
    // Bundle everything (Qwik, the router, this adapter) into one file: the
    // rari runtime evaluates the entry as a single module and does not walk
    // node_modules. Node built-ins (`node:async_hooks`) stay external; the host
    // provides them.
    ssr: { noExternal: true, target: 'node' },
    build: {
      ssr: ENTRY_ID,
      outDir: paths.serverDir,
      emptyOutDir: true,
      minify: false,
      rolldownOptions: {
        input: ENTRY_ID,
        output: {
          format: 'es',
          entryFileNames: SERVER_ENTRY_FILE,
          codeSplitting: false,
        },
      },
    },
  })

  // rari owns route matching for caching and 404s; feed its router the same
  // routes Qwik serves.
  const routeManifest = await generateAppRouteManifest(paths.routesDir, {
    conventions: QWIK_ROUTE_CONVENTIONS,
    extensions: QWIK_ROUTE_EXTENSIONS,
  })
  await writeJson(path.join(paths.serverDir, 'routes.json'), routeManifest)

  const configPath = path.join(paths.serverDir, 'config.json')
  const existing = await readJsonIfExists(configPath)
  await writeJson(configPath, { ...existing, framework: 'qwik' })

  return {
    serverEntry: path.join(paths.serverDir, SERVER_ENTRY_FILE),
    clientDir: paths.clientDir,
    routeManifest,
  }
}

/**
 * Full production build of a Qwik app for the rari host: the standard Qwik
 * client build, then {@link buildQwikServer}.
 */
export async function buildQwik(options: BuildQwikOptions = {}): Promise<BuildQwikResult> {
  const paths = resolvePaths(options)

  await build({
    root: paths.root,
    configFile: false,
    logLevel: 'warn',
    plugins: [
      qwikRouter({ routesDir: paths.routesDir }),
      qwikVite({
        srcDir: paths.srcDir,
        experimental: paths.experimental,
        client: { outDir: paths.clientDir },
      }),
      replaceVitePreloadTokens(),
    ],
    build: { outDir: paths.clientDir, emptyOutDir: true },
  })

  return buildQwikServer(options)
}
