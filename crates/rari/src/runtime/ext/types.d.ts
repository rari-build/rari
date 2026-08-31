/// <reference path="./deno-namespace.d.ts" />
/// <reference path="./deno-extensions.d.ts" />
/// <reference path="./extension-module-types.d.ts" />

declare global {
  // Deno websocket extension; not in lib.dom.
  class WebSocketStream {}

  interface GlobalThis {
    [K: string]: unknown
    Deno: typeof Deno
    RARI_STREAM_DEBUG?: boolean
    process?: {
      env: Record<string, string | undefined>
      cwd: () => string
      version: string
      versions: Record<string, string>
      platform: string
      arch: string
      argv: string[]
      execPath: string
      execArgv: string[]
      pid: number
      ppid: number
      nextTick?: (callback: () => void) => void
      [key: string]: any
    }
    Buffer?: any
    global?: typeof globalThis
    require?: {
      (specifier: string): any
      resolve: (specifier: string) => string
    }
  }

  var g: GlobalThis
}

export {}
