/* oxlint-disable typescript/prefer-readonly-parameter-types -- stream APIs intentionally take mutable error buffers */
/// <reference path="../../types.d.ts" />

declare global {
  interface GlobalThis {
    '~rsc'?: {
      modules?: Record<string, { default?: unknown; [key: string]: unknown }>
      functions?: Record<string, unknown>
      renderResult?: unknown
    }
    '~render'?: {
      lastResult?: unknown
      currentComponent?: string
    }
    '~suspense'?: {
      discoveredBoundaries?: unknown[]
      pendingPromises?: unknown[]
      promises?: Record<string, unknown>
      currentBoundaryId?: string | null
    }
    '~reactServer'?: {
      renderToReadableStream: (
        element: unknown,
        options?: Readonly<{ onError?: (error: unknown) => void }>,
      ) => Promise<ReadableStream<Uint8Array>>
    }
    '~flightClient'?: {
      createFromReadableStream: (
        stream: ReadableStream,
        options?: Readonly<{ ssrManifest?: unknown }>,
      ) => Promise<unknown>
    }
    '~reactServerRenderer'?: {
      renderToReadableStream: (
        element: unknown,
        bundlerConfig: unknown,
        options?: Readonly<{ formState?: unknown; onError?: (error: unknown) => void }>,
      ) => Promise<ReadableStream<Uint8Array>>
      decodeAction?: (
        body: FormData,
        serverManifest: Readonly<{
          readonly [key: string]: Readonly<{
            readonly id: string
            readonly name?: string
            readonly chunks: readonly string[]
          }>
        }>,
      ) => Promise<(() => Promise<unknown>) | null>
      decodeFormState?: (
        actionResult: unknown,
        body: FormData,
        serverManifest: Readonly<{
          readonly [key: string]: Readonly<{
            readonly id: string
            readonly name?: string
            readonly chunks: readonly string[]
          }>
        }>,
      ) => Promise<unknown>
      decodeReply?: (
        body: string | FormData,
        serverManifest: Readonly<{
          readonly [key: string]: Readonly<{
            readonly id: string
            readonly name?: string
            readonly chunks: readonly string[]
          }>
        }>,
      ) => Promise<unknown>
    }
    '~promises'?: {
      currentObject?: unknown
      resolvedValue?: unknown
      resolutionComplete?: boolean
    }
    '~clientComponents'?: Record<
      string,
      { id: string; path: string; type: 'client'; component: unknown; registered: boolean }
    >
    '~clientComponentNames'?: Record<string, string>
    '~clientComponentPaths'?: Record<string, string>
    'registerClientComponent'?: (
      componentId: string,
      componentPath: string,
      component?: unknown,
    ) => void
    'isClientComponent'?: (
      componentType: unknown,
      registry?: {
        readonly [key: string]: Readonly<{
          readonly id: string
          readonly path: string
          readonly type: 'client'
          readonly component: unknown
          readonly registered: boolean
        }>
      },
    ) => boolean
    'getClientComponentInfo'?: (componentType: unknown) => {
      id: string
      path: string
      type: 'client'
      component: unknown
      registered: boolean
    } | null
    'getClientComponentId'?: (componentType: unknown) => string | null
    'listClientComponents'?: () => Record<
      string,
      { id: string; path: string; type: 'client'; component: unknown; registered: boolean }
    >
    'listClientComponentNames'?: () => Record<string, string>
    'clearClientComponents'?: () => void
    'registerClientComponentFromModule'?: (componentPath: string, moduleExports: unknown) => void
    'markAsClientComponent'?: (component: unknown, componentId?: string) => void
    'createClientReference'?: (componentId: string, componentPath: string) => unknown
    'getServerFunction'?: (
      name: string,
    ) => ((...args: readonly unknown[]) => Promise<unknown>) | null
    'renderToRsc'?: (element: unknown) => Promise<string>
    'renderToHtmlFizz'?: (element: unknown) => Promise<string>
    'React'?: {
      createElement: (
        component: unknown,
        props: unknown,
        ...children: readonly unknown[]
      ) => unknown
      Fragment: symbol
      Suspense: symbol
      use: <T>(usable: T | Promise<T>) => T
      cache: <T extends (...args: readonly any[]) => any>(fn: T) => T
    }
    'resolveServerFunctionsForComponent'?: (componentId?: string) => Promise<unknown>
    'clearServerFunctionCache'?: () => void
    'isServerFunctionRegistered'?: (functionName: string) => boolean
    'registerModule'?: (
      moduleKeyOrModule: any,
      moduleNameOrMainExport: any,
      exportedFunctions?: Readonly<{ readonly [key: string]: (...args: readonly any[]) => any }>,
    ) => { success: boolean; exportCount: number }
    'executeServerFunction'?: (functionName: string, args?: readonly any[]) => Promise<any>
    'createEnhancedServerFunctionPromise'?: (
      functionName: string,
      args?: readonly any[],
    ) => Promise<any>
    'discoverModuleExports'?: (code: string) => string[]
    'createServerFunctionPromise'?: (functionName: string, args?: readonly any[]) => Promise<any>
    'createLoaderStub'?: (componentId: string) => string
    'createComponentStub'?: (componentName: string) => string
    'RscModuleManager'?: {
      register: (
        moduleKeyOrModule: any,
        moduleNameOrMainExport: any,
        exportedFunctions?: Readonly<{ readonly [key: string]: (...args: readonly any[]) => any }>,
      ) => { success: boolean; exportCount: number }
      getFunction: (name: string) => ((...args: readonly any[]) => any) | null
      createPromise: (functionName: string, args?: readonly any[]) => Promise<any>
      discoverExports: (code: string) => string[]
      stubs: {
        loader: (componentId: string) => string
        component: (componentName: string) => string
      }
    }
    'ServerFunctions'?: {
      resolve: (componentId?: string) => Promise<unknown>
      execute: (functionName: string, args?: readonly any[]) => Promise<any>
      createPromise: (functionName: string, args?: readonly any[]) => Promise<any>
      isRegistered: (functionName: string) => boolean
      clear: () => void
    }
    '__RARI_DEV__'?: boolean
    '__rariInvalidateUseCache'?: (tag: string) => Promise<number>
    '__rariGetActiveUseCacheTags'?: () => string[]
    '~rari'?: {
      isDevelopment?: boolean
      apiHandler?: {
        callHandler: (requestData: any, moduleSpecifier: string, methodName: string) => Promise<any>
      }
      readStream?: (stream: ReadableStream) => Promise<string>
      ssrModules?: Partial<Record<string, { default?: unknown; [key: string]: unknown }>>
      serverManifest?: Partial<Record<string, { id: string; name?: string; chunks: string[] }>>
      registeredServerFunctions?: Set<string>
      clientReferenceManifest?: Record<string, { id: string; chunks: string; name: string }>
      lastRscBinary?: Uint8Array
      actionPostUrl?: string
      actionRefreshSearch?: string
      isActionRefreshCompose?: boolean
      actionRefreshElement?: unknown
      actionFormState?: unknown
      capturedElement?: unknown
      capturedByStream?: Record<string, unknown>
      pendingActionResult?: unknown
      exportOwners?: Record<string, string>
      metadataCollector?: {
        collect: (
          layoutPaths: readonly string[],
          pagePath: string,
          params: Readonly<Record<string, string>>,
          searchParams: Readonly<Record<string, string>>,
        ) => Promise<unknown[]>
      }
      componentLoader?: {
        registerComponent: (
          moduleSpecifier: string,
          componentId: string,
          skipGlobalBinding?: boolean,
        ) => Promise<unknown>
      }
      cookies?: () => unknown
      headers?: () => unknown
      pageCacheTags?: Set<string>
      useCacheBuildId?: string
      useCacheDynamicDepth?: number
      markUseCacheDynamic?: () => void
      invalidateUseCache?: (input: Readonly<{ tag?: string; path?: string }>) => Promise<void>
      requestStorage?: {
        run: <T>(
          store: Readonly<{ requestId: string; streamId?: string; capturedElement?: unknown }>,
          fn: () => T,
        ) => T
        getStore: () =>
          | { requestId?: string; streamId?: string; capturedElement?: unknown }
          | undefined
      }
      currentRequestId?: () => string
      renderStreamingDocument?: (
        options: Readonly<{
          readonly capturedElement: unknown
          readonly headContent: string
          readonly caughtErrors: unknown[]
          readonly streamId: string
        }>,
      ) => Promise<void>
      renderStaticDocument?: (
        options: Readonly<{
          readonly capturedElement: unknown
          readonly headContent: string
          readonly caughtErrors: unknown[]
          readonly streamId?: string
        }>,
      ) => Promise<string>
      injectStreamError?: (caughtErrors: unknown[], streamId: string) => Promise<void>
      pumpRscElementStream?: (
        element: unknown,
        pumpChunk: (text: string) => Promise<boolean>,
      ) => Promise<void>
      streaming?: { complete?: boolean }
      loadFullReactVendors?: () => boolean
      loadRscReactVendors?: () => boolean
    }
  }

  namespace Deno {
    namespace core {
      namespace ops {
        function op_get_cookies(requestId?: string): string
        function op_get_request_headers(requestId?: string): string
        function op_get_csp_nonce(requestId: string): string
        function op_set_cookie(
          options: Readonly<{
            name: string
            value: string
            path?: string
            domain?: string
            expires?: string
            maxAge?: number
            httpOnly?: boolean
            secure?: boolean
            sameSite?: 'strict' | 'lax' | 'none'
            priority?: 'low' | 'medium' | 'high'
            partitioned?: boolean
            requestId?: string
          }>,
        ): void
        function op_delete_cookie(name: string, requestId?: string): void
        function op_cache_get(key: string, requestId?: string): any
        function op_cache_set(key: string, value: any, requestId?: string): void
      }
    }
  }
}

export {}
