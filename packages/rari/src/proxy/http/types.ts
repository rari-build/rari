import type { RariResponse } from './response'

export interface RariRequest {
  readonly url: string
  readonly method: string
  readonly headers: Headers
  readonly cookies: RequestCookies
  readonly rariUrl: RariURL
  readonly ip?: string
  readonly geo?: Readonly<{
    city?: string
    country?: string
    region?: string
    latitude?: string
    longitude?: string
  }>
}

export interface RariURL {
  readonly href: string
  readonly origin: string
  readonly protocol: string
  readonly hostname: string
  readonly port: string
  readonly pathname: string
  readonly search: string
  readonly searchParams: URLSearchParams
  readonly hash: string
}

export interface RequestCookies {
  readonly get: (
    name: string,
  ) => Readonly<{ name: string; value: string; path?: string }> | undefined
  readonly getAll: () => ReadonlyArray<Readonly<{ name: string; value: string; path?: string }>>
  readonly has: (name: string) => boolean
  readonly delete: (name: string) => void
  readonly set: ((name: string, value: string, options?: Readonly<CookieOptions>) => void) &
    ((options: Readonly<{ name: string; value: string } & CookieOptions>) => void)
}

export interface ResponseCookies {
  readonly get: (
    name: string,
  ) => Readonly<{ name: string; value: string; path?: string }> | undefined
  readonly getAll: () => ReadonlyArray<Readonly<{ name: string; value: string; path?: string }>>
  readonly set: ((name: string, value: string, options?: Readonly<CookieOptions>) => void) &
    ((options: Readonly<{ name: string; value: string } & CookieOptions>) => void)
  readonly delete: (name: string) => void
  readonly toSetCookieHeaders: () => string[]
}

export interface CookieOptions {
  readonly path?: string
  readonly domain?: string
  readonly maxAge?: number
  readonly expires?: Date
  readonly httpOnly?: boolean
  readonly secure?: boolean
  readonly sameSite?: 'strict' | 'lax' | 'none'
}

export interface RariFetchEvent {
  readonly waitUntil: (promise: Promise<unknown>) => void
}

export type ProxyFunctionResult = Response | RariResponse | null | undefined

export type ProxyFunction = (
  request: RariRequest,
  event?: RariFetchEvent,
) => Promise<ProxyFunctionResult> | ProxyFunctionResult

export interface ProxyCondition {
  readonly type: 'header' | 'query' | 'cookie'
  readonly key: string
  readonly value?: string
}

export type ProxyRuleCondition =
  | ProxyCondition
  | { readonly type: 'host'; readonly key: string; readonly value?: string }

export interface ProxyMatcher {
  readonly source: string
  readonly locale?: boolean
  readonly has?: ReadonlyArray<ProxyRuleCondition>
  readonly missing?: ReadonlyArray<ProxyRuleCondition>
}

export interface ProxyConfig {
  readonly matcher?: string | readonly string[] | ProxyMatcher | readonly ProxyMatcher[]
}

export interface ProxyModule {
  readonly proxy?: ProxyFunction
  readonly default?: ProxyFunction
  readonly config?: ProxyConfig
}

export interface ProxyResult {
  readonly continue: boolean
  readonly response?: Response
  readonly requestHeaders?: Readonly<Record<string, string | readonly string[]>>
  readonly responseHeaders?: Readonly<Record<string, string | readonly string[]>>
  readonly rewrite?: string
  readonly redirect?: {
    readonly destination: string
    readonly permanent: boolean
  }
}

export interface ProxyRule {
  readonly source: string
  readonly type: 'redirect' | 'rewrite' | 'header' | 'block'
  readonly destination?: string
  readonly permanent?: boolean
  readonly headers?: Readonly<Record<string, string>>
  readonly conditions?: {
    readonly has?: ReadonlyArray<ProxyRuleCondition>
    readonly missing?: ReadonlyArray<ProxyRuleCondition>
  }
}

export interface ProxyManifest {
  readonly proxyFile: string
  readonly enabled: boolean
  readonly generated: string
  readonly rules?: readonly ProxyRule[]
  readonly matcher?: ProxyConfig['matcher']
  readonly requiresRuntime?: boolean
}
