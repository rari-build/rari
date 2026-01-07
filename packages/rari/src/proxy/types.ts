export interface RariRequest {
  url: string
  method: string
  headers: Headers
  cookies: RequestCookies
  rariUrl: RariURL
  ip?: string
  geo?: {
    city?: string
    country?: string
    region?: string
    latitude?: string
    longitude?: string
  }
}

export interface RariURL {
  href: string
  origin: string
  protocol: string
  hostname: string
  port: string
  pathname: string
  search: string
  searchParams: URLSearchParams
  hash: string
}

export interface RequestCookies {
  get: (name: string) => { name: string, value: string, path?: string } | undefined
  getAll: () => Array<{ name: string, value: string, path?: string }>
  has: (name: string) => boolean
  delete: (name: string) => void
  set: ((name: string, value: string, options?: CookieOptions) => void) & ((options: { name: string, value: string } & CookieOptions) => void)
}

export interface ResponseCookies {
  get: (name: string) => { name: string, value: string, path?: string } | undefined
  getAll: () => Array<{ name: string, value: string, path?: string }>
  set: ((name: string, value: string, options?: CookieOptions) => void) & ((options: { name: string, value: string } & CookieOptions) => void)
  delete: (name: string) => void
}

export interface CookieOptions {
  path?: string
  domain?: string
  maxAge?: number
  expires?: Date
  httpOnly?: boolean
  secure?: boolean
  sameSite?: 'strict' | 'lax' | 'none'
}

export interface RariFetchEvent {
  waitUntil: (promise: Promise<unknown>) => void
}

export type ProxyFunction = (
  request: any,
  event?: RariFetchEvent,
) => Promise<any> | any

export interface ProxyMatcher {
  source: string
  locale?: boolean
  has?: Array<{
    type: 'header' | 'query' | 'cookie'
    key: string
    value?: string
  }>
  missing?: Array<{
    type: 'header' | 'query' | 'cookie'
    key: string
    value?: string
  }>
}

export interface ProxyConfig {
  matcher?: string | string[] | ProxyMatcher | ProxyMatcher[]
}

export interface ProxyModule {
  proxy?: ProxyFunction
  default?: ProxyFunction
  config?: ProxyConfig
}

export interface ProxyResult {
  continue: boolean
  response?: Response
  requestHeaders?: Record<string, string>
  responseHeaders?: Record<string, string>
  rewrite?: string
  redirect?: {
    destination: string
    permanent: boolean
  }
}

export interface ProxyRule {
  source: string
  type: 'redirect' | 'rewrite' | 'header' | 'block'
  destination?: string
  permanent?: boolean
  headers?: Record<string, string>
  conditions?: {
    has?: Array<{
      type: 'header' | 'cookie' | 'query' | 'host'
      key: string
      value?: string
    }>
    missing?: Array<{
      type: 'header' | 'cookie' | 'query'
      key: string
      value?: string
    }>
  }
}

export interface ProxyManifest {
  proxyFile: string
  enabled: boolean
  generated: string
  rules?: ProxyRule[]
  matcher?: ProxyConfig['matcher']
  requiresRuntime?: boolean
}
