export interface ModuleData {
  id: string
  chunks: string[]
  name: string
}

export interface ComponentInfo {
  id: string
  path: string
  type: string
  component?: any
  registered: boolean
  loader?: () => Promise<any>
  loading?: boolean
  loadPromise?: Promise<any>
}

export interface GlobalWithRari {
  '~rari': {
    isDevelopment?: boolean
    AppRouterProvider?: any
    ClientRouter?: any
    getClientComponent?: (id: string) => any
    preloadClientComponent?: (id: string) => Promise<void>
    hydrateClientComponents?: (boundaryId: string, content: any, boundaryElement: Element) => void
    lazy?: {
      pending: Map<string, any>
      resolved: Map<string, any>
      counter: number
      resolve: (promiseId: string) => Promise<any>
      clear: (promiseId?: string) => void
    }
    streaming?: {
      enabled?: boolean
      complete?: boolean
      bufferedRows: string[]
      bufferedEvents: any[]
    }
    hmr?: {
      refreshCounters: Record<string, number>
    }
    processBoundaryUpdate?: (boundaryId: string, rscRow: string, rowId: string) => void
    boundaryModules?: Map<string, ModuleData>
    pendingBoundaryHydrations?: Map<string, any>
    serverComponents?: Set<string>
    routeInfoCache?: Map<string, any>
    bridge?: any
  }
  '~clientComponents': Record<string, ComponentInfo>
  '~clientComponentPaths': Record<string, string>
  '~clientComponentNames': Record<string, string>
}

export interface WindowWithRari extends Window {
  '~rari': GlobalWithRari['~rari']
  '~clientComponents': GlobalWithRari['~clientComponents']
  '~clientComponentPaths': GlobalWithRari['~clientComponentPaths']
  '~clientComponentNames': GlobalWithRari['~clientComponentNames']
}
