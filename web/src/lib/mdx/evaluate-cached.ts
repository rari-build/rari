import type { ComponentType } from 'react'
import { createHash } from 'node:crypto'

const MAX_ENTRIES = 64

const cache = new Map<string, ComponentType>()
const inflight = new Map<string, Promise<ComponentType>>()

function contentKey(filePath: string, content: string): string {
  return `${filePath}:${createHash('sha256').update(content).digest('hex')}`
}

// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
function remember(key: string, component: ComponentType): ComponentType {
  if (cache.has(key)) cache.delete(key)
  cache.set(key, component)
  while (cache.size > MAX_ENTRIES) {
    const oldest = cache.keys().next().value
    if (oldest == null) break
    cache.delete(oldest)
  }
  return component
}

export async function withMdxEvaluateCache(
  filePath: string,
  content: string,
  load: () => Promise<ComponentType>,
): Promise<ComponentType> {
  const key = contentKey(filePath, content)
  const hit = cache.get(key)
  if (hit != null) {
    cache.delete(key)
    cache.set(key, hit)
    return hit
  }

  const existing = inflight.get(key)
  if (existing != null) return existing

  const pending = load()
    .then(component => remember(key, component))
    .finally(() => {
      inflight.delete(key)
    })
  inflight.set(key, pending)
  return pending
}

export function resetMdxEvaluateCacheForTests(): void {
  cache.clear()
  inflight.clear()
}
