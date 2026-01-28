export type {
  ApiRouteHandlers,
  RouteContext,
  RouteHandler,
} from './api-routes'

export { ApiResponse } from './api-routes'

export { RariRequest } from './proxy/RariRequest'
export { RariResponse } from './proxy/RariResponse'

export type Request = globalThis.Request
export type Response = globalThis.Response

export type {
  CookieOptions,
  ProxyConfig,
  ProxyFunction,
  ProxyMatcher,
  ProxyModule,
  ProxyResult,
  RariFetchEvent,
  RariURL,
  RequestCookies,
  ResponseCookies,
} from './proxy/types'

export { rariProxy } from './proxy/vite-plugin'

export type { ProxyPluginOptions } from './proxy/vite-plugin'

export {
  clearPropsCache,
  clearPropsCacheForComponent,
  extractMetadata,
  extractServerProps,
  extractServerPropsWithCache,
  extractStaticParams,
  hasServerSideDataFetching,
} from './router/props-extractor'

export type {
  MetadataResult,
  ServerSidePropsResult,
  StaticParamsResult,
} from './router/props-extractor'

export {
  generateAppRouteManifest,
} from './router/routes'

export type {
  AppRouteEntry,
  AppRouteManifest,
  AppRouteMatch,
  ErrorEntry,
  ErrorProps,
  GenerateMetadata,
  GenerateStaticParams,
  LayoutEntry,
  LayoutProps,
  LoadingEntry,
  NotFoundEntry,
  PageProps,
  RouteSegment,
  RouteSegmentType,
} from './router/types'

export { rariRouter } from './router/vite-plugin'

export { createHttpRuntimeClient, HttpRuntimeClient } from './runtime-client'

export type { RuntimeClient } from './runtime-client'

export type { Robots, RobotsRule, Sitemap, SitemapEntry, SitemapImage, SitemapVideo } from './types/metadata-route'

export type {
  ServerConfig,
  ServerCSPConfig,
  ServerRateLimitConfig,
  ServerSpamBlockerConfig,
} from './types/server-config'

export { defineRariConfig, defineRariOptions, rari } from './vite/index'
