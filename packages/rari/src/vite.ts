import { rari as _rari } from './vite/index'
import './fetch-cache'

export type {
  ApiRouteHandlers,
  RouteContext,
  RouteHandler,
} from './api-routes'

export { ApiResponse } from './api-routes'

export type Request = globalThis.Request
export type Response = globalThis.Response

export { RariRequest } from './proxy/RariRequest'
export { RariResponse } from './proxy/RariResponse'

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

export type { Robots, RobotsRule, Sitemap, SitemapEntry, SitemapImage, SitemapVideo } from './router/metadata-route'

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
  TemplateEntry,
} from './router/types'

export type { Metadata } from './router/types'

export { rariRouter } from './router/vite-plugin'

export { defineRariConfig, defineRariOptions } from './vite/index'

export type { RariOptions, RouterPluginOptions } from './vite/index'

export type {
  ServerCacheConfig,
  ServerCacheControlConfig,
  ServerCacheLayerConfig,
  ServerConfig,
  ServerCSPConfig,
  ServerUseCacheConfig,
} from './vite/server-config'

export function rari(options?: Parameters<typeof _rari>[0]): any[] {
  return _rari(options)
}
