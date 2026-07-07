'use cache'

declare global {
  // eslint-disable-next-line vars-on-top
  var __rariUseCacheFileCounter: { total: number } | undefined
}

function bumpCallCount(): number {
  globalThis.__rariUseCacheFileCounter ??= { total: 0 }
  globalThis.__rariUseCacheFileCounter.total += 1
  return globalThis.__rariUseCacheFileCounter.total
}

export async function getCachedData(label: string) {
  const count = bumpCallCount()
  return `${label}:${count}`
}
