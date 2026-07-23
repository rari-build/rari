import { getRariGlobal } from '@/runtime/shared/rari-global'

declare global {
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
  if (testBuildId != null && testBuildId !== '') return testBuildId

  const buildId = getRariGlobal().useCacheBuildId
  if (buildId != null && buildId !== '') return buildId

  if (
    globalThis.__RARI_USE_CACHE_BUILD_ID__ != null &&
    globalThis.__RARI_USE_CACHE_BUILD_ID__ !== ''
  )
    return globalThis.__RARI_USE_CACHE_BUILD_ID__

  return 'development'
}
