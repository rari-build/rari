/// <reference types="vite-plus/client" />

declare module 'virtual:react-flight-client' {
  import type { ReactPromise } from 'react'

  export type Thenable<T> = ReactPromise<T> & Promise<T>

  export function createServerReference<A extends unknown[] = unknown[], R = unknown>(
    id: string,
    callServer?: (id: string, args: A) => Promise<R>,
    encodeFormAction?: (args: A) => Promise<FormData | string>,
    findSourceMapURL?: (filename: string, environmentName: string) => string | null,
    functionName?: string,
  ): unknown

  export function createFromReadableStream<T>(
    stream: ReadableStream<Uint8Array>,
    options?: Readonly<{
      readonly callServer?: (id: string, args: readonly unknown[]) => Promise<unknown>
      readonly moduleMap?: unknown
      readonly moduleLoading?: unknown
      readonly findSourceMapURL?: (filename: string, environmentName: string) => string | null
    }>,
  ): Thenable<T>

  export function createFromFetch<T>(
    promiseForResponse: Promise<Response>,
    options?: Readonly<{
      readonly callServer?: (id: string, args: readonly unknown[]) => Promise<unknown>
      readonly temporaryReferences?: Map<string, unknown>
      readonly findSourceMapURL?: (filename: string, environmentName: string) => string | null
    }>,
  ): Thenable<T>

  export function createTemporaryReferenceSet(): Map<string, unknown>

  export function encodeReply(
    value: unknown,
    options?: Readonly<{
      readonly temporaryReferences?: Map<string, unknown>
      readonly signal?: AbortSignal
    }>,
  ): Promise<FormData | string>
}

declare module 'virtual:client-router' {
  import type { ReactNode } from 'react'

  export interface ClientRouterProps {
    readonly children: ReactNode
    readonly initialRoute: string
  }

  export function ClientRouter(props: ClientRouterProps): ReactNode
}

declare module 'virtual:app-router-provider' {
  import type { ReactNode } from 'react'

  export interface AppRouterProviderProps {
    readonly children?: ReactNode
    readonly initialPayload?: {
      readonly element: unknown
      readonly flightProtocol?: string
    }
    readonly onNavigate?: (detail: Readonly<Record<string, unknown>>) => void
  }

  export function AppRouterProvider(props: AppRouterProviderProps): ReactNode
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
