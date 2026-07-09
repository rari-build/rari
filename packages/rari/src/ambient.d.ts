/// <reference types="vite-plus/client" />

interface ImportMetaEnv {
  readonly RARI_SERVER_URL?: string
  readonly VITE_RSC_PORT?: string
}

declare module 'virtual:react-flight-client' {
  export interface Thenable<T> extends Promise<T> {
    status?: 'pending' | 'fulfilled' | 'rejected'
    value?: T
    reason?: unknown
  }

  export function createServerReference<T, A extends unknown[] = unknown[], R = unknown>(
    id: string,
    callServer: (id: string, args: A) => Promise<R>,
    encodeFormAction?: (args: A) => Promise<FormData | string>,
  ): T

  export function createFromReadableStream<T>(
    stream: ReadableStream<Uint8Array>,
    options?: {
      callServer?: (id: string, args: unknown[]) => Promise<unknown>
      moduleMap?: unknown
      moduleLoading?: unknown
    },
  ): Thenable<T>

  export function createFromFetch<T>(
    promiseForResponse: Promise<Response>,
    options?: {
      callServer?: (id: string, args: unknown[]) => Promise<unknown>
      temporaryReferences?: Map<string, unknown>
    },
  ): Promise<T>

  export function createTemporaryReferenceSet(): Map<string, unknown>

  export function encodeReply(
    value: unknown,
    options?: {
      temporaryReferences?: Map<string, unknown>
      signal?: AbortSignal
    },
  ): Promise<FormData | string>
}

declare module 'react-server-dom-webpack/client' {
  export type { Thenable } from 'virtual:react-flight-client'
  export {
    createFromReadableStream,
    createServerReference,
    encodeReply,
  } from 'virtual:react-flight-client'
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

declare global {
  interface RequestInit {
    rari?: {
      revalidate?: number | false
      tags?: string[]
      timeout?: number
    }
  }
}
