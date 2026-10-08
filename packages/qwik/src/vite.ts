import type { Plugin, PluginOption, ResolvedConfig } from 'vite'
import type { BuildQwikServerOptions, QwikExperimentalFeature } from './build'
import path from 'node:path'
import { qwikVite } from '@qwik.dev/core/optimizer'
import { qwikRouter } from '@qwik.dev/router/vite'
import { buildQwikServer } from './build'

export interface RariQwikOptions {
  /** Source directory containing the Qwik app. Defaults to `src`. */
  readonly srcDir?: string
  /** Routes directory. Defaults to `<srcDir>/routes`. */
  readonly routesDir?: string
  /** The app's SSR entry. Defaults to `<srcDir>/entry.ssr`. */
  readonly entrySsr?: string
  /** Qwik optimizer experimental features (e.g. `['pendingBoundary']`). */
  readonly experimental?: readonly QwikExperimentalFeature[]
}

/**
 * The Vite plugin for a Qwik app hosted by rari.
 *
 * `vite dev` runs Qwik's own dev server (Qwik Router's dev middleware), so
 * authoring works exactly as in any Qwik project. `vite build` (what `rari
 * build` runs) produces the client output, then the rari server bundle, route
 * manifest and server config, so `rari start` serves the app from the Rust
 * host with no further configuration.
 */
export function rariQwik(options: RariQwikOptions = {}): PluginOption[] {
  const srcDir = options.srcDir ?? 'src'
  const experimental = [...(options.experimental ?? [])]
  const routesDir = options.routesDir
  const entrySsr = options.entrySsr
  let resolved: ResolvedConfig | undefined

  // Qwik's plugin factories are typed `any`; pin them to Vite's plugin type.
  /* oxlint-disable typescript/no-unsafe-assignment */
  const router: PluginOption = qwikRouter(routesDir == null ? {} : { routesDir })
  const optimizer: PluginOption = qwikVite({
    srcDir,
    experimental,
    client: { outDir: 'dist/client' },
  })
  /* oxlint-enable typescript/no-unsafe-assignment */
  const serverBuild: Plugin = {
    name: 'rari-qwik',
    apply: 'build',
    configResolved(config) {
      resolved = config
    },
    async closeBundle() {
      // Only the client pass runs through the app config; the server pass is
      // our own nested build (buildQwikServer runs it with configFile: false).
      if (resolved?.build.ssr !== false) return
      const root = resolved.root
      const serverOptions: BuildQwikServerOptions = {
        root,
        srcDir,
        routesDir: routesDir == null ? undefined : path.resolve(root, routesDir),
        entrySsr: entrySsr == null ? undefined : path.resolve(root, entrySsr),
        outDir: 'dist',
        experimental,
      }
      const result = await buildQwikServer(serverOptions)
      resolved.logger.info(`[rari/qwik] server entry: ${path.relative(root, result.serverEntry)}`)
    },
  }

  return [router, optimizer, serverBuild]
}
