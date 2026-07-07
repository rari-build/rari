import type { CacheLifeProfile } from './cache-life'

import { AsyncLocalStorage } from 'node:async_hooks'
import { getRariGlobal } from '@/runtime/shared/rari-global'
import { normalizeCacheLife } from './cache-life'

export interface CacheScopeContext {
  life?: CacheLifeProfile
  tags: string[]
}

const storage = new AsyncLocalStorage<CacheScopeContext>()

function currentContext(): CacheScopeContext | undefined {
  return storage.getStore()
}

export function runWithCacheContext<T>(fn: () => T | Promise<T>): Promise<T> {
  return Promise.resolve(storage.run({ tags: [] }, fn))
}

export function getCacheContext(): CacheScopeContext {
  const ctx = currentContext()
  if (!ctx)
    throw new Error('[rari] cache context is only available inside a cached function call.')

  return ctx
}

export function setCacheLife(
  profile: Parameters<typeof normalizeCacheLife>[0],
): void {
  const ctx = currentContext()
  if (!ctx) {
    console.warn('[rari] cacheLife() has no effect outside a cached function call.')
    return
  }
  ctx.life = normalizeCacheLife(profile)
}

export function registerPageCacheTags(...tags: string[]): void {
  if (tags.length === 0)
    return

  const pageCacheTags = getRariGlobal().pageCacheTags ??= new Set()
  for (const tag of tags)
    pageCacheTags.add(tag)
}

export function addCacheTags(...tags: string[]): void {
  const ctx = currentContext()
  if (!ctx) {
    console.warn('[rari] cacheTag() has no effect outside a cached function call.')
    return
  }
  const next = new Set(ctx.tags)

  for (const tag of tags) {
    if (next.size >= 128) {
      console.warn('[rari] cacheTag: maximum of 128 tags per cache scope reached; skipping remaining tags.')
      break
    }
    if (tag.length > 256) {
      console.warn(`[rari] cacheTag: tag longer than 256 characters skipped: "${tag.slice(0, 32)}..."`)
      continue
    }
    next.add(tag)
  }

  ctx.tags = [...next]
  registerPageCacheTags(...ctx.tags)
}
