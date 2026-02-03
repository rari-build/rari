export interface ModuleData {
  id: string
  chunks: string[]
  name: string
}

export interface ComponentInfo {
  id: string
  path: string
  type: string
  component: any
  registered: boolean
}

export interface GlobalWithRari {
  '~rari': {
    isDevelopment?: boolean
    AppRouterProvider?: any
    ClientRouter?: any
    getClientComponent?: (id: string) => any
    hydrateClientComponents?: (boundaryId: string, content: any, boundaryElement: Element) => void
    processBoundaryUpdate?: (boundaryId: string, rscRow: string, rowId: string) => void
    boundaryModules?: Map<string, ModuleData>
    bufferedRows?: string[]
    streamComplete?: boolean
    pendingBoundaryHydrations?: Map<string, any>
    bufferedEvents?: any[]
    serverComponents?: Set<string>
    routeInfoCache?: Map<string, any>
    bridge?: any
  }
  '~clientComponents': Record<string, ComponentInfo>
  '~clientComponentPaths': Record<string, string>
  '~clientComponentNames': Record<string, string>
  '~rscRefreshCounters'?: Record<string, number>
}

export interface WindowWithRari extends Window {
  '~rari': GlobalWithRari['~rari']
  '~clientComponents': GlobalWithRari['~clientComponents']
  '~clientComponentPaths': GlobalWithRari['~clientComponentPaths']
  '~clientComponentNames': GlobalWithRari['~clientComponentNames']
  '~rscRefreshCounters'?: Record<string, number>
}
