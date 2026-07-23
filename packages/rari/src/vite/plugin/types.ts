import type { Plugin as VitePlugin } from 'vite-plus'

/**
 * Public plugin shape returned by rari helpers.
 *
 * Vite and vite-plus publish separate Plugin types that are structurally
 * identical at runtime but nominally incompatible in TypeScript. Keeping the
 * exported surface minimal avoids TS2321 when mixing rari with either toolchain.
 */
export interface RariPlugin {
  name: string
  enforce?: 'pre' | 'post'
}

export function toRariPlugins(plugins: readonly VitePlugin[]): RariPlugin[] {
  return plugins.map((plugin): RariPlugin => plugin)
}

export function toRariPlugin(plugin: VitePlugin): RariPlugin {
  return plugin
}
