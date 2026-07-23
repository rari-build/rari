'use cache'

declare global {
  var __rariUseCacheFileCounter: { total: number } | undefined
}

function bumpCallCount(): number {
  globalThis.__rariUseCacheFileCounter ??= { total: 0 }
  globalThis.__rariUseCacheFileCounter.total += 1
  return globalThis.__rariUseCacheFileCounter.total
}

// oxlint-disable-next-line typescript/require-await -- async is required for the file-level 'use cache' transform
export async function getCachedData(label: string) {
  const count = bumpCallCount()
  return `${label}:${count}`
}
