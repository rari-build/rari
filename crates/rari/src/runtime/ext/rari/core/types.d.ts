/// <reference path="../../types.d.ts" />

declare global {
  interface GlobalThis {
    '~rsc'?: {
      modules?: Record<string, { default?: unknown, [key: string]: unknown }>
      functions?: Record<string, unknown>
      keyCounter?: number
      renderGeneration?: number
      renderResult?: unknown
    }
    '~render'?: {
      lastResult?: unknown
      currentComponent?: string
    }
    '~suspense'?: {
      streaming?: boolean
      promises?: Record<string, unknown>
      boundaryProps?: Record<string, unknown>
      discoveredBoundaries?: unknown[]
      pendingPromises?: unknown[]
      pendingPromisesById?: Record<string, unknown>
      currentBoundaryId?: string | null
      renderGeneration?: number
      SAFE_PROPS?: Set<string>
      isSafeProp?: (key: string) => boolean
      safeSerializeElement?: (element: unknown) => unknown
    }
    '~reactServer'?: {
      renderToReadableStream: (element: unknown, options?: { onError?: (error: unknown) => void }) => Promise<ReadableStream>
    }
    '~flightClient'?: {
      createFromReadableStream: (stream: ReadableStream, options?: { ssrManifest?: unknown }) => Promise<unknown>
    }
    '~reactServerRenderer'?: {
      renderToReadableStream: (element: unknown, bundlerConfig: unknown, options?: { onError?: (error: unknown) => void }) => Promise<ReadableStream>
    }
    '~promises'?: {
      currentObject?: unknown
      resolvedValue?: unknown
      resolutionComplete?: boolean
    }
    '~clientComponents'?: Record<string, { id: string, path: string, type: 'client', component: any, registered: boolean }>
    '~clientComponentNames'?: Record<string, string>
    '~clientComponentPaths'?: Record<string, string>
    'registerClientComponent'?: (componentId: string, componentPath: string, component?: any) => void
    'isClientComponent'?: (componentType: any, registry?: Record<string, any>) => boolean
    'getClientComponentInfo'?: (componentType: any) => { id: string, path: string, type: 'client', component: any, registered: boolean } | null
    'getClientComponentId'?: (componentType: any) => string | null
    'listClientComponents'?: () => Record<string, any>
    'listClientComponentNames'?: () => Record<string, string>
    'clearClientComponents'?: () => void
    'registerClientComponentFromModule'?: (componentPath: string, moduleExports: any) => void
    'markAsClientComponent'?: (component: any, componentId?: string) => void
    'createClientReference'?: (componentId: string, componentPath: string) => any
    '~serverFunctions'?: {
      registered?: Set<string>
      exported?: Record<string, unknown>
      all?: Record<string, unknown>
    }
    'getServerFunction'?: (name: string) => ((...args: unknown[]) => Promise<unknown>) | null
    'renderToRsc'?: (element: unknown) => Promise<string>
    'renderToHtmlFizz'?: (element: unknown) => Promise<string>
    'React'?: {
      createElement: (component: unknown, props: unknown, ...children: unknown[]) => unknown
      Fragment: symbol
      Suspense: symbol
      use: <T>(usable: T | Promise<T>) => T
      cache: <T extends (...args: any[]) => any>(fn: T) => T
    }
    'resolveServerFunctionsForComponent'?: (componentId?: string) => Promise<unknown>
    'clearServerFunctionCache'?: () => void
    'isServerFunctionRegistered'?: (functionName: string) => boolean
    'registerModule'?: (moduleKeyOrModule: string | any, moduleNameOrMainExport: string | any, exportedFunctions?: Record<string, (...args: any[]) => any>) => { success: boolean, exportCount: number }
    'executeServerFunction'?: (functionName: string, args?: any[]) => Promise<any>
    'createEnhancedServerFunctionPromise'?: (functionName: string, args?: any[], options?: { componentId?: string }) => Promise<any>
    'discoverModuleExports'?: (code: string) => string[]
    'createServerFunctionPromise'?: (functionName: string, args?: any[]) => Promise<any>
    'createLoaderStub'?: (componentId: string) => string
    'createComponentStub'?: (componentName: string) => string
    'RscModuleManager'?: {
      register: (moduleKeyOrModule: string | any, moduleNameOrMainExport: string | any, exportedFunctions?: Record<string, (...args: any[]) => any>) => { success: boolean, exportCount: number }
      getFunction: (name: string) => ((...args: any[]) => any) | null
      createPromise: (functionName: string, args?: any[]) => Promise<any>
      discoverExports: (code: string) => string[]
      stubs: {
        loader: (componentId: string) => string
        component: (componentName: string) => string
      }
    }
    'ServerFunctions'?: {
      resolve: (componentId?: string) => Promise<unknown>
      execute: (functionName: string, args?: any[]) => Promise<any>
      createPromise: (functionName: string, args?: any[], options?: { componentId?: string }) => Promise<any>
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
      ssrModules?: Record<string, { default?: unknown, [key: string]: unknown }>
      clientReferenceManifest?: Record<string, { id: string, chunks: string, name: string }>
      lastRscBinary?: Uint8Array
      capturedElement?: unknown
      exportOwners?: Record<string, string>
      metadataCollector?: {
        collect: (layoutPaths: string[], pagePath: string, params: Record<string, string>, searchParams: Record<string, string>) => Promise<unknown[]>
      }
      componentLoader?: {
        registerComponent: (moduleSpecifier: string, componentId: string, skipGlobalBinding?: boolean) => Promise<unknown>
      }
      cookies?: () => unknown
      headers?: () => unknown
      pageCacheTags?: Set<string>
      useCacheBuildId?: string
      useCacheDynamicDepth?: number
      markUseCacheDynamic?: () => void
      invalidateUseCache?: (input: { tag?: string, path?: string }) => Promise<void>
      renderStreamingDocument?: (options: {
        capturedElement: unknown
        headContent: string
        caughtErrors: unknown[]
      }) => Promise<void>
      renderStaticDocument?: (options: {
        capturedElement: unknown
        headContent: string
        caughtErrors: unknown[]
      }) => Promise<string>
      pumpStreamingCompleteScript?: () => Promise<void>
      injectStreamError?: (caughtErrors: unknown[]) => Promise<void>
      pumpFizzChunk?: (text: string) => Promise<boolean>
      pumpRscElementStream?: (element: unknown, pumpChunk: (text: string) => Promise<boolean>) => Promise<void>
      streaming?: { complete?: boolean }
      loadFullReactVendors?: () => boolean
      loadRscReactVendors?: () => boolean
    }
  }

  namespace Deno {
    namespace core {
      namespace ops {
        function op_get_cookies(): string
        function op_get_request_headers(): string
        function op_set_cookie(options: {
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
        }): void
        function op_delete_cookie(name: string): void
        function op_cache_get(key: string): any
        function op_cache_set(key: string, value: any): void
      }
    }
  }
}

export {}
