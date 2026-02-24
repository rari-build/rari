export interface RouteInfoRequest {
  path: string
}

export interface RouteInfoResponse {
  exists: boolean
  layouts: string[]
  loading: string | null
  isDynamic: boolean
  params?: string[]
  segments?: Array<{
    type: 'static' | 'dynamic' | 'catch-all' | 'optional-catch-all'
    value: string
    param?: string
  }>
}

export interface RouteInfoError {
  error: string
  code: 'NOT_FOUND' | 'INVALID_PATH' | 'SERVER_ERROR'
}
