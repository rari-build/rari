export interface LayoutBoundary {
  layoutPath: string
  startLine: number
  endLine: number
  props: any
}

export interface RSCRouteMetadata {
  route: string
  layoutChain: string[]
  timestamp: number
}

export interface EnhancedRSCPayload {
  wireFormat: string
  layoutBoundaries: LayoutBoundary[]
  routeMetadata: RSCRouteMetadata
  element?: any
  modules?: Map<string, any>
}

export interface ParsedRSCPayload {
  element: any
  modules: Map<string, any>
  symbols?: Map<string, string>
  wireFormat: string
  layoutBoundaries?: LayoutBoundary[]
  routeMetadata?: RSCRouteMetadata
}
