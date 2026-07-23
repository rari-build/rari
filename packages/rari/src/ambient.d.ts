/// <reference types="vite-plus/client" />

interface ImportMetaEnv {
  readonly RARI_SERVER_URL?: string
  readonly VITE_RSC_PORT?: string
}

declare module 'virtual:react-flight-client' {
  export interface Thenable<T> extends Promise<T> {
    readonly status?: 'pending' | 'fulfilled' | 'rejected'
    readonly value?: T
    readonly reason?: unknown
  }

  export function createServerReference<A extends unknown[] = unknown[], R = unknown>(
    id: string,
    callServer: (id: string, args: A) => Promise<R>,
    encodeFormAction?: (args: A) => Promise<FormData | string>,
  ): unknown

  export function createFromReadableStream<T>(
    stream: ReadableStream<Uint8Array>,
    options?: Readonly<{
      readonly callServer?: (id: string, args: readonly unknown[]) => Promise<unknown>
      readonly moduleMap?: unknown
      readonly moduleLoading?: unknown
    }>,
  ): Thenable<T>

  export function createFromFetch<T>(
    promiseForResponse: Promise<Response>,
    options?: Readonly<{
      readonly callServer?: (id: string, args: readonly unknown[]) => Promise<unknown>
      readonly temporaryReferences?: Map<string, unknown>
    }>,
  ): Promise<T>

  export function createTemporaryReferenceSet(): Map<string, unknown>

  export function encodeReply(
    value: unknown,
    options?: Readonly<{
      readonly temporaryReferences?: Map<string, unknown>
      readonly signal?: AbortSignal
    }>,
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
  export function registerClientReference<T>(clientReference: T, id: string, exportName: string): T

  export function createClientModuleProxy(moduleId: string): unknown

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

  interface GlobalThis {
    '~rariExecuteProxy'?: (
      request: Readonly<{
        readonly url: string
        readonly method: string
        readonly headers: { readonly [key: string]: string }
      }>,
    ) => Promise<{
      continue: boolean
      redirect?: {
        destination: string
        permanent: boolean
      }
      rewrite?: string
      requestHeaders?: Record<string, string | string[]>
      responseHeaders?: Record<string, string | string[]>
      response?: {
        status: number
        headers: Record<string, string | string[]>
        body?: string
      }
    }>
  }
}
