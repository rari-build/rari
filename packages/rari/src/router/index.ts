export {
  AppRouteGenerator,
  generateAppRouteManifest,
  loadManifest,
  writeManifest,
} from './app-routes'

export type * from './app-types'

export {
  determineAffectedRoutes,
  extractRoutePathFromFile,
  getAppRouterFileInfo,
  getAppRouterFileType,
  isAppRouterFile,
} from './hmr-utils'

export type {
  AppRouterFileInfo,
  AppRouterFileType,
} from './hmr-utils'

export {
  clearPropsCache,
  clearPropsCacheForComponent,
  extractMetadata,
  extractServerProps,
  extractServerPropsWithCache,
  extractStaticParams,
  hasServerSideDataFetching,
} from './props-extractor'

export type {
  MetadataResult,
  ServerPropsResult,
  StaticParamsResult,
} from './props-extractor'

export { rariRouter } from './vite-plugin'
