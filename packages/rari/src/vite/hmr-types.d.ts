/// <reference types="rolldown-vite/client" />

interface ImportMetaEnv {
  readonly RARI_SERVER_URL?: string
  readonly VITE_RSC_PORT?: string
  readonly DEV: boolean
}

interface ImportMetaHot {
  on: <T = any>(event: string, callback: (data: T) => void) => void
  off: <T = any>(event: string, callback: (data: T) => void) => void
  send: <T = any>(event: string, data?: T) => void
  dispose: (callback: () => void) => void
  accept: (() => void) & ((callback: (mod: any) => void) => void) & ((deps: string[], callback: (mods: any[]) => void) => void)
  decline: () => void
  invalidate: () => void
  data: any
}

interface ImportMeta {
  readonly hot?: ImportMetaHot
  readonly env: ImportMetaEnv
}
