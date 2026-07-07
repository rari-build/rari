import type { CacheStorage } from '../storage/types'

const tagToKeys = new Map<string, Set<string>>()
const keyToTags = new Map<string, Set<string>>()

function addKeyTags(key: string, tags: readonly string[]): void {
  if (tags.length === 0)
    return

  const normalized = [...new Set(tags)]
  keyToTags.set(key, new Set(normalized))

  for (const tag of normalized) {
    let keys = tagToKeys.get(tag)
    if (!keys) {
      keys = new Set()
      tagToKeys.set(tag, keys)
    }
    keys.add(key)
  }
}

function removeKeyTags(key: string): void {
  const tags = keyToTags.get(key)
  if (!tags)
    return

  for (const tag of tags) {
    const keys = tagToKeys.get(tag)
    keys?.delete(key)
    if (keys && keys.size === 0)
      tagToKeys.delete(tag)
  }

  keyToTags.delete(key)
}

export function registerUseCacheEntryTags(key: string, tags: readonly string[]): void {
  removeKeyTags(key)
  addKeyTags(key, tags)
}

export async function invalidateUseCacheTag(
  tag: string,
  deleteKey: (key: string) => Promise<void>,
): Promise<number> {
  const keys = tagToKeys.get(tag)
  if (!keys || keys.size === 0)
    return 0

  const snapshot = [...keys]
  let invalidated = 0
  for (const key of snapshot) {
    try {
      await deleteKey(key)
      removeKeyTags(key)
      invalidated += 1
    }
    catch (error) {
      console.error(`[rari] failed to invalidate use cache key "${key}" for tag "${tag}":`, error)
    }
  }

  if (!tagToKeys.get(tag)?.size)
    tagToKeys.delete(tag)

  return invalidated
}

export async function invalidateUseCacheKey(
  key: string,
  deleteKey: (key: string) => Promise<void>,
): Promise<void> {
  await deleteKey(key)
  removeKeyTags(key)
}

export function getActiveUseCacheTags(): string[] {
  return [...tagToKeys.keys()]
}

export function resetUseCacheTagRegistryForTests(): void {
  tagToKeys.clear()
  keyToTags.clear()
}

export function getUseCacheTagRegistryStorage(): {
  tagToKeys: Map<string, Set<string>>
  keyToTags: Map<string, Set<string>>
} {
  return { tagToKeys, keyToTags }
}

export type UseCacheDeleteKey = (key: string) => Promise<void>

export function createRegistryBackedDelete(
  storages: readonly CacheStorage[],
): UseCacheDeleteKey {
  return async (key: string) => {
    await Promise.all(storages.map(async (storage) => {
      if ('delete' in storage && typeof storage.delete === 'function')
        await storage.delete(key)
    }))
  }
}
