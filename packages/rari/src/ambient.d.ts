/// <reference types="vite-plus/client" />

interface ImportMetaEnv {
  readonly RARI_SERVER_URL?: string
  readonly VITE_RSC_PORT?: string
}

declare module 'react-server-dom-webpack/server' {
  export function registerClientReference<T>(
    clientReference: T,
    id: string,
    exportName: string,
  ): T

  export function createClientModuleProxy<T>(moduleId: string): T

  export function registerServerReference<T>(
    serverReference: T,
    id: string,
    exportName: string | null,
  ): T
}

declare module 'virtual:react-flight-client' {
  export interface Thenable<T> extends Promise<T> {
    status?: 'pending' | 'fulfilled' | 'rejected'
    value?: T
    reason?: unknown
  }

  export function createFromReadableStream<T>(
    stream: ReadableStream<Uint8Array>,
    options?: {
      callServer?: <A, T>(id: string, args: A) => Promise<T>
      moduleMap?: unknown
      moduleLoading?: unknown
    },
  ): Thenable<T>
}
