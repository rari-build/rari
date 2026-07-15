/**
 * Residual `node:console` statically imports Deno's
 * `ext:runtime/98_global_scope_shared.js`. rari filters Deno's real shared
 * module and registers this file under that specifier instead so console can
 * read `globalThis.console` (installed by init_console).
 *
 * Keep this file type-annotation-free: the extension specifier ends in `.js`.
 */
export const windowOrWorkerGlobalScope = {
  console: {
    get() {
      return globalThis.console
    },
  },
}

export const unstableForWindowOrWorkerGlobalScope = {}
