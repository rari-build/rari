import type { CacheLifeProfile, CacheLifeProfileName } from '@/runtime/context/cache-life'
import { addCacheTags, setCacheLife } from '@/runtime/context/cache-context'
import { invalidateUseCacheByTag, invalidateUseCacheEntries } from '@/runtime/invalidation/cache-invalidation'

export type { CacheLifeProfile, CacheLifeProfileName } from '@/runtime/context/cache-life'

export function cacheLife(profile: CacheLifeProfileName | CacheLifeProfile): void {
  setCacheLife(profile)
}

export function cacheTag(...tags: string[]): void {
  addCacheTags(...tags)
}

export async function revalidateTag(tag: string): Promise<void> {
  await invalidateUseCacheByTag(tag)
  await invalidateUseCacheEntries({ tag })
}

export async function revalidatePath(path: string): Promise<void> {
  await revalidateTag(path)
  await invalidateUseCacheEntries({ path })
}

export async function updateTag(tag: string): Promise<void> {
  await revalidateTag(tag)
}
