import { getRariGlobal } from '@/runtime/shared/rari-global'

declare global {
  // eslint-disable-next-line vars-on-top
  var __RARI_USE_CACHE_BUILD_ID__: string | undefined
}

let testBuildId: string | undefined

export function setUseCacheBuildId(buildId: string): void {
  testBuildId = buildId
}

export function resetUseCacheBuildIdForTests(): void {
  testBuildId = undefined
}

export function getUseCacheBuildId(): string {
  if (testBuildId)
    return testBuildId

  const buildId = getRariGlobal().useCacheBuildId
  if (buildId)
    return buildId

  if (globalThis.__RARI_USE_CACHE_BUILD_ID__)
    return globalThis.__RARI_USE_CACHE_BUILD_ID__

  return 'development'
}
